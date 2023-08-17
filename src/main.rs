use std::{
    env::args,
    io::{
        stdin,
        Read
    },
    process::exit,
    str::FromStr,
    collections::HashMap,
    mem::replace,
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
    syntax_def::ExpandParser,
    tag_code::TagCodes,
};

/// 带有错误前缀, 并且文本为红色的eprintln
macro_rules! err {
    ( $fmtter:expr $(, $args:expr)* $(,)? ) => {
        eprintln!(concat!("\x1b[1;31m", "MainError: ", $fmtter, "\x1b[0m"), $($args),*);
    };
}

macro_rules! concat_lines {
    ( $( $( $s:expr ),* ; )* ) => {
        concat!( $( $($s ,)* "\n" ),* )
    };
}

pub const HELP_MSG: &str = concat_lines! {
    "<MODE>";
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
    "\t", "R: compile MdtLogicCode to MdtBangLang";
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
    match &*mode {
        "c" => {
            let ast = from_stdin_build_ast();
            let mut meta = compile_ast(ast);
            build_tag_down(&mut meta);
            let logic_lines = meta.tag_codes_mut().compile().unwrap();
            for line in logic_lines {
                println!("{}", line);
            }
        },
        "a" => {
            let ast = from_stdin_build_ast();
            println!("{:#?}", ast)
        },
        "A" => {
            let ast = from_stdin_build_ast();
            display_ast(&ast);
        },
        "t" => {
            let ast = from_stdin_build_ast();
            let meta = compile_ast(ast);
            for line in meta.tag_codes().lines() {
                println!("{}", line)
            }
        },
        "T" => {
            let ast = from_stdin_build_ast();
            let mut meta = compile_ast(ast);
            build_tag_down(&mut meta);
            for line in meta.tag_codes().lines() {
                println!("{}", line)
            }
        },
        "f" => {
            match TagCodes::from_str(&read_stdin()) {
                Ok(lines) => {
                    for line in lines.lines() {
                        println!("{}", line)
                    }
                },
                Err((line, e)) => {
                    err!("line: {}, {:?}", line, e);
                    exit(4);
                },
            }
        },
        "F" => {
            match TagCodes::from_str(&read_stdin()) {
                Ok(mut lines) => {
                    lines.build_tagdown().unwrap();
                    lines.tag_up();
                    for line in lines.lines() {
                        println!("{}", line)
                    }
                },
                Err((line, e)) => {
                    err!("line: {}, {:?}", line, e);
                    exit(4);
                },
            }
        },
        "C" => {
            let src = read_stdin();
            let tag_codes = TagCodes::from_tag_lines(&src);
            let mut meta = CompileMeta::new();
            // 将我构建好的TagCodes换入并drop掉老的
            drop(replace(meta.tag_codes_mut(), tag_codes));
            build_tag_down(&mut meta);
            let logic_lines = meta.tag_codes_mut().compile().unwrap();
            for line in logic_lines {
                println!("{}", line);
            }
        },
        "R" => {
            match TagCodes::from_str(&read_stdin()) {
                Ok(lines) => {
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
                    display_ast(&ast);
                },
                Err((line, e)) => {
                    err!("line: {}, {:?}", line, e);
                    exit(4);
                },
            }
        },
        mode => {
            err!("mode {:?} no pattern", mode);
            help();
            exit(2)
        },
    }
}

fn display_ast(ast: &Expand) {
    let mut meta = Default::default();
    ast.display_source(&mut meta);
    assert!(meta.pop_lf());
    println!("{}", meta.buffer());
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

fn from_stdin_build_ast() -> Expand {
    build_ast(&read_stdin())
}

fn build_ast(src: &str) -> Expand {
    let parser = ExpandParser::new();
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
