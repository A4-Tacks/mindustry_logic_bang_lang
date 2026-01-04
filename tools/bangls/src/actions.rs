use std::{collections::HashMap, mem};

use anyhow::{Result, bail};
use display_source::{DisplaySource, DisplaySourceMeta};
use itertools::Itertools;
use line_column::line_column;
use lsp_types::{CodeActionParams, TextEdit, WorkspaceEdit};
use syntax::EmulateConfig;
use tag_code::TagCodes;

use crate::{Ctx, emulate, rgpos};

pub struct Info {
    pub handler: Box<dyn FnOnce(&mut Ctx) -> Result<WorkspaceEdit>>,
}
pub type Handle = fn(ctx: &Ctx, param: &CodeActionParams) -> Option<(String, Info)>;

pub fn all() -> &'static [Handle] {
    &[
        compile_to_logic_code,
        compile_to_desugar,
        compile_to_logic_label_code
    ]
}

fn compile_to_logic_code(ctx: &Ctx, param: &CodeActionParams) -> Option<(String, Info)> {
    let uri = param.text_document.uri.clone();
    let _file = ctx.open_files.get(&uri)?;

    let handler = Box::new(move |ctx: &mut Ctx| {
        let file = ctx.open_files.get(&uri).map_or("", |it| it);
        let expand = parse_file(ctx, file)?;
        let (infos, mut meta) = emulate(expand, file.to_owned(), EmulateConfig {
            diagnostics: true,
            record_free_info: true,
            ..Default::default()
        });
        let parse_lines = mem::take(meta.parse_lines_mut());
        let mut tag_codes = match TagCodes::try_from(parse_lines) {
            Ok(it) => it,
            Err(e) => {
                bail!("{}", e.value.to_string().replace('\n', " , "))
            },
        };
        let compiled = match tag_codes.compile() {
            Ok(it) => it,
            Err(_) => {
                bail!("有重复的 tag")
            },
        };
        let diagnostics = infos.into_iter()
            .filter_map(|info| info.diagnostic)
            .map(|diag| format!("# {}", diag.replace('\n', "\n# ")));
        let output_lines = diagnostics.chain(compiled).join("\n");

        Ok(WorkspaceEdit::new(HashMap::from_iter([
            (uri, [
                TextEdit::new(full_range(file), output_lines)
            ].into()),
        ])))
    });
    Some(("Compile to LogicCode".to_owned(), Info {
        handler,
    }))
}

fn compile_to_desugar(ctx: &Ctx, param: &CodeActionParams) -> Option<(String, Info)> {
    let uri = param.text_document.uri.clone();
    let _file = ctx.open_files.get(&uri)?;

    let handler = Box::new(move |ctx: &mut Ctx| {
        let file = ctx.open_files.get(&uri).map_or("", |it| it);
        let expand = parse_file(ctx, file)?;

        let display = expand.display_source_and_get(&mut DisplaySourceMeta::new()).to_owned();

        Ok(WorkspaceEdit::new(HashMap::from_iter([
            (uri, [
                TextEdit::new(full_range(file), display)
            ].into())
        ])))
    });
    Some(("Compile to BangLang (desugar)".to_owned(), Info {
        handler,
    }))
}

fn compile_to_logic_label_code(ctx: &Ctx, param: &CodeActionParams) -> Option<(String, Info)> {
    let uri = param.text_document.uri.clone();
    let _file = ctx.open_files.get(&uri)?;

    let handler = Box::new(move |ctx: &mut Ctx| {
        let file = ctx.open_files.get(&uri).map_or("", |it| it);
        let expand = parse_file(ctx, file)?;
        let (infos, mut meta) = emulate(expand, file.to_owned(), EmulateConfig {
            diagnostics: true,
            record_free_info: true,
            ..Default::default()
        });
        let mut parse_lines = mem::take(meta.parse_lines_mut());
        parse_lines.index_label_popup();
        let diagnostics = infos.into_iter()
            .filter_map(|info| info.diagnostic)
            .map(|diag| format!("# {}\n", diag.replace('\n', "\n# ")));
        let output_lines = format!("{}{parse_lines:#}", diagnostics.format(""));

        Ok(WorkspaceEdit::new(HashMap::from_iter([
            (uri, [
                TextEdit::new(full_range(file), output_lines)
            ].into()),
        ])))
    });
    Some(("Compile to LogicCode (Label)".to_owned(), Info {
        handler,
    }))
}

fn parse_file(ctx: &Ctx, file: &str) -> Result<syntax::Expand> {
    match ctx.parse_for_parse_error(file) {
        Err((_, e)) => bail!("{}", e.replace('\n', " , ")),
        Ok(expand) => Ok(expand),
    }
}

fn full_range(file: &str) -> lsp_types::Range {
    let end = rgpos(line_column(file, file.len()));
    let start = rgpos((1, 1));
    let range = lsp_types::Range { start, end };
    range
}
