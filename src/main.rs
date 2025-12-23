use std::{
    env::args,
    io::{
        stdin,
        Read
    },
    mem,
    process::exit,
    fmt::Display,
    cell::RefCell,
    borrow::Cow,
    rc::Rc,
};

use display_source::{
    DisplaySource,
    DisplaySourceMeta,
};
use syntax::{
    CompileMeta,
    Error,
    Expand,
    Meta,
    CompileMetaExtends,
};
use parser::{
    TopLevelParser,
    lalrpop_util::{
        lexer::Token,
        ParseError
    },
};
use tag_code::{
    logic_parser::{parser as tparser, ParseLines}, TagCodes,
};
use logic_lint::Source;

/// 带有错误前缀, 并且文本为红色的eprintln
macro_rules! err {
    ( $($args:tt)* ) => {{
        let str = format!($($args)*);
        let mut iter = str.lines();
        eprintln!("\x1b[1;31mMainError: {}\x1b[22;39m", iter.next().unwrap());
        for line in iter {
            eprintln!("    \x1b[1;31m{}\x1b[22;39m", line);
        }
    }};
}

macro_rules! concat_lines {
    ( $( $( $s:expr ),* ; )* ) => {
        concat!( $( $($s ,)* "\n" ),* )
    };
}

const MAX_INVALID_TOKEN_VIEW: usize = 5;

fn help() {
    print!("Usage: {} {}", args().next().unwrap(), HELP_MSG);
}

fn main() {
    let args = args().skip(1).collect::<Vec<_>>();
    if args.iter().any(|arg| arg == "-h" || arg == "--help") {
        help();
        exit(0)
    }
    let Some(mode) = args.first() else {
        err!("missing MODE args");
        help();
        exit(1)
    };
    if let Some(arg) = args.get(1) {
        err!("多余的参数: {:?}", arg);
        exit(2);
    }
    let modes = Vec::from_iter(
        mode.chars()
            .map(|char| {
                CompileMode::try_from(char).unwrap_or_else(|mode| {
                    err!("mode {mode:?} no pattern");
                    help();
                    exit(2)
                })
            })
    );
    let mut src = read_stdin();
    for mode in modes {
        src = mode.compile(src)
    }
    println!("{src}")
}

