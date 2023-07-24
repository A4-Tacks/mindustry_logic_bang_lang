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

use lalrpop_util::{
    lexer::Token,
    ParseError
};
use mindustry_logic_bang_lang::{
    err,
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
    "\t", "t: compile MdtBangLang to MdtTagCode";
    "\t", "T: compile MdtBangLang to MdtTagCode (Builded TagDown)";
    "\t", "f: compile MdtLogicCode to MdtTagCode";
    "\t", "F: compile MdtLogicCode to MdtTagCode (Builded TagDown)";
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
        mode => {
            err!("mode {:?} no pattern", mode);
            help();
            exit(2)
        },
    }
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
    let parser = ExpandParser::new();
    let mut meta = Meta::new();
    let buf = read_stdin();
    unwrap_parse_err(parser.parse(&mut meta, &buf), &buf)
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
                        err: Errors::NotALiteralUInteger(str, err)
                    }
                } => {
                    let [start, end] = get_locations(src, [start, end]);
                    err!(
                        "在 {:?} 至 {:?} 处的错误: {:?} 不是一个有效的无符号整数, 错误: {}",
                        start, end,
                        str, err,
                    );
                },
                #[allow(unreachable_patterns)]
                e => {
                    err!("未被枚举的错误: {:?}", e);
                },
            }
            exit(4)
        }
    }
}

fn compile_ast(ast: Expand) -> CompileMeta {
    CompileMeta::new().compile_res_self(ast)
}
