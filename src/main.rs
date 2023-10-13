use std::{
    env::args,
    io::{
        stdin,
        Read
    },
    process::exit,
    str::FromStr,
    collections::HashMap,
};

use display_source::DisplaySource;
use lalrpop_util::{
    lexer::Token,
    ParseError
};
use mindustry_logic_bang_lang::{
    syntax::{
        CompileMeta,
        Error,
        Errors,
        Expand,
        Meta
    },
    syntax::def::TopLevelParser,
    tag_code::TagCodes,
};

/// 带有错误前缀, 并且文本为红色的eprintln
macro_rules! err {
    ( $($args:tt)* ) => {
        eprintln!("\x1b[1;31mMainError: {}\x1b[0m", format_args!($($args)*));
    };
}

macro_rules! concat_lines {
    ( $( $( $s:expr ),* ; )* ) => {
        concat!( $( $($s ,)* "\n" ),* )
    };
}

pub const HELP_MSG: &str = concat_lines! {
    "<MODE...>";
    "Author: A4-Tacks A4的钉子";
    "Version: ", env!("CARGO_PKG_VERSION");
    "https://github.com/A4-Tacks/mindustry_logic_bang_lang";
    "MODE:";
    "\t", "c: compile MdtBangLang to MdtLogicCode";
    "\t", "a: compile MdtBangLang to AST Debug";
    "\t", "A: compile MdtBangLang to MdtBangLang";
    "\t", "t: compile MdtBangLang to MdtTagCode";
    "\t", "T: compile MdtBangLang to MdtTagCode (Builded TagDown)";
    "\t", "f: compile MdtLogicCode to MdtTagCode";
    "\t", "F: compile MdtLogicCode to MdtTagCode (Builded TagDown)";
    "\t", "r: compile MdtLogicCode to MdtBangLang";
    "\t", "R: compile MdtLogicCode to MdtBangLang (Builded TagDown)";
    "\t", "C: compile MdtTagCode to MdtLogicCode";
    ;
    "input from stdin";
    "output to stdout";
    "error to stderr";
};

fn help() {
    eprint!("{} {}", args().next().unwrap(), HELP_MSG);
}

fn main() {
    let mut args = args();
    args.next().unwrap();
    let Some(mode) = args.next() else {
        err!("no MODE");
        help();
        exit(1)
    };
    if let Some(arg) = args.next() {
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
        src = mode.compile(&src)
    }
    println!("{src}")
}

enum CompileMode {
    BangToMdtLogic,
    BangToASTDebug,
    BangToASTDisplay,
    BangToMdtTagCode { tag_down: bool },
    MdtLogicToMdtTagCode { tag_down: bool },
    MdtLogicToBang { tag_down: bool },
    MdtTagCodeToMdtLogic,
}
impl CompileMode {
    fn compile(&self, src: &str) -> String {
        match *self {
            Self::BangToMdtLogic => {
                let ast = build_ast(src);
                let mut meta = compile_ast(ast);
                build_tag_down(&mut meta);
                let logic_lines = meta.tag_codes_mut().compile().unwrap();
                logic_lines.join("\n")
            },
            Self::BangToASTDebug => {
                let ast = build_ast(src);
                format!("{ast:#?}")
            },
            Self::BangToASTDisplay => {
                let ast = build_ast(src);
                display_ast(&ast)
            },
            Self::BangToMdtTagCode { tag_down } => {
                let ast = build_ast(src);
                let mut meta = compile_ast(ast);
                if tag_down { build_tag_down(&mut meta); }
                meta.tag_codes().to_string()
            },
            Self::MdtLogicToMdtTagCode { tag_down } => {
                match TagCodes::from_str(src) {
                    Ok(mut lines) => {
                        if tag_down {
                            lines.build_tagdown().unwrap();
                            lines.tag_up();
                        }
                        lines.to_string()
                    },
                    Err((line, e)) => {
                        err!("line: {}, {:?}", line, e);
                        exit(4);
                    },
                }
            },
            Self::MdtLogicToBang { tag_down } => {
                match TagCodes::from_str(src) {
                    Ok(mut lines) => {
                        if tag_down {
                            lines.build_tagdown().unwrap();
                            lines.tag_up();
                        }
                        let ast = Expand::try_from(&lines)
                            .unwrap_or_else(|(idx, e)| {
                                let lines_str = lines.iter()
                                    .map(|line| format!("\t{line}"))
                                    .collect::<Vec<_>>();
                                err!(
                                    "已构建的行:\n{}\n在构建第{}行时出错: {}",
                                    lines_str.join("\n"),
                                    idx + 1,
                                    e
                                );
                                exit(4);
                            });
                        display_ast(&ast)
                    },
                    Err((line, e)) => {
                        err!("line: {}, {:?}", line, e);
                        exit(4);
                    },
                }
            },
            Self::MdtTagCodeToMdtLogic => {
                let tag_codes = TagCodes::from_tag_lines(src);
                let mut meta = CompileMeta::with_tag_codes(tag_codes);
                build_tag_down(&mut meta);
                let logic_lines = meta.tag_codes_mut().compile().unwrap();
                logic_lines.join("\n")
            },
        }
    }
}
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
            'r' => Self::MdtLogicToBang { tag_down: false },
            'R' => Self::MdtLogicToBang { tag_down: true },
            'C' => Self::MdtTagCodeToMdtLogic,
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