enum CompileMode {
    BangToMdtLogic,
    BangToASTDebug,
    BangToASTDisplay,
    BangToMdtTagCode { tag_down: bool },
    MdtLogicToMdtTagCode { tag_down: bool },
    MdtLogicToBang,
    MdtTagCodeToMdtLogic,
    LintLogic,
    IndentLogic,
    RenameLabel,
    BangToMdtLabel,
    BuildExpr,
    ParenToMdtLogic,
}
impl CompileMode {
    fn compile(&self, src: String) -> String {
        match *self {
            Self::BangToMdtLogic => {
                let ast = build_ast(&src);
                let mut meta = compile_ast(ast, src.clone());
                let logic_codes = mem::take(meta.parse_lines_mut());
                let mut tag_codes = logic_to_tagcode(logic_codes, &src);
                build_tag_down(&mut tag_codes);
                let logic_lines = tag_codes.compile().unwrap();
                logic_lines.join("\n")
            },
            Self::BangToASTDebug => {
                let ast = build_ast(&src);
                format!("{ast:#?}")
            },
            Self::BangToASTDisplay => {
                let ast = build_ast(&src);
                display_ast(&ast)
            },
            Self::BangToMdtTagCode { tag_down } => {
                let ast = build_ast(&src);
                let mut meta = compile_ast(ast, src.clone());
                let mut tag_codes = logic_to_tagcode(mem::take(meta.parse_lines_mut()), &src);
                if tag_down { build_tag_down(&mut tag_codes); }
                tag_codes.to_string()
            },
            Self::MdtLogicToMdtTagCode { tag_down } => {
                let mut lines = logic_src_to_tagcode(&src);
                if tag_down {
                    lines.build_tagdown().unwrap();
                    lines.tag_up();
                }
                lines.to_string()
            },
            Self::MdtLogicToBang => {
                let logic_lines = logic_parse(&src);
                let ast = Expand::try_from(logic_lines)
                    .unwrap_or_else(|e| {
                        let (line, col) = e.location(&src);
                        err!("MdtLogicToBang {line}:{col} {}", e.value);
                        exit(4)
                    });
                display_ast(&ast)
            },
            Self::MdtTagCodeToMdtLogic => {
                let mut tag_codes = logic_src_to_tagcode(&src);
                build_tag_down(&mut tag_codes);
                let logic_lines = tag_codes.compile().unwrap();
                logic_lines.join("\n")
            },
            Self::LintLogic => {
                let linter = Source::from_str(&src);
                linter.show_lints();
                src
            },
            Self::IndentLogic => {
                let mut logic_lines = logic_parse(&src);
                logic_lines.index_label_popup();
                format!("{logic_lines:#}")
            },
            Self::RenameLabel => {
                let mut logic_lines = logic_parse(&src);
                logic_lines.for_each_inner_label_mut(|mut lab| {
                    lab.to_mut().push_str("_RENAME");
                });
                format!("{logic_lines:#}")
            },
            Self::BangToMdtLabel => {
                let ast = build_ast(&src);
                let mut meta = compile_ast(ast, src.clone());
                meta.parse_lines_mut().index_label_popup();
                format!("{}", meta.parse_lines())
            },
            Self::BuildExpr => {
                let lines = logic_parse(&src);
                let out = tag_code::expr_builder::build(lines.iter()
                    .map(|x| &**x));
                out.join("\n")
            },
            Self::ParenToMdtLogic => {
                let lines = match mini_paren::parser::lines(&src) {
                    Ok(lines) => lines,
                    Err(e) => {
                        err!("ParseParenCode {}:{} expected {}",
                            e.location.line,
                            e.location.column,
                            e.expected,
                        );
                        exit(9)
                    },
                };
                let mut state = mini_paren::State::default();
                state.process_lines(&lines);
                state.out.truncate(state.out.trim_end().len());
                state.out
            },
        }
    }
}

fn logic_to_tagcode<'a>(lines: ParseLines<'a>, src: &str) -> TagCodes {
    let tagcodes = match TagCodes::try_from(lines) {
        Ok(tagcode) => tagcode,
        Err(e) => {
            let (line, column) = e.location(src);
            let prefix = format!("ParseTagCode {line}:{column}");
            err!("{prefix} {e}\n或许你可以使用`Li`选项编译来详细查看");
            exit(10)
        },
    };
    tagcodes
}

fn logic_parse(src: &str) -> ParseLines<'_> {
    match tparser::lines(src) {
        Ok(lines) => lines,
        Err(e) => {
            err!("ParseLogicCode {}:{} expected {}",
                e.location.line,
                e.location.column,
                e.expected,
            );
            exit(9)
        },
    }
}

fn logic_src_to_tagcode(src: &str) -> TagCodes {
    let lines = logic_parse(src);
    logic_to_tagcode(lines, src)
}
pub const HELP_MSG: &str = concat_lines! {
    "<MODE...>";
    env!("CARGO_PKG_DESCRIPTION");
    ;
    "MODE:";
    "\t", "c: compile MdtBangLang to MdtLogicCode";
    "\t", "a: compile MdtBangLang to AST Debug";
    "\t", "A: compile MdtBangLang to MdtBangLang";
    "\t", "t: compile MdtBangLang to MdtTagCode";
    "\t", "T: compile MdtBangLang to MdtTagCode (Builded TagDown)";
    "\t", "f: compile MdtLogicCode to MdtTagCode";
    "\t", "F: compile MdtLogicCode to MdtTagCode (Builded TagDown)";
    "\t", "r: compile MdtLogicCode to MdtBangLang";
    "\t", "C: compile MdtTagCode to MdtLogicCode";
    "\t", "l: lint MdtLogicCode";
    "\t", "i: indent MdtLogicCode";
    "\t", "n: rename MdtLogicCode";
    "\t", "L: compile MdtBangLang to MdtLabelCode";
    "\t", "b: compile MdtLogicCode to expressions";
    "\t", "p: compile MdtParenCode to MdtLogicCode";
    ;
    "input from stdin";
    "output to stdout";
    "error to stderr";
    "Learning this language, from mindustry_logic_bang_lang/examples/README.md";
    ;
    "Repository: https://github.com/A4-Tacks/mindustry_logic_bang_lang";
    "Author: A4-Tacks A4的钉子";
    "Version: ", env!("CARGO_PKG_VERSION");
};
impl TryFrom<char> for CompileMode {
    type Error = char;

