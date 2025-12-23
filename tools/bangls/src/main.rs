use anyhow::{Result, anyhow, bail};
use crossbeam_channel::{Receiver, Sender};
use linked_hash_map::LinkedHashMap;
use lsp_server::{IoThreads, Message};
use lsp_types::{CompletionItem, CompletionOptions, InitializeParams, MessageType, Position, ServerCapabilities, ShowMessageParams, TextDocumentSyncCapability, TextDocumentSyncKind, Uri, notification::{self, Notification}, request::{self, Request}};
use syntax::{Compile, CompileMeta, EmulateInfo, Expand, LSP_DEBUG};

fn main() {
    main_loop().unwrap();
}

fn lopos(pos: Position) -> (u32, u32) {
    (pos.line + 1, pos.character + 1)
}

struct IoJoiner(pub Option<IoThreads>);
impl std::ops::DerefMut for IoJoiner {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.as_mut().unwrap()
    }
}
impl std::ops::Deref for IoJoiner {
    type Target = IoThreads;

    fn deref(&self) -> &Self::Target {
        self.0.as_ref().unwrap()
    }
}
impl Drop for IoJoiner {
    fn drop(&mut self) {
        if let Some(io) = self.0.take() {
            io.join().unwrap()
        }
    }
}
impl IoJoiner {
    fn _consume(mut self) -> IoThreads {
        self.0.take().unwrap()
    }
}

fn main_loop() -> Result<()> {
    let (connect, io) = lsp_server::Connection::stdio();
    let _io = IoJoiner(Some(io));
    let server_capabilities = ServerCapabilities {
        completion_provider: Some(CompletionOptions {
            trigger_characters: Some(vec![]),
            ..Default::default()
        }),
        text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
        ..Default::default()
    };
    let server_capabilities = serde_json::to_value(server_capabilities)?;
    let init_params = connect.initialize(server_capabilities)?;
    let InitializeParams {
        workspace_folders,
        ..
    } = serde_json::from_value::<InitializeParams>(init_params)?;
    let _workspace_folders = workspace_folders.ok_or(anyhow!("Cannot find workspace folder"))?;
    let mut ctx = Ctx {
        open_files: Default::default(),
        sender: connect.sender,
        recver: connect.receiver,
    };
    ctx.run()
}

struct Ctx {
    open_files: LinkedHashMap<Uri, String>,
    sender: Sender<Message>,
    recver: Receiver<Message>,
}
impl Ctx {
    fn run(&mut self) -> Result<(), anyhow::Error> {
        while let Ok(msg) = self.recver.recv() {
            match msg {
                Message::Request(request) => self.handle_requests(request)?,
                Message::Response(response) => {
                    eprintln!("unknown response {response:?}")
                },
                Message::Notification(notification) => {
                    let notification = &mut Some(notification);
                    self.try_handle_notif::<notification::DidOpenTextDocument>(notification)?;
                    self.try_handle_notif::<notification::DidChangeTextDocument>(notification)?;
                    self.try_handle_notif::<notification::DidCloseTextDocument>(notification)?;
                    eprintln!("unknown notification {notification:?}")
                },
            }
        }
        Ok(())
    }

    fn handle_requests(&mut self, request: lsp_server::Request) -> Result<()> {
        let request = &mut Some(request);

        self.try_handle_req::<request::Completion>(request).transpose()?;

        if let Some(request) = request {
            bail!("unknown request {request:#?}")
        }
        Ok(())
    }

    fn try_handle_notif<T: NotificationHandler>(&mut self, notification: &mut Option<lsp_server::Notification>) -> Result<()> {
        let Some(lsp_server::Notification { params, .. })
            = notification.take_if(|it| it.method == T::METHOD) else { return Ok(()) };
        let params = match serde_json::from_value(params) {
            Ok(it) => it,
            Err(err) => bail!(err),
        };
        T::handle(self, params)
    }

    fn try_handle_req<T: RequestHandler>(&mut self, request: &mut Option<lsp_server::Request>) -> Option<Result<()>> {
        let lsp_server::Request { id, params, .. }
            = request.take_if(|req| req.method == T::METHOD)?;
        let params = match serde_json::from_value(params) {
            Ok(it) => it,
            Err(err) => return Some(Err(err.into())),
        };
        let result = T::handle(self, params);
        let response = match result {
            Ok(value) => {
                match serde_json::to_value(value) {
                    Ok(to_value) => lsp_server::Response { id, result: Some(to_value), error: None },
                    Err(e) => return Some(Err(anyhow!(e))),
                }
            },
            Err(err) => lsp_server::Response {
                id,
                result: None,
                error: Some(lsp_server::ResponseError {
                    code: 1,
                    message: err.to_string(),
                    data: None,
                }),
            },
        };
        Some(self.sender.send(Message::Response(response)).map_err(Into::into))
    }