fn build_tag_down(meta: &mut CompileMeta) {
    let tag_codes = meta.tag_codes_mut();
    tag_codes.build_tagdown()
        .unwrap_or_else(|(_line, tag)| {
            let (tag_str, _id) = meta.tags_map()
                .iter()
                .filter(|&(_k, v)| *v == tag)
                .take(1)
                .last()
                .unwrap();
            err!("重复的标记: {:?}", tag_str);
            exit(4)
        })
}

type ParseResult<'a> = Result<Expand, ParseError<usize, Token<'a>, Error>>;

fn build_ast(src: &str) -> Expand {
    let parser = TopLevelParser::new();
    let mut meta = Meta::new();
    unwrap_parse_err(parser.parse(&mut meta, src), src)
}

fn read_stdin() -> String {
    let mut buf = String::new();
    let _byte_count = stdin()
        .read_to_string(&mut buf).unwrap_or_else(|e| {
            err!("read from stdin error: {}", e);
            exit(3)
        });
    buf
}

/// 给定位置与源码, 返回行列号, 行列都从1开始<br/>
/// 如果没有找到, 则返回`[0, 0]`
fn get_locations<const N: usize>(src: &str, indexs: [usize; N]) -> [[usize; 2]; N] {
    const CR: char = '\n';

    let mut index_maps: HashMap<usize, Vec<usize>> = HashMap::with_capacity(indexs.len());
    for (i, loc) in indexs.into_iter().enumerate() {
        index_maps.entry(loc).or_default().push(i)
    }
    let mut res = [[0, 0]; N];
    let [mut line, mut column] = [1, 1];
    for (i, ch) in src.char_indices() {
        if let Some(idxs) = index_maps.get(&i) {
            for &idx in idxs {
                res[idx] = [line, column]
            }
        }
        if ch == CR {
            line += 1;
            column = 1;
        } else {
            column += 1;
        }
    }
    res
}

fn unwrap_parse_err<'a>(result: ParseResult<'a>, src: &str) -> Expand {
    match result {
        Ok(ast) => ast,
        Err(e) => {
            match e {
                ParseError::UnrecognizedToken {
                    token: (start, token, end),
                    expected
                } => {
                    let [start, end] = get_locations(src, [start, end]);
                    err!(
                        "在位置: {:?} 至 {:?} 处找到未知的令牌: {:?}, 预期: [{}]",
                        start, end,
                        token.1,
                        expected.join(", "),
                    );
                },
                ParseError::ExtraToken { token: (start, token, end) } => {
                    let [start, end] = get_locations(src, [start, end]);
                    err!(
                        "在位置: {:?} 至 {:?} 处找到多余的令牌: {:?}",
                        start, end,
                        token.1,
                    );
                },
                ParseError::InvalidToken { location } => {
                    let [start] = get_locations(src, [location]);
                    err!(
                        "在位置: {:?} 处找到无效的令牌",
                        start,
                    );
                },
                ParseError::UnrecognizedEof {
                    location,
                    expected
                } => {
                    let [start] = get_locations(src, [location]);
                    err!(
                        "在位置: {:?} 处找到未结束的令牌, 预期: [{}]",
                        start,
                        expected.join(", "),
                    );
                },
                ParseError::User {
                    error: Error {
                        start,
                        end,
                        err
                    }
                } => {
                    let [start, end]
                        = get_locations(src, [start, end]);
                    let head = format!(
                        "在 {:?} 至 {:?} 处的错误: ",
                        start,
                        end
                    );
                    match err {
                        Errors::NotALiteralUInteger(str, err) => {
                            err!(
                                "{}{:?} 不是一个有效的无符号整数, 错误: {}",
                                head,
                                str,
                                err,
                            );
                        },
                        Errors::SetVarNoPatternValue(var_count, val_count) => {
                            err!(
                                "{}sets两侧值数量不匹配, {} != {}",
                                head,
                                var_count,
                                val_count,
                            );
                        },
                        #[allow(unreachable_patterns)]
                        e => {
                            err!("未被枚举的错误: {:?}", e);
                        },
                    }
                },
            }
            exit(4)
        }
    }
}

fn compile_ast(ast: Expand) -> CompileMeta {
    CompileMeta::new().compile_res_self(ast)
}
