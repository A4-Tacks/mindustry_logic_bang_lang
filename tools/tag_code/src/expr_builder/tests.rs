use super::*;

peg::parser!(grammar simple_parser() for str {
    rule _() = " "*
    rule var() -> Var
        = s:$("-"? ['a'..='z' | '0'..='9' | '_']+)
        { s.into() }
    rule sin() -> Sin
        = "not" { Sin::Not }
        / "abs" { Sin::Abs }
        / "sign" { Sin::Sign }
        / "log" { Sin::Log }
        / "log10" { Sin::Log10 }
        / "floor" { Sin::Floor }
        / "ceil" { Sin::Ceil }
        / "round" { Sin::Round }
        / "sqrt" { Sin::Sqrt }
        / "rand" { Sin::Rand }
        / "sin" { Sin::Sin }
        / "cos" { Sin::Cos }
        / "tan" { Sin::Tan }
        / "asin" { Sin::Asin }
        / "acos" { Sin::Acos }
        / "atan" { Sin::Atan }
    rule bin() -> Bin
        = "add" { Bin::Add }
        / "sub" { Bin::Sub }
        / "mul" { Bin::Mul }
        / "div" { Bin::Div }
        / "idiv" { Bin::Idiv }
        / "mod" { Bin::Mod }
        / "emod" { Bin::Emod }
        / "pow" { Bin::Pow }
        / "land" { Bin::Land }
        / "equal" { Bin::Equal }
        / "notEqual" { Bin::NotEqual }
        / "lessThan" { Bin::LessThan }
        / "lessThanEq" { Bin::LessThanEq }
        / "greaterThan" { Bin::GreaterThan }
        / "greaterThanEq" { Bin::GreaterThanEq }
        / "strictEqual" { Bin::StrictEqual }
        / "shl" { Bin::Shl }
        / "shr" { Bin::Shr }
        / "ushr" { Bin::Ushr }
        / "or" { Bin::Or }
        / "xor" { Bin::Xor }
        / "and" { Bin::And }
        / "max" { Bin::Max }
        / "min" { Bin::Min }
        / "angle" { Bin::Angle }
        / "angleDiff" { Bin::AngleDiff }
        / "len" { Bin::Len }
        / "noise" { Bin::Noise }
        / "logn" { Bin::Logn }
        / "read" { Bin::Read }
        / "sense" { Bin::Sense }
    pub
    rule expr() -> Expr = precedence! {
        cond:@ _ "?" a:expr() _ ":" _ b:(@)
        {
            match cond {
                Expr::Binary(cmp, ca, cb) if Cmp::try_from(cmp).is_ok() => {
                    Expr::Select(
                        cmp.try_into().unwrap(),
                        ca,
                        cb,
                        a.into(),
                        b.into(),
                    )
                },
                _ => {
                    Expr::Select(
                        Cmp::NotEqual,
                        cond.into(),
                        Expr::Var("false".into()).into(),
                        a.into(),
                        b.into(),
                    )
                }
            }
        }
        --
        a:(@) _ "&&" _ b:@  { Expr::Binary(Bin::Land, a.into(), b.into()) }
        --
        a:(@) _ "=="  _ b:@ { Expr::Binary(Bin::Equal,         a.into(), b.into()) }
        a:(@) _ "!="  _ b:@ { Expr::Binary(Bin::NotEqual,      a.into(), b.into()) }
        a:(@) _ "===" _ b:@ { Expr::Binary(Bin::StrictEqual,   a.into(), b.into()) }
        --
        a:(@) _ "<"   _ b:@ { Expr::Binary(Bin::LessThan,      a.into(), b.into()) }
        a:(@) _ "<="  _ b:@ { Expr::Binary(Bin::LessThanEq,    a.into(), b.into()) }
        a:(@) _ ">"   _ b:@ { Expr::Binary(Bin::GreaterThan,   a.into(), b.into()) }
        a:(@) _ ">="  _ b:@ { Expr::Binary(Bin::GreaterThanEq, a.into(), b.into()) }
        --
        a:(@) _ "|"   _ b:@ { Expr::Binary(Bin::Or,   a.into(), b.into()) }
        --
        a:(@) _ "^"   _ b:@ { Expr::Binary(Bin::Xor,  a.into(), b.into()) }
        --
        a:(@) _ "&"   _ b:@ { Expr::Binary(Bin::And,  a.into(), b.into()) }
        --
        a:(@) _ "<<"  _ b:@ { Expr::Binary(Bin::Shl,  a.into(), b.into()) }
        a:(@) _ ">>"  _ b:@ { Expr::Binary(Bin::Shr,  a.into(), b.into()) }
        a:(@) _ ">>>" _ b:@ { Expr::Binary(Bin::Ushr, a.into(), b.into()) }
        --
        a:(@) _ "+"   _ b:@ { Expr::Binary(Bin::Add,  a.into(), b.into()) }
        a:(@) _ "-"   _ b:@ { Expr::Binary(Bin::Sub,  a.into(), b.into()) }
        --
        a:(@) _ "*"   _ b:@ { Expr::Binary(Bin::Mul,  a.into(), b.into()) }
        a:(@) _ "/"   _ b:@ { Expr::Binary(Bin::Div,  a.into(), b.into()) }
        a:(@) _ "//"  _ b:@ { Expr::Binary(Bin::Idiv, a.into(), b.into()) }
        a:(@) _ "%"   _ b:@ { Expr::Binary(Bin::Mod,  a.into(), b.into()) }
        a:(@) _ "%%"  _ b:@ { Expr::Binary(Bin::Emod, a.into(), b.into()) }
        --
        "~" _ b:(@)         { Expr::Single(Sin::Not, b.into()) }
        --
        a:@ _ "**" _ b:(@)  { Expr::Binary(Bin::Pow, a.into(), b.into()) }
        --
        f:sin() _ b:(@)     { Expr::Single(f, b.into()) }
        f:bin() _ "(" _ a:expr() _ "," _ b:expr() _ ")"
                            { Expr::Binary(f, a.into(), b.into()) }
        --
        "(" _ e:expr() _ ")" {e}
        v:var() { v.into() }
    }
});

#[test]
fn test_display() {
    let s = r#"
        a
        ~2
        ~2 ** 3
        ~(a + b)
        ~~a
        ~~a + b
        abs x
        abs abs x
        abs x + 2
        abs(x * 2) + 2
        len(2, 3) + 2
        a + b + c + (d + e) + f * g + h * i
        a == b && c == d && e == f && g == i && j == k
        a == b && c == d && (e == f && g == i) && j == k
        a == b && c == d && ~(e == f && g == i) && j == k
        a == b && c == d && ~(e == f) && g == i && j == k
        a == b && c == d && ~(e == f) && g === i && j == k
        sin abs len(x, y) %% m
        sin abs(~len(x, y)) %% m
        sin(~abs(~len(x, y))) %% m
    "#;

    for input in s.lines().map(str::trim).filter(|s| !s.is_empty()) {
        println!("--");
        let expr = simple_parser::expr(input).expect(input);
        let formatted = format!("{expr}");
        assert_eq!(input, formatted);
    }
}

#[test]
fn test_example_build() {
    let s = r#"
    op add x a b
    op abs x x 1
    foo
    op add m a b
    sensor m @unit @x-y
    select result notEqual x false m 3

    "#;
    let lines = crate::logic_parser::parser::lines(s).unwrap();
    build(lines.iter().map(|line| &**line));
}
