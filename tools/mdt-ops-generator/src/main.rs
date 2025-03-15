use std::{env::args, io::{stdin, Read}, process::exit};

type Res<'a> = (Vec<String>, &'a str);

fn run<'a>((((mut a, ah), (b, bh)), h): (
    (Res<'a>, Res<'a>),
    &'a str
), oper: &str) -> Res<'a> {
    a.extend(b);
    a.push(format!("op {oper} {h} {ah} {bh}"));
    (a, h)
}

peg::parser!(grammar parser() for str {
    rule sget<T>(p: rule<T>) -> (T, &'input str)
        = start:position!()
        value:p()
        src:#{|input, pos| {
            peg::RuleResult::Matched(pos, &input[start..pos])
        }}
        { (value, src) }
    rule var()
        = quiet!{
            ! ['0'..='9' | '-']
            ['a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '-' | '.' | ':' | '@' | '$']+
        } / expected!("var")
    rule number()
        = quiet!{
            "0x" "-"? ['0'..='9' | 'a'..='f' | 'A'..='F']+
            / "0b" "-"? ['0' | '1']+
            / "-"? ['0'..='9']+ (("." / "e" ['+'|'-']?) ['0'..='9']+)?
        } / expected!("number")
    rule funcname() -> &'input str
        = $(quiet!{
            "max" / "min" / "angle" / "angleDiff" / "len" / "noise" / "abs"
            / "log" / "log10" / "floor" / "ceil" / "sqrt" / "rand" / "sin"
            / "cos" / "tan" / "asin" / "acos" / "atan"
        }) / expected!("funcname")
    rule atom() -> Res<'input>
        = n:$(number() / var()) { (vec![], n) }
        / "(" e:equalty() ")" {e}
    rule call() -> Res<'input>
        = s:sget(<
            op:funcname() "(" a:equalty() b:("," b:equalty() {b})? ")"
            {(op, (a, b.unwrap_or((vec![], "0"))))}
        >) { let ((op, x), h) = s; run((x, h), op) }
        / atom()
    rule pow() -> Res<'input>
        = s:sget(<a:call() "**" b:pow() {(a, b)}>) { run(s, "pow") }
        / call()
    rule neg() -> Res<'input>
        = s:sget(<"!" a:neg() {(a, (vec![], "false"))}>) { run(s, "equal") }
        / s:sget(<"~" a:neg() {(a, (vec![], "0"))}>) { run(s, "not") }
        / s:sget(<"-" !['0'..='9'] b:neg() {((vec![], "0"), b)}>) { run(s, "sub") }
        / pow()
    #[cache_left_rec]
    rule mul() -> Res<'input>
        = s:sget(<a:mul() "*" b:neg() {(a, b)}>) { run(s, "mul") }
        / s:sget(<a:mul() "/" b:neg() {(a, b)}>) { run(s, "div") }
        / s:sget(<a:mul() "%" b:neg() {(a, b)}>) { run(s, "mod") }
        / s:sget(<a:mul() "//" b:neg() {(a, b)}>) { run(s, "idiv") }
        / neg()
    #[cache_left_rec]
    rule add() -> Res<'input>
        = s:sget(<a:add() "+" b:mul() {(a, b)}>) { run(s, "add") }
        / s:sget(<a:add() "-" b:mul() {(a, b)}>) { run(s, "sub") }
        / mul()
    #[cache_left_rec]
    rule shift() -> Res<'input>
        = s:sget(<a:shift() "<<" b:add() {(a, b)}>) { run(s, "shl") }
        / s:sget(<a:shift() ">>" b:add() {(a, b)}>) { run(s, "shr") }
        / add()
    #[cache_left_rec]
    rule band() -> Res<'input>
        = s:sget(<a:band() "&" b:shift() {(a, b)}>) { run(s, "and") }
        / shift()
    #[cache_left_rec]
    rule bxor() -> Res<'input>
        = s:sget(<a:bxor() "^" b:band() {(a, b)}>) { run(s, "xor") }
        / band()
    #[cache_left_rec]
    rule bor() -> Res<'input>
        = s:sget(<a:bor() "|" b:bxor() {(a, b)}>) { run(s, "or") }
        / bxor()
    rule cmp() -> Res<'input>
        = s:sget(<a:bor() "<" b:bor() {(a, b)}>) { run(s, "lessThan") }
        / s:sget(<a:bor() "<=" b:bor() {(a, b)}>) { run(s, "lessThanEq") }
        / s:sget(<a:bor() ">" b:bor() {(a, b)}>) { run(s, "greaterThan") }
        / s:sget(<a:bor() ">=" b:bor() {(a, b)}>) { run(s, "greaterThanEq") }
        / bor()
    rule equalty() -> Res<'input>
        = s:sget(<a:cmp() "==" b:cmp() {(a, b)}>) { run(s, "equal") }
        / s:sget(<a:cmp() "!=" b:cmp() {(a, b)}>) { run(s, "notEqual") }
        / s:sget(<a:cmp() "===" b:cmp() {(a, b)}>) { run(s, "strictEqual") }
        / cmp()

    rule _() = [' ' | '\t']*
    rule nl() = _ (quiet!{("\r"? "\n")+} / ![_]) / expected!("newline")
    rule job() -> Vec<String>
        = _ x:equalty() {x.0}
    pub
    rule jobs() -> Vec<String>
        = jobs:job()++nl() nl()?
        { jobs.into_iter().reduce(|mut a, b| { a.extend(b); a }).unwrap() }
});

fn main() {
    let args = args().skip(1).collect::<Vec<_>>();
    eprintln!("Generate some mindustry logic operations, \
              tmp var name using expression itself");
    eprintln!("Repo: {}", env!("CARGO_PKG_REPOSITORY"));
    let buf = &mut String::new();

    if args.len() == 0 {
        eprintln!("Reading from stdin...");
        stdin().read_to_string(buf).unwrap();
    } else {
        eprintln!("Reading from args...");
        for arg in args {
            buf.push_str(&arg);
            buf.push('\n');
        }
    }

    match parser::jobs(&buf) {
        Ok(output) => {
            for line in output {
                println!("{line}")
            }
        },
        Err(e) => {
            eprintln!("{e}");
            exit(1)
        },
    }
}