    fn read_file(&self, uri: &Uri) -> Result<&str> {
        match self.open_files.get(uri) {
            Some(s) => Ok(s),
            None => bail!("Cannot read no opened file"),
        }
    }

    fn try_parse(&self, (line, col): (u32, u32), file: &str) -> Result<(Expand, String)> {
        let index = line_column::index(&file, line, col);
        let placeholders = [
            format!("{LSP_DEBUG} "),
            format!("{LSP_DEBUG} __lsp_arg;"),
        ];
        let parser = parser::TopLevelParser::new();
        let mut parse_err = "".to_owned();
        for placeholder in &placeholders {
            let source = String::from_iter([&file[..index], placeholder, &file[index..]]);
            match parser.parse(&mut syntax::Meta::new(), &source) {
                Err(e) => if parse_err.is_empty() {
                    parse_err = parser::format_parse_err::<5>(e, &source);
                },
                Ok(top) => return Ok((top, source)),
            }
        }
        Err(anyhow!("Fake parse err: {parse_err}"))
    }

    fn send_window_notif(&self, typ: MessageType, msg: impl std::fmt::Display) -> Result<()> {
        let params = ShowMessageParams {
            typ,
            message: msg.to_string(),
        };
        let params = serde_json::to_value(params)?;
        let msg = Message::Notification(lsp_server::Notification {
                method: notification::ShowMessage::METHOD.to_owned(),
                params,
            });
        self.sender.send(msg)?;
        Ok(())
    }
}

trait RequestHandler: Request {
    fn handle(ctx: &Ctx, param: Self::Params) -> Result<Self::Result>;
}
impl RequestHandler for request::Completion {
    fn handle(ctx: &Ctx, param: Self::Params) -> Result<Self::Result> {
        let uri = param.text_document_position.text_document.uri;
        let (line, col) = lopos(param.text_document_position.position);

        let file = ctx.read_file(&uri)?;
        let (top, src) = ctx.try_parse((line, col), &file)?;
        let mut meta = CompileMeta::with_source(src.into());
        meta.is_emulated = true;

        let assert_meta = std::panic::AssertUnwindSafe(&mut meta);
        let _ = std::panic::catch_unwind(|| {
            top.compile(&mut {assert_meta}.0);
        });
        let infos = meta.emulate_infos;

        let completes = generate_completes(&infos);
        let completes = lsp_types::CompletionResponse::Array(completes);
        Ok(Some(completes))
    }
}

fn generate_completes(infos: &[EmulateInfo]) -> Vec<CompletionItem> {
    let mut var_counter = LinkedHashMap::new();
    for info in infos {
        for var in info.exist_vars.iter() {
            if var.starts_with("__") {
                continue;
            }
            let slot: &mut u32 = var_counter.entry(var).or_default();
            *slot += 1;
        }
    }
    let full_count = infos.len() as u32;
    var_counter.iter().map(|(&var, &count)| {
        let is_full_deps = count == full_count;
        let (label, detail) = if is_full_deps {
            (var.to_string(), format!("full deps ({count}/{full_count})"))
        } else {
            (format!("{var}?"), format!("partial deps ({count}/{full_count})"))
        };
        CompletionItem {
            label,
            detail: Some(detail),
            insert_text: Some(var.to_string()),
            ..Default::default()
        }
    }).collect()
}

trait NotificationHandler: Notification {
    fn handle(ctx: &mut Ctx, param: Self::Params) -> Result<()>;
}
impl NotificationHandler for notification::DidOpenTextDocument {
    fn handle(ctx: &mut Ctx, param: Self::Params) -> Result<()> {
        ctx.send_window_notif(MessageType::LOG, "open file")?;
        let file = ctx.open_files.entry(param.text_document.uri).or_default();
        *file = param.text_document.text;
        Ok(())
    }
}
impl NotificationHandler for notification::DidChangeTextDocument {
    fn handle(ctx: &mut Ctx, param: Self::Params) -> Result<()> {
        let file = ctx.open_files.entry(param.text_document.uri).or_default();

        for change in param.content_changes {
            if change.range.is_some() {
                bail!("unsupported range change sync: {change:#?}")
            }
            *file = change.text;
        }
        Ok(())
    }
}
impl NotificationHandler for notification::DidCloseTextDocument {
    fn handle(ctx: &mut Ctx, param: Self::Params) -> Result<()> {
        let uri = param.text_document.uri;
        if ctx.open_files.remove(&uri).is_none() {
            ctx.send_window_notif(MessageType::WARNING, format_args!("Cannot close unknown file: {uri:?}"))?;
        }
        Ok(())
    }
}