    fn try_from(mode: char) -> Result<Self, Self::Error> {
        Ok(match mode {
            'c' => Self::BangToMdtLogic,
            'a' => Self::BangToASTDebug,
            'A' => Self::BangToASTDisplay,
            't' => Self::BangToMdtTagCode { tag_down: false },
            'T' => Self::BangToMdtTagCode { tag_down: true },
            'f' => Self::MdtLogicToMdtTagCode { tag_down: false },
            'F' => Self::MdtLogicToMdtTagCode { tag_down: true },
            'r' => Self::MdtLogicToBang,
            'C' => Self::MdtTagCodeToMdtLogic,
            'l' => Self::LintLogic,
            'i' => Self::IndentLogic,
            'n' => Self::RenameLabel,
            'L' => Self::BangToMdtLabel,
            'b' => Self::BuildExpr,
            'p' => Self::ParenToMdtLogic,
            mode => return Err(mode),
        })
    }
}

fn display_ast(ast: &Expand) -> String {
    let mut meta = Default::default();
    ast.display_source(&mut meta);
    let _ = meta.pop_lf();
    meta.buffer().into()
}

fn build_tag_down(tag_codes: &mut TagCodes) {
    let result = tag_codes.build_tagdown();
    result.unwrap_or_else(|(line, tag)| {
        err!("重复的标记: {tag:?} (line {line})");
        exit(4)
    })
}

type ParseResult<'a> = Result<Expand, ParseError<usize, Token<'a>, Error>>;

fn build_ast(src: &str) -> Expand {
    let parser = TopLevelParser::new();
    let mut meta = Meta::new();
    unwrap_parse_err(parser.parse(&mut meta, src), src)
}

fn read_stdin_unwrapper(e: impl Display) -> ! {
    err!("read from stdin error: {}", e);
    exit(3)
}

fn read_stdin() -> String {
    let mut buf = String::new();
    let _byte_count = stdin()
        .read_to_string(&mut buf)
        .unwrap_or_else(|e| read_stdin_unwrapper(e));
    buf
}

fn unwrap_parse_err(result: ParseResult<'_>, src: &str) -> Expand {
    match result {
        Ok(ast) => ast,
        Err(e) => {
            err!("{}", parser::format_parse_err::<MAX_INVALID_TOKEN_VIEW>(e, src));
            exit(4)
        }
    }
}

struct CompileMetaExtender {
    source: Rc<String>,
    display_meta: RefCell<DisplaySourceMeta>,
}
impl CompileMetaExtender {
    fn new(source: Rc<String>, display_meta: RefCell<DisplaySourceMeta>) -> Self {
        Self {
            source,
            display_meta,
        }
    }
}
impl CompileMetaExtends for CompileMetaExtender {
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

fn compile_ast(ast: Expand, src: String) -> CompileMeta {
    let mut meta = CompileMeta::new();
    let src = Rc::new(src);
    meta.set_extender(Box::new(CompileMetaExtender::new(
        src.clone(),
        DisplaySourceMeta::new().into(),
    )));
    meta.set_source(src);
    meta.compile_res_self(ast)
}
