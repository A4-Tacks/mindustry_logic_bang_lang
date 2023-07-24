use std::{
    env::args,
    io::{
        stdin,
        Read
    },
    process::exit,
    str::FromStr,
};

use lalrpop_util::{
    lexer::Token,
    ParseError
};
use mindustry_logic_bang_lang::{
    syntax::{
        CompileMeta,
        Error,
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
    "https://github.com/A4-Tacks/mindustry_logic_bang_lang";
    "MODE:";
    "\t", "c: compile MdtBangLang to MdtLogicCode";
    "\t", "a: compile MdtBangLang to AST Debug";
    "\t", "t: compile MdtBangLang to MdtTagCode";
    "\t", "T: compile MdtBangLang to MdtTagCode (Builded TagDown)";
    "\t", "f: compile MdtLogicCode to MdtTagCode";
    "\t", "F: compile MdtLogicCode to MdtTagCode (Builded TagDown)";
    ;
    "input from stdin";
    "output to stdout";
    "error to stderr";
};

fn help() {
    eprint!("{} {}", args().next().unwrap(), HELP_MSG);
}

macro_rules! err {
    ( $fmtter:expr $(, $args:expr)* $(,)? ) => {
        eprintln!(concat!("\x1b[1;31m", "Error: ", $fmtter, "\x1b[0m"), $($args),*);
    };
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
    unwrap_parse_err(parser.parse(&mut meta, &buf))
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

fn unwrap_parse_err<'a>(result: ParseResult<'a>) -> Expand {
    match result {
        Ok(ast) => ast,
        Err(e) => {
            dbg!(&e);
            exit(4)
        }
    }
}

fn compile_ast(ast: Expand) -> CompileMeta {
    CompileMeta::new().compile_res_self(ast)
}
