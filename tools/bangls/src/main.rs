use display_source::DisplaySource;
use itertools::Itertools;
use std::{borrow::Cow, cell::RefCell, collections::HashSet, rc::Rc};

use anyhow::{Result, anyhow, bail};
use crossbeam_channel::{Receiver, Sender};
use display_source::DisplaySourceMeta;
use line_column::line_column;
use linked_hash_map::LinkedHashMap;
use lsp_server::{IoThreads, Message};
use lsp_types::{CompletionItem, CompletionItemKind, CompletionOptions, Diagnostic, DiagnosticSeverity, InitializeParams, InitializeResult, InsertTextFormat, MessageType, Position, ServerCapabilities, ShowMessageParams, TextDocumentSyncCapability, TextDocumentSyncKind, Uri, notification::{self, Notification}, request::{self, Request}};
use syntax::{Compile, CompileMeta, CompileMetaExtends, Emulate, EmulateInfo, Expand, LSP_DEBUG, LSP_HOVER};

fn main() {
    main_loop().unwrap();
}

fn lopos(pos: Position) -> (u32, u32) {
    (pos.line + 1, pos.character + 1)
}

fn rgpos((line, column): (u32, u32)) -> Position {
    debug_assert_ne!(line, 0);
    debug_assert_ne!(column, 0);
    Position { line: line - 1, character: column - 1 }
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
            trigger_characters: Some(vec![".".to_owned(), ">".to_owned()]),
            ..Default::default()
        }),
        diagnostic_provider: Some(lsp_types::DiagnosticServerCapabilities::Options(
            lsp_types::DiagnosticOptions::default(),
        )),
        text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
        hover_provider: Some(lsp_types::HoverProviderCapability::Simple(true)),
        ..Default::default()
    };
    let init_params = {
        let (id, params) = connect.initialize_start()?;

        let initialize_data = InitializeResult {
            capabilities: server_capabilities,
            server_info: Some(lsp_types::ServerInfo {
                name: env!("CARGO_PKG_NAME").into(),
                version: Some(env!("CARGO_PKG_VERSION").into()),
            }),
        };

        let initialize_data = serde_json::to_value(initialize_data)?;
        connect.initialize_finish(id, initialize_data)?;

        params
    };
    let InitializeParams {
        workspace_folders,
        ..
    } = serde_json::from_value::<InitializeParams>(init_params)?;
    let _workspace_folders = workspace_folders.ok_or(anyhow!("Cannot find workspace folder"))?;
    let mut ctx = Ctx::new(connect.sender, connect.receiver);
    ctx.run()
}

struct Ctx {
    open_files: LinkedHashMap<Uri, String>,
    sender: Sender<Message>,
    recver: Receiver<Message>,
}
impl Ctx {
    fn new(sender: Sender<Message>, recver: Receiver<Message>) -> Self {
        Self {
            open_files: Default::default(),
            sender,
            recver,
        }
    }

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

        self.try_handle_req::<request::Completion>(request)?;
        self.try_handle_req::<request::HoverRequest>(request)?;
        self.try_handle_req::<request::DocumentDiagnosticRequest>(request)?;

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

    fn try_handle_req<T: RequestHandler>(&mut self, request: &mut Option<lsp_server::Request>) -> Result<()> {
        let Some(lsp_server::Request { id, params, .. })
            = request.take_if(|req| req.method == T::METHOD) else { return Ok(()) };
        let params = match serde_json::from_value(params) {
            Ok(it) => it,
            Err(err) => return Err(err.into()),
        };
        let result = T::handle(self, params);
        let response = match result {
            Ok(value) => {
                match serde_json::to_value(value) {
                    Ok(to_value) => lsp_server::Response { id, result: Some(to_value), error: None },
                    Err(e) => return Err(anyhow!(e)),
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
        self.sender.send(Message::Response(response)).map_err(Into::into)
    }

    fn read_file(&self, uri: &Uri) -> Result<&str> {
        match self.open_files.get(uri) {
            Some(s) => Ok(s),
            None => bail!("Cannot read no opened file"),
        }
    }

    fn try_parse_for_complete(&self, (line, col): (u32, u32), file: &str) -> Option<(Expand, String)> {
        let index = line_column::index(&file, line, col);
        let placeholders = [
            format!("{LSP_DEBUG} "),
            format!("{LSP_DEBUG} __lsp_arg;"),
        ];
        let parser = parser::TopLevelParser::new();
        for placeholder in &placeholders {
            let source = String::from_iter([&file[..index], placeholder, &file[index..]]);
            match parser.parse(&mut syntax::Meta::new(), &source) {
                Err(_) => (),
                Ok(top) => return Some((top, source)),
            }
        }
        None
    }

    fn try_parse_for_hover(&self, (line, col): (u32, u32), file: &str) -> Option<(Expand, String)> {
        let index = line_column::index(&file, line, col);
        let parser = parser::TopLevelParser::new();
        let source = String::from_iter([&file[..index], LSP_HOVER, &file[index..]]);
        match parser.parse(&mut syntax::Meta::new(), &source) {
            Err(_) => None,
            Ok(top) => Some((top, source)),
        }
}

    fn parse_for_parse_error(&self, file: &str) -> Result<Expand, ((usize, usize), String)> {
        let parser = parser::TopLevelParser::new();
        match parser.parse(&mut syntax::Meta::new(), file) {
            Ok(top) => Ok(top),
            Err(e) => {
                let loc = match e {
                    parser::lalrpop_util::ParseError::InvalidToken { location } |
                    parser::lalrpop_util::ParseError::UnrecognizedEof { location, .. } => {
                        (location, location)
                    },
                    parser::lalrpop_util::ParseError::UnrecognizedToken { token: (start, _, end), .. } |
                    parser::lalrpop_util::ParseError::ExtraToken { token: (start, _, end) } |
                    parser::lalrpop_util::ParseError::User { error: syntax::Error { start, end, .. } } => {
                        (start, end)
                    },
                };
                let fmtted_err = parser::format_parse_err::<5>(e, file);
                Err((loc, fmtted_err))
            },
        }
    }

    fn send_window_notif(&self, typ: MessageType, msg: impl std::fmt::Display) -> Result<()> {
        let params = ShowMessageParams {
            typ,
            message: msg.to_string(),
        };
        self.send_notif::<notification::ShowMessage>(params)
    }

    fn send_notif<T: Notification>(&self, params: T::Params) -> Result<()> {
        let params = serde_json::to_value(params)?;
        let msg = Message::Notification(lsp_server::Notification {
            method: T::METHOD.to_owned(),
            params,
        });
        self.sender.send(msg)?;
        Ok(())
    }
}

trait RequestHandler: Request {
    fn handle(ctx: &mut Ctx, param: Self::Params) -> Result<Self::Result>;
}
impl RequestHandler for request::Completion {
    fn handle(ctx: &mut Ctx, param: Self::Params) -> Result<Self::Result> {
        let uri = param.text_document_position.text_document.uri;
        let (line, col) = lopos(param.text_document_position.position);

        let file = ctx.read_file(&uri)?;
        let Some((top, src)) = ctx.try_parse_for_complete((line, col), &file) else {
            return Ok(None);
        };
        let infos = emulate(top, src);

        let completes = generate_completes(&infos);
        let completes = lsp_types::CompletionResponse::Array(completes);
        Ok(Some(completes))
    }
}
impl RequestHandler for request::HoverRequest {
    fn handle(ctx: &mut Ctx, param: Self::Params) -> Result<Self::Result> {
        let uri = param.text_document_position_params.text_document.uri;
        let (line, col) = lopos(param.text_document_position_params.position);

        let file = ctx.read_file(&uri)?;
        let Some((top, src)) = ctx.try_parse_for_hover((line, col), &file) else {
            return Ok(None);
        };
        let infos = emulate(top, src);
        let mut strings = vec![];
        let mut dedup_set = HashSet::new();

        for info in infos {
            let Some(hover_doc) = info.hover_doc else {
                continue;
            };
            if dedup_set.contains(&hover_doc) {
                continue;
            }
            dedup_set.insert(hover_doc.clone());
            strings.push(lsp_types::MarkedString::LanguageString(
                lsp_types::LanguageString {
                    language: "mdtlbl".to_owned(),
                    value: hover_doc,
                },
            ));
        }

        Ok(Some(lsp_types::Hover { contents: lsp_types::HoverContents::Array(strings), range: None }))
    }
}
impl RequestHandler for request::DocumentDiagnosticRequest {
    fn handle(ctx: &mut Ctx, param: Self::Params) -> Result<Self::Result> {
        Ok(lsp_types::DocumentDiagnosticReportResult::Report(
            lsp_types::DocumentDiagnosticReport::Full(
                lsp_types::RelatedFullDocumentDiagnosticReport {
                    related_documents: None,
                    full_document_diagnostic_report: lsp_types::FullDocumentDiagnosticReport {
                        result_id: None,
                        items: tigger_diagnostics(ctx, &param.text_document.uri)
                    },
                },
            ),
        ))
    }
}

fn generate_completes(infos: &[EmulateInfo]) -> Vec<CompletionItem> {
    let on_line_first = infos.iter().any(|it| it.in_other_line_first);
    let infos = infos.iter()
        .filter_map(|it| it.exist_vars.as_ref());
    let full_count = infos.clone().count() as u32;
    let mut var_counter: LinkedHashMap<&syntax::Var, (u32, Vec<_>, bool)> = LinkedHashMap::new();
    for info in infos {
        for (kind, var, use_args) in info {
            if var.starts_with("__") {
                continue;
            }
            let (slot, kinds, use_args_slot)
                = var_counter.entry(var).or_default();
            *slot += 1;
            *use_args_slot |= *use_args;
            kinds.push(kind);
        }
    }
    let mut items: Vec<CompletionItem> = var_counter.iter().map(
        |(&var, &(count, ref kinds, use_args))|
    {
        let is_full_deps = count == full_count;

        let mut first_kind = None;
        let kind = kinds.iter()
            .inspect(|k| _ = first_kind.get_or_insert(**k))
            .map(|kind| match kind {
                Emulate::Const => format!("constant"),
                Emulate::Binder => format!("binder"),
                Emulate::ConstBind(var) => format!("const bind to `{var}`"),
                Emulate::NakedBind(var) => format!("naked bind to `{var}`"),
            }).unique().join(" & ");

        let (label, detail) = if is_full_deps {
            (var.to_string(), format!("kind: {kind}\nfull deps ({count}/{full_count})"))
        } else {
            (format!("{var}?"), format!("kind: {kind}\npartial deps ({count}/{full_count})"))
        };

        let first_kind = first_kind.map(|it| match it {
            Emulate::Const => CompletionItemKind::CONSTANT,
            Emulate::Binder => CompletionItemKind::VALUE,
            Emulate::ConstBind(_) => CompletionItemKind::METHOD,
            Emulate::NakedBind(_) => CompletionItemKind::FIELD,
        });

        let insert_snippet = match (use_args, on_line_first) {
            (true, true) => format!("{var}! $0;"),
            (true, false) => format!("{var}[$0]"),
            _ => var.to_string(),
        };
        let insert_text_format = (*insert_snippet != **var)
            .then_some(InsertTextFormat::SNIPPET);

        CompletionItem {
            label,
            detail: Some(detail),
            kind: first_kind,
            insert_text: Some(insert_snippet),
            insert_text_format,
            ..Default::default()
        }
    }).collect();
    items.sort_by(|a, b| {
        let a = a.sort_text.as_ref().unwrap_or(&a.label);
        let b = b.sort_text.as_ref().unwrap_or(&b.label);
        (a.starts_with('_'), a).cmp(&(b.starts_with('_'), b))
    });
    items
}

trait NotificationHandler: Notification {
    fn handle(ctx: &mut Ctx, param: Self::Params) -> Result<()>;
}
impl NotificationHandler for notification::DidOpenTextDocument {
    fn handle(ctx: &mut Ctx, param: Self::Params) -> Result<()> {
        let file = ctx.open_files.entry(param.text_document.uri).or_default();
        *file = param.text_document.text;
        Ok(())
    }
}
impl NotificationHandler for notification::DidChangeTextDocument {
    fn handle(ctx: &mut Ctx, param: Self::Params) -> Result<()> {
        if !ctx.open_files.contains_key(&param.text_document.uri) {
            ctx.open_files.insert(param.text_document.uri.clone(), String::new());
        }
        let file = ctx.open_files.get_mut(&param.text_document.uri).unwrap();

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

fn tigger_diagnostics(ctx: &mut Ctx, uri: &Uri) -> Vec<Diagnostic> {
    let Some(file) = ctx.open_files.get(uri) else { return vec![] };
    let mut diags = vec![];

    match ctx.parse_for_parse_error(file) {
        Err(((sindex, eindex), error)) => {
            let start = rgpos(line_column(file, sindex));
            let end = rgpos(line_column(file, eindex));
            diags.push(Diagnostic {
                message: error,
                range: lsp_types::Range { start, end },
                severity: Some(DiagnosticSeverity::ERROR),
                ..Default::default()
            });
        }
        Ok(top) => {
            let infos = emulate(top, file.clone());
            for info in infos {
                let Some(diagnostic) = info.diagnostic else { continue };
                let Some(loc) = info.location.or_else(|| info.is_error.then(|| {
                    (1, 1)
                })) else { continue };
                let start = rgpos(loc);
                diags.push(Diagnostic {
                    range: lsp_types::Range { start, end: start },
                    severity: Some(if info.is_error {
                        DiagnosticSeverity::ERROR
                    } else {
                        DiagnosticSeverity::HINT
                    }),
                    message: diagnostic,
                    ..Default::default()
                });
            }
        },
    }

    diags
}

fn emulate(top: Expand, src: String) -> Vec<EmulateInfo> {
    let source: Rc<String> = src.into();
    let mut meta = CompileMeta::with_source(source.clone());
    meta.is_emulated = true;
    meta.set_extender(Box::new(Extender::new(source, DisplaySourceMeta::new().into())));

    let assert_meta = std::panic::AssertUnwindSafe(&mut meta);
    let _ = std::panic::catch_unwind(|| {
        top.compile(&mut {assert_meta}.0);
    });
    meta.emulate_infos.take()
}

struct Extender {
    source: Rc<String>,
    display_meta: RefCell<DisplaySourceMeta>,
}
impl Extender {
    fn new(source: Rc<String>, display_meta: RefCell<DisplaySourceMeta>) -> Self {
        Self {
            source,
            display_meta,
        }
    }
}
impl CompileMetaExtends for Extender {
    fn source_location(&self, index: usize) -> [syntax::Location; 2] {
        let (line, col) = line_column::line_column(&self.source, index);
        [line as syntax::Location, col as syntax::Location]
    }
    fn display_value(&self, value: &syntax::Value) -> Cow<'_, str> {
        let meta = &mut *self.display_meta.borrow_mut();
        meta.to_default();
        value.display_source_and_get(meta).to_owned().into()
    }
    fn display_binds(&self, value: syntax::BindsDisplayer<'_>) -> Cow<'_, str> {
        let meta = &mut *self.display_meta.borrow_mut();
        meta.to_default();
        value.display_source_and_get(meta).to_owned().into()
    }
}
