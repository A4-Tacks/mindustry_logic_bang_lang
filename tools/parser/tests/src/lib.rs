#![cfg(test)]
use ::parser::*;
use ::syntax::*;
use ::tag_code::*;
use ::either::Either::{self, Left, Right};
use logic_parser::{ParseLine, IdxBox};

/// 快捷的创建一个新的`Meta`并且`parse`
macro_rules! parse {
    ( $parser:expr, $src:expr ) => {{
        let meta = &mut Meta::new();
        meta.testing = true;
        ($parser).parse(meta, $src)
    }};
}

macro_rules! check_sugar {
    ( $parser:expr, $a:expr, $b:expr $(,)? ) => {
        let (a, b) = (parse!($parser, $a), parse!($parser, $b));
        if a != b {
            let (a, b) = (format!("{a:#?}"), format!("{b:#?}"));
            differ(&a, &b);
            panic!("check sugar fail");
        }
    };
}

macro_rules! check_compile {
    ( $parser:expr, $a:expr, $b:expr $(,)? ) => {{
        let meta = CompileMeta::new();
        let meta = check_compile!{@compile(meta) $parser, $a, $b};
        meta.hit_log(0);
        meta
    }};
    (@with_source $parser:expr, $a:expr, $b:expr $(,)? ) => {{
        let src = $a;
        let meta = CompileMeta::with_source(src.to_owned().into());
        check_compile!{@compile(meta) $parser, src, $b}
    }};
    (@compile($meta:ident) $parser:expr, $a:expr, $b:expr $(,)? ) => {{
        println!("check_compile, line {}", line!());
        let mut meta = $meta.compile_res_self(parse!($parser, $a).unwrap());
        let lines = std::mem::take(meta.parse_lines_mut());
        let linked = lines.compile().unwrap();
        check_compile_result(linked, $b);
        meta
    }};
}

macro_rules! check_compile_eq {
    ( $parser:expr, $a:expr, $b:expr $(,)? ) => {{
        check_compile!{$parser,
            $a,
            &CompileMeta::new().compile(
                parse!($parser, $b).unwrap()
            ).compile().unwrap().join("\n"),
        };
    }};
}

#[track_caller]
fn check_compile_result(current: Vec<String>, expected: &str) {
    let lines = expected.lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty());
    if lines.clone().eq(&current) {
        return;
    }

    differ(&current.join("\n"), &lines.collect::<Vec<_>>().join("\n"));

    println!();
    panic!("compile result mismatch")
}

#[track_caller]
fn differ(current: &str, expect: &str) {
    println!("-- current --");
    println!("{current}");
    println!();
    println!("-- expected --");
    println!("{expect}");

    println!();
    println!("-- diff --");

    for diff_chunk in dissimilar::diff(
        &expect,
        &current,
    ) {
        match diff_chunk {
            dissimilar::Chunk::Equal(s) => eprint!("{s}"),
            dissimilar::Chunk::Delete(s) => eprint!("\x1b[41m{s}\x1b[m\x1b[K"),
            dissimilar::Chunk::Insert(s) => eprint!("\x1b[42m{s}\x1b[m\x1b[K"),
        }
    }
}

trait PCompile {
    type E;
    fn compile(self) -> Result<Vec<String>, Self::E>;
}
impl<'a> PCompile for tag_code::logic_parser::ParseLines<'a> {
    type E = Either<IdxBox<ParseTagCodesError>, (usize, Tag)>;
    fn compile(self) -> Result<Vec<String>, Self::E> {
        let mut tagcodes = TagCodes::try_from(self).map_err(Left)?;
        tagcodes.compile().map_err(Right)
    }
}

trait HitLog {
    fn hit_log(&self, expected: usize);
}
impl HitLog for CompileMeta {
    #[track_caller]
    fn hit_log(&self, expected: usize) {
        assert_eq!(self.log_count(), expected, "log count mismatch");
    }
}

#[test]
fn var_test() {
    let parser = VarParser::new();

    assert_eq!(parse!(parser, "_abc").unwrap(), "_abc");
    assert_eq!(parse!(parser, "'ab-cd'").unwrap(), "ab-cd");
    assert_eq!(parse!(parser, "'ab.cd'").unwrap(), "ab.cd");
    assert_eq!(parse!(parser, "0x1_b").unwrap(), "0x1b");
    assert_eq!(parse!(parser, "-4_3_.7_29").unwrap(), "-43.729");
    assert_eq!(parse!(parser, "0b-00_10").unwrap(), "0b-0010");
    assert_eq!(parse!(parser, "-0b00_10").unwrap(), "-0b0010");
    assert_eq!(parse!(parser, "-0x00_10").unwrap(), "-0x0010");
    assert_eq!(parse!(parser, "-0x00_1f").unwrap(), "-0x001f");
    assert_eq!(parse!(parser, "@abc-def").unwrap(), "@abc-def");
    assert_eq!(parse!(parser, "@abc-def_30").unwrap(), "@abc-def_30");
    assert_eq!(parse!(parser, "@abc-def-34").unwrap(), "@abc-def-34");
    assert_eq!(parse!(parser, r#"'abc"def'"#).unwrap(), "abc'def"); // 双引号被替换为单引号

    assert!(parse!(parser, "'ab cd'").is_err());
    assert!(parse!(parser, "ab-cd").is_err());
    assert!(parse!(parser, "0o25").is_err()); // 不支持8进制, 懒得弄转换
    assert!(parse!(parser, r"@ab\c").is_err());
    assert!(parse!(parser, "-_2").is_err());
    assert!(parse!(parser, "-0._3").is_err());
    assert!(parse!(parser, "0x_2").is_err());
}

#[test]
fn expand_test() {
    let parser = TopLevelParser::new();
    let lines = parse!(parser, r#"
    op + a a 1;
    op - a a 1;
    op a a * 2;
    "#).unwrap();
    let mut iter = lines.iter();
    assert_eq!(iter.next().unwrap(), &Op::Add("a".into(), "a".into(), "1".into()).into());
    assert_eq!(iter.next().unwrap(), &Op::Sub("a".into(), "a".into(), "1".into()).into());
    assert_eq!(iter.next().unwrap(), &Op::Mul("a".into(), "a".into(), "2".into()).into());
    assert!(iter.next().is_none());

    assert_eq!(parse!(parser, "op x sin y 0;").unwrap()[0], Op::Sin("x".into(), "y".into()).into());
    assert_eq!(
        parse!(
            parser,
            "op res (op $ 1 + 2; op $ $ * 2;) / (x: op $ 2 * 3;);"
        ).unwrap()[0],
        Op::Div(
            "res".into(),
            DExp::new_nores(
                vec![
                    Op::Add(
                        Value::ResultHandle(None),
                        "1".into(),
                        "2".into()
                    ).into(),
                    Op::Mul(
                        Value::ResultHandle(None),
                        Value::ResultHandle(None),
                        "2".into()
                    ).into()
                ].into()).into(),
            DExp::new_optional_res(
                Some(IdxBox::new(36, "x".into())),
                vec![
                    Op::Mul(Value::ResultHandle(None), "2".into(), "3".into()).into()
                ].into(),
            ).into()
        ).into()
    );
    assert_eq!(
        parse!(
            parser,
            "op res (op $ 1 + 2; op $ $ * 2;) / (op $ 2 * 3;);"
        ).unwrap()[0],
        Op::Div(
            "res".into(),
            DExp::new_nores(
                vec![
                    Op::Add(
                        Value::ResultHandle(None),
                        "1".into(),
                        "2".into()
                    ).into(),
                    Op::Mul(
                        Value::ResultHandle(None),
                        Value::ResultHandle(None),
                        "2".into()
                    ).into()
                ].into()).into(),
            DExp::new_nores(
                vec![
                    Op::Mul(Value::ResultHandle(None), "2".into(), "3".into()).into()
                ].into(),
            ).into()
        ).into()
    );
}

#[test]
fn goto_test() {
    let parser = TopLevelParser::new();
    assert_eq!(parse!(parser, "goto :a 1 <= 2; :a").unwrap(), vec![
        Goto("a".into(), JumpCmp::LessThanEq("1".into(), "2".into()).into()).into(),
        LogicLine::Label("a".into()),
    ].into());
}

#[test]
fn control_test() {
    let parser = TopLevelParser::new();
    assert_eq!(
        parse!(parser, r#"skip 1 < 2 print "hello";"#).unwrap(),
        Expand(vec![
            Expand(vec![
                Goto("___0".into(), JumpCmp::LessThan("1".into(), "2".into()).into()).into(),
                LogicLine::Other(vec![Value::ReprVar("print".into()), r#""hello""#.into()].into()),
                LogicLine::Label("___0".into()),
            ]).into()
        ]).into()
    );

    check_sugar! {parser,
        r#"
        if 2 < 3 {
            print 1;
        } elif 3 < 4 {
            print 2;
        } elif 4 < 5 {
            print 3;
        } else print 4;
        "#,
        r#"
        {
            goto :___1 2 < 3;
            goto :___2 3 < 4;
            goto :___3 4 < 5;
            print 4;
            goto :___0 _;
            :___3 {
                print 3;
            }
            goto :___0 _;
            :___2 {
                print 2;
            }
            goto :___0 _;
            :___1 {
                print 1;
            }
            :___0
        }
        "#,
    };

    check_sugar! {parser,
        r#"
        if 2 < 3 {
            print 1;
        } elif 3 < 4 {
            print 2;
        } elif 4 < 5 {
            print 3;
        }
        "#,
        r#"
        {
            goto :___1 2 < 3;
            goto :___2 3 < 4;
            goto :___0 !4 < 5;
            {
                print 3;
            }
            goto :___0 _;
            :___2 {
                print 2;
            }
            goto :___0 _;
            :___1 {
                print 1;
            }
            :___0
        }
        "#,
    };

    check_sugar! {parser,
        r#"
        if 2 < 3 { # 对于没有elif与else的if, 会将条件反转并构建为skip
            print 1;
        }
        "#,
        r#"
        skip ! 2 < 3 {
            print 1;
        }
        "#,
    };

    check_sugar! {parser,
        r#"
        while a < b
            print 3;
        "#,
        r#"
        {
            goto :___0 a >= b;
            :___1
            print 3;
            goto :___1 a < b;
            :___0
        }
        "#,
    };

    check_sugar! {parser,
        r#"
        do {
            print 1;
        } while a < b;
        "#,
        r#"
        {
            :___0 {
                print 1;
            }
            goto :___0 a < b;
        }
        "#,
    };

    check_sugar! {parser,
        r#"
        gwhile a < b {
            print 1;
        }
        "#,
        r#"
        {
            goto :___0 _;
            :___1 {
                print 1;
            }
            :___0
            goto :___1 a < b;
        }
        "#,
    };

    check_sugar! {parser,
        r#"
        skip _ {
            print 1;
        }
        "#,
        r#"
        skip {
            print 1;
        }
        "#,
    };

    check_sugar! {parser,
        r#"
        do do {
            print 1;
        } while a < b; while c < d;
        "#,
        r#"
        {
            :___1 {
                :___0 {
                    print 1;
                }
                goto :___0 a < b;
            }
            goto :___1 c < d;
        }
        "#,
    };

    let _ = parse!(parser, r#"
    while a < b if c < d {
        print 1;
    } elif e < f {
        print 2;
    } else {
        print 3;
    }
    "#).unwrap();

    let _ = parse!(parser, r#"
    do if c < d {
        print 1;
    } elif e < f {
        print 2;
    } else {
        print 3;
    } while a < b;
    "#).unwrap();

    let _ = parse!(parser, r#"
    do print; while a < b;
    "#).unwrap_err();
}

#[test]
fn reverse_test() {
    let parser = TopLevelParser::new();

    let datas = vec![
        [r#"goto :a x === y;"#, r#"goto :a x !== y;"#],
        [r#"goto :a x == y;"#, r#"goto :a x != y;"#],
        [r#"goto :a x != y;"#, r#"goto :a x == y;"#],
        [r#"goto :a x < y;"#, r#"goto :a x >= y;"#],
        [r#"goto :a x > y;"#, r#"goto :a x <= y;"#],
        [r#"goto :a x <= y;"#, r#"goto :a x > y;"#],
        [r#"goto :a x >= y;"#, r#"goto :a x < y;"#],
        [r#"goto :a x;"#, r#"goto :a x == `false`;"#],
        [r#"goto :a _;"#, r#"goto :a !_;"#],
    ];
    for [src, dst] in datas {
        assert_eq!(
            parse!(parser, src).unwrap()[0].as_goto().unwrap().1.clone().reverse(),
            parse!(parser, dst).unwrap()[0].as_goto().unwrap().1,
        );
    }

    // 手动转换
    let datas = vec![
        [r#"goto :a ! x === y;"#, r#"goto :a x !== y;"#],
        [r#"goto :a ! x == y;"#, r#"goto :a x != y;"#],
        [r#"goto :a ! x != y;"#, r#"goto :a x == y;"#],
        [r#"goto :a ! x < y;"#, r#"goto :a x >= y;"#],
        [r#"goto :a ! x > y;"#, r#"goto :a x <= y;"#],
        [r#"goto :a ! x <= y;"#, r#"goto :a x > y;"#],
        [r#"goto :a ! x >= y;"#, r#"goto :a x < y;"#],
        [r#"goto :a ! x;"#, r#"goto :a x == `false`;"#],
        // 多次取反
        [r#"goto :a !!! x == y;"#, r#"goto :a x != y;"#],
        [r#"goto :a !!! x != y;"#, r#"goto :a x == y;"#],
        [r#"goto :a !!! x < y;"#, r#"goto :a x >= y;"#],
        [r#"goto :a !!! x > y;"#, r#"goto :a x <= y;"#],
        [r#"goto :a !!! x <= y;"#, r#"goto :a x > y;"#],
        [r#"goto :a !!! x >= y;"#, r#"goto :a x < y;"#],
        [r#"goto :a !!! x;"#, r#"goto :a x == `false`;"#],
        [r#"goto :a !!! x === y;"#, r#"goto :a x !== y;"#],
        [r#"goto :a !!! _;"#, r#"goto :a !_;"#],
    ];
    for [src, dst] in datas {
        check_sugar! {parser,
            src,
            dst,
        };
    }
}

#[test]
fn goto_compile_test() {
    let parser = TopLevelParser::new();

    check_compile!{parser,
        r#"
        goto :x _;
        :x
        end;
        "#,
        r#"
        jump 1 always 0 0
        end
        "#,
    };

    check_compile!{parser,
        r#"
        const 0 = 1;
        goto :x _;
        :x
        end;
        "#,
        r#"
               jump 1 always 0 0
               end
        "#
    };

    check_compile!{parser,
        r#"
        const 0 = 1;
        goto :x !_;
        :x
        end;
        "#,
        r#"
               end
        "#
    };

    check_compile!{parser,
        r#"
        const 0 = 1;
        const false = true;
        goto :x a === b;
        :x
        end;
        "#,
        r#"
               jump 1 strictEqual a b
               end
        "#
    };

    check_compile!{parser,
        r#"
        const 0 = 1;
        const false = true;
        goto :x !!a === b;
        :x
        end;
        "#,
        r#"
               jump 1 strictEqual a b
               end
        "#
    };

    check_compile!{parser,
        r#"
        const 0 = 1;
        const false = true;
        goto :x !a === b;
        :x
        end;
        "#,
        r#"
               op strictEqual __0 a b
               jump 2 equal __0 false
               end
        "#
    };

    check_compile!{parser,
        r#"
        const 0 = 1;
        const false = true;
        goto :x a !== b;
        :x
        end;
        "#,
        r#"
               op strictEqual __0 a b
               jump 2 equal __0 false
               end
        "#
    };

    check_compile!{parser,
        r#"
        skip !_ && !_ {
            print true;
        }
        end;
        "#,
        r#"
               jump 1 always 0 0
               print true
               end
        "#
    };

    check_compile!{parser,
        r#"
        skip _ && !_ {
            print true;
        }
        end;
        "#,
        r#"
               print true
               end
        "#
    };

    check_compile!{parser,
        r#"
        skip !_ {
            print true;
        }
        end;
        "#,
        r#"
               print true
               end
        "#
    };
}

#[test]
fn line_test() {
    let parser = TopLevelParser::new();
    assert_eq!(
        parse!(parser, "noop;").unwrap(),
        Expand(vec![
            LogicLine::NoOp,
        ]).into()
    );
    assert_eq!(
        parse!(parser, "foo;").unwrap(),
        Expand(vec![
            LogicLine::Other(vec!["foo".into()].into()),
        ]).into()
    );
    assert_eq!(
        parse!(parser, "foo bar;").unwrap(),
        Expand(vec![
            LogicLine::Other(vec!["foo".into(), "bar".into()].into()),
        ]).into()
    );
    assert_eq!(
        parse!(parser, "foo, bar;").unwrap(),
        Expand(vec![
            LogicLine::Other(vec!["foo".into(), "bar".into()].into()),
        ]).into()
    );
}

#[test]
fn literal_uint_test() {
    let parser = LiteralUIntParser::new();
    assert!(parse!(parser, "1.5").is_err());

    assert_eq!(parse!(parser, "23").unwrap(), 23);
    assert_eq!(parse!(parser, "0x1b").unwrap(), 0x1b);
    assert_eq!(parse!(parser, "0b10_1001").unwrap(), 0b10_1001);
}

#[test]
fn switch_test() {
    let parser = TopLevelParser::new();

    let ast = parse!(parser, r#"
        switch 2 {
        case 1:
            print 1;
        case 2 4:
            print 2;
            print 4;
        case 5:
            :a
            :b
            print 5;
        }
    "#).unwrap();
    assert_eq!(
        ast,
        Expand(vec![Select(
            "2".into(),
            Expand(vec![
                LogicLine::Ignore,
                Expand(vec![LogicLine::Other(vec![Value::ReprVar("print".into()), "1".into()].into())]).into(),
                Expand(vec![
                    LogicLine::Other(vec![Value::ReprVar("print".into()), "2".into()].into()),
                    LogicLine::Other(vec![Value::ReprVar("print".into()), "4".into()].into()),
                ]).into(),
                LogicLine::Ignore,
                Expand(vec![
                    LogicLine::Other(vec![Value::ReprVar("print".into()), "2".into()].into()),
                    LogicLine::Other(vec![Value::ReprVar("print".into()), "4".into()].into()),
                ]).into(),
                Expand(vec![
                    LogicLine::Label("a".into()),
                    LogicLine::Label("b".into()),
                    LogicLine::Other(vec![Value::ReprVar("print".into()), "5".into()].into()),
                ]).into(),
            ])
        ).into()])
    );
    let tag_codes = CompileMeta::new()
        .compile(ast);
    let lines = tag_codes
        .compile()
        .unwrap();
    assert_eq!(lines, [
        "op mul __0 2 2",
        "op add @counter @counter __0",
        "jump 4 always 0 0",
        "noop",
        "print 1",
        "jump 6 always 0 0",
        "print 2",
        "print 4",
        "jump 10 always 0 0",
        "noop",
        "print 2",
        "print 4",
        "print 5",
    ]);

    let ast = parse!(parser, r#"
        switch 1 {
        print end;
        case 0: print 0;
        case 1: print 1;
        }
    "#).unwrap();
    assert_eq!(
        ast,
        Expand(vec![Select(
            "1".into(),
            Expand(vec![
                Expand(vec![
                        LogicLine::Other(vec![Value::ReprVar("print".into()), "0".into()].into()),
                        LogicLine::Other(vec![Value::ReprVar("print".into()), "end".into()].into()),
                ]).into(),
                Expand(vec![
                        LogicLine::Other(vec![Value::ReprVar("print".into()), "1".into()].into()),
                        LogicLine::Other(vec![Value::ReprVar("print".into()), "end".into()].into()),
                ]).into(),
            ])
        ).into()])
    );

    // 测试追加对于填充的效用
    let ast = parse!(parser, r#"
        switch 1 {
        print end;
        case 1: print 1;
        }
    "#).unwrap();
    assert_eq!(
        ast,
        Expand(vec![Select(
            "1".into(),
            Expand(vec![
                Expand(vec![
                        LogicLine::Other(vec![Value::ReprVar("print".into()), "end".into()].into()),
                ]).into(),
                Expand(vec![
                        LogicLine::Other(vec![Value::ReprVar("print".into()), "1".into()].into()),
                        LogicLine::Other(vec![Value::ReprVar("print".into()), "end".into()].into()),
                ]).into(),
            ])
        ).into()])
    );

    assert_eq!(
        parse!(parser, r#"
            switch 1 {
            print end;
            print end1;
            case 0: print 0;
            }
        "#).unwrap(),
        Expand(vec![Select(
            "1".into(),
            Expand(vec![
                Expand(vec![
                        LogicLine::Other(vec![Value::ReprVar("print".into()), "0".into()].into()),
                        Expand(vec![
                            LogicLine::Other(vec![Value::ReprVar("print".into()), "end".into()].into()),
                            LogicLine::Other(vec![Value::ReprVar("print".into()), "end1".into()].into()),
                        ]).into(),
                ]).into(),
            ])
        ).into()])
    );

}

#[test]
fn comments_test() {
    let parser = TopLevelParser::new();
    check_sugar! {parser,
        r#"
        # inline comment
        #comment1
        #* this is a long comments
         * ...
         * gogogo
         *#
        #***x*s;;@****\*\*#
        #*##xs*** #** *#
        #*r*#
        #
        #*一行内的长注释*#
        #*语句前面的长注释*#noop;#语句后注释
        #注释
        "#,
        r#"
        noop;
        "#,
    };
}

#[test]
fn op_generate_test() {
    assert_eq!(
        Op::Add("x".into(), "y".into(), "z".into()).generate_args(&mut Default::default()),
        args!["op", "add", "x", "y", "z"],
    );
    assert_eq!(
        Op::AngleDiff("x".into(), "y".into(), "z".into()).generate_args(&mut Default::default()),
        args!["op", "angleDiff", "x", "y", "z"],
    );
}

#[test]
fn compile_test() {
    let parser = TopLevelParser::new();

    check_compile!{parser,
        r#"
        op x 1 + 2;
        op y (op $ x + 3;) * (op $ x * 2;);
        if (op tmp y & 1; op $ tmp + 1;) == 1 {
            print "a ";
        } else {
            print "b ";
        }
        print (op $ y + 3;);
        "#,
        r#"
        op add x 1 2
        op add __0 x 3
        op mul __1 x 2
        op mul y __0 __1
        op and tmp y 1
        op add __2 tmp 1
        jump 9 equal __2 1
        print "b "
        jump 10 always 0 0
        print "a "
        op add __3 y 3
        print __3
        "#
    };
}

#[test]
fn compile_take_test() {
    let parser = TopLevelParser::new();
    let ast = parse!(parser, "op x ({}op $ 1 + 2;) + 3;").unwrap();
    let mut meta = CompileMeta::new();
    meta.push(ParseLine::Args(args!("noop")));
    assert_eq!(
        ast.compile_take(&mut meta),
        vec![
            ParseLine::Args(args!("op", "add", "__0", "1", "2")),
            ParseLine::Args(args!("op", "add", "x", "__0", "3")),
        ]
    );
    assert_eq!(meta.parse_lines().len(), 1);
    assert_eq!(meta.parse_lines().lines(), &vec![ParseLine::Args(args!("noop"))]);
}

#[test]
fn const_value_test() {
    let parser = TopLevelParser::new();

    check_compile!{parser,
        r#"
        x = C;
        const C = (read $ cell1 0;);
        y = C;
        "#,
        r#"
        set x C
        read __0 cell1 0
        set y __0
        "#
    };

    check_compile!{parser,
        r#"
        x = C;
        const C = (k: read k cell1 0;);
        y = C;
        "#,
        r#"
        set x C
        read k cell1 0
        set y k
        "#
    };

    check_compile!{parser,
        r#"
        x = C;
        const C = (read $ cell1 0;);
        foo a b C d C;
        "#,
        r#"
        set x C
        read __0 cell1 0
        read __1 cell1 0
        foo a b __0 d __1
        "#
    };

    check_compile!{parser,
        r#"
        const C = (m: read $ cell1 0;);
        x = C;
        "#,
        r#"
        read m cell1 0
        set x m
        "#
    };

    check_compile!{parser,
        r#"
        const C = (read $ cell1 (i: read $ cell2 0;););
        print C;
        "#,
        r#"
        read i cell2 0
        read __0 cell1 i
        print __0
        "#
    };
}

#[test]
fn const_value_block_range_test() {
    let parser = TopLevelParser::new();

    check_compile!{parser,
        r#"
        {
            x = C;
            const C = (read $ cell1 0;);
            const C = (read $ cell2 0;); # 常量覆盖
            {
                const C = (read $ cell3 0;); # 子块常量
                m = C;
            }
            y = C;
            foo C C;
        }
        z = C;
        "#,
        r#"
        set x C
        read __0 cell3 0
        set m __0
        read __1 cell2 0
        set y __1
        read __2 cell2 0
        read __3 cell2 0
        foo __2 __3
        set z C
        "#
    };
}

#[test]
fn take_test() {
    let parser = TopLevelParser::new();

    check_compile!{parser,
        r#"
        print start;
        const F = (read $ cell1 0;);
        take V = F; # 求值并映射
        print V;
        print V; # 再来一次
        foo V V;
        take V1 = F; # 再求值并映射
        print V1;
        "#,
        r#"
        print start
        read __0 cell1 0
        print __0
        print __0
        foo __0 __0
        read __1 cell1 0
        print __1
        "#
    };

    check_compile!{parser,
        r#"
        const F = (m: read $ cell1 0;);
        take V = F; # 求值并映射
        print V;
        "#,
        r#"
        read m cell1 0
        print m
        "#
    };

    check_compile!{parser,
        r#"
        take X = 2;
        take Y = `X`;
        const Z = `X`;
        print Y Z;
        "#,
        r#"
        print X
        print X
        "#
    };

    check_compile!{parser,
        r#"
        take X = 2;
        take Y = X;
        const Z = X;
        print Y Z;
        "#,
        r#"
        print 2
        print 2
        "#
    };

    check_compile!{parser,
        r#"
        const 2 = 3;
        take X = `2`;
        take Y = X;
        const Z = X;
        print Y Z;
        "#,
        r#"
        print 2
        print 2
        "#
    };

    check_sugar! {parser,
        r#"
        take+A+B+C+D;
        "#,
        r#"
        inline {
            take A = ();
            take B = ();
            take C = ();
            take D = ();
        }
        "#,
    };

    check_sugar! {parser,
        r#"
        take+A, +B, +C+D,;
        "#,
        r#"
        inline {
            take A = ();
            take B = ();
            take C = ();
            take D = ();
        }
        "#,
    };

    check_sugar! {parser,
        r#"
        take +A, B, C,;
        "#,
        r#"
        take +A  B  C;
        "#,
    };

    check_sugar! {parser,
        r#"
        take +A, B, C;
        "#,
        r#"
        take +A  B  C;
        "#,
    };

    check_sugar! {parser,
        r#"
        const A=2, B=3,;
        "#,
        r#"
        const A=2 B=3;
        "#,
    };

    check_sugar! {parser,
        r#"
        const A=2, B=3;
        "#,
        r#"
        const A=2 B=3;
        "#,
    };

    check_sugar! {parser,
        r#"
        take X{A, B:C} = (c;);
        "#,
        r#"
        inline {
            take X = (c;);
            take A = X.A;
            take B = X.C;
        }
        "#,
    };

    check_sugar! {parser,
        r#"
        take M=() X{A, B:C} = (c;);
        "#,
        r#"
        inline {
            take M=();
            inline {
                take X = (c;);
                take A = X.A;
                take B = X.C;
            }
        }
        "#,
    };

    check_sugar! {parser,
        r#"
        take X{A, B:C} = (c;);
        "#,
        r#"
        inline {
            take X = (c;);
            take A = X.A;
            take B = X.C;
        }
        "#,
    };

    check_sugar! {parser,
        r#"
        take X{A B:C} = (c;);
        "#,
        r#"
        inline {
            take X = (c;);
            take A = X.A;
            take B = X.C;
        }
        "#,
    };

    check_sugar! {parser,
        r#"
        take X{A &B:C} = (c;);
        "#,
        r#"
        inline {
            take X = (c;);
            take A = X.A;
            const B = X->C;
        }
        "#,
    };

    check_sugar! {parser,
        r#"
        take _{A &B:C} = (c;);
        "#,
        r#"
        inline {
            take ___0 = (c;);
            take A = ___0.A;
            const B = ___0->C;
        }
        "#,
    };

    check_sugar! {parser,
        r#"
        take {A &B:C} = (c;);
        "#,
        r#"
        inline {
            take ___0 = (c;);
            take A = ___0.A;
            const B = ___0->C;
        }
        "#,
    };

    check_sugar! {parser,
        r#"
        take {A &B:C} = (c;) X=M;
        "#,
        r#"
        inline {
            inline {
                take ___0 = (c;);
                take A = ___0.A;
                const B = ___0->C;
            }
            take X=M;
        }
        "#,
    };

    check_sugar! {parser,
        r#"
        take {A &B:C} = (c;) {X}=M;
        "#,
        r#"
        inline {
            inline {
                take ___0 = (c;);
                take A = ___0.A;
                const B = ___0->C;
            }
            inline {
                take ___1=M;
                take X = ___1.X;
            }
        }
        "#,
    };

    check_sugar! {parser,
        r#"
        take {A &B:C} = (c;) {X}=M {Y}=N;
        "#,
        r#"
        inline {
            inline {
                take ___0 = (c;);
                take A = ___0.A;
                const B = ___0->C;
            }
            inline {
                take ___1=M;
                take X = ___1.X;
            }
            inline {
                take ___2=N;
                take Y = ___2.Y;
            }
        }
        "#,
    };

    check_sugar! {parser,
        r#"
        take;
        "#,
        r#"
        inline {}
        "#,
    };
}

#[test]
fn print_test() {
    let parser = TopLevelParser::new();

    check_compile!{parser,
        r#"
        print "abc" "def" "ghi" j 123 @counter;
        "#,
        r#"
        print "abc"
        print "def"
        print "ghi"
        print j
        print 123
        print @counter
        "#
    };

}

#[test]
fn in_const_label_test() {
    let parser = TopLevelParser::new();
    let mut meta = Meta::new();
    let ast = parser.parse(&mut meta, r#"
    :start
    const X = (
        :in_const
        print "hi";
    );
    "#).unwrap();
    let mut iter = ast.0.into_iter();
    assert_eq!(iter.next().unwrap(), LogicLine::Label("start".into()));
    assert_eq!(
        iter.next().unwrap(),
        Const(
            "X".into(),
            DExp::new_nores(
                vec![
                    LogicLine::Label("in_const".into()),
                    LogicLine::Other(vec![Value::ReprVar("print".into()), "\"hi\"".into()].into())
                ].into()
            ).into(),
            vec!["in_const".into()]
        ).into()
    );
    assert_eq!(iter.next(), None);
}

#[test]
fn const_expand_label_rename_test() {
    let parser = TopLevelParser::new();

    check_compile!{parser,
        r#"
        :start
        const X = (
            if num < 2 {
                print "num < 2";
            } else
                print "num >= 2";
            goto :start _;
        );
        take __ = X;
        take __ = X;
        "#,
        r#"
            jump 3 lessThan num 2
            print "num >= 2"
            jump 0 always 0 0
            print "num < 2"
            jump 0 always 0 0
            jump 8 lessThan num 2
            print "num >= 2"
            jump 0 always 0 0
            print "num < 2"
            jump 0 always 0 0
        "#
    };

    check_compile!{parser,
        r#"
        # 这里是__0以此类推, 所以接下来的使用C的句柄为__2, 测试数据解释
        const A = (
            const B = (
                i = C;
                goto :next _; # 测试往外跳
            );
            const C = ({}op $ 1 + 1;);
            take __ = B;
            print "skiped";
            :next
            do {
                print "in a";
                op i i + 1;
            } while i < 5;
        );
        take __ = A;
        "#,
        r#"
            op add __2 1 1
            set i __2
            jump 4 always 0 0
            print "skiped"
            print "in a"
            op add i i 1
            jump 4 lessThan i 5
        "#
    };
}

#[test]
fn dexp_result_handle_use_const_test() {
    let parser = TopLevelParser::new();

    check_compile!{parser,
        r#"
        {
            print (R: $ = 2;);
            const R = x;
            print (R: $ = 2;);
        }
        print (R: $ = 2;);
        "#,
        r#"
        set R 2
        print R
        set x 2
        print x
        set R 2
        print R
        "#
    };
}

#[test]
fn dexp_result_handle_use_result_handle_test() {
    let parser = TopLevelParser::new();

    check_compile!{parser,
        r#"
        {
            print (R: $ = 2;);
            const R = $;
            print (R: $ = 2;);
        }
        print (R: $ = 2;);
        "#,
        r#"
        set R 2
        print R
        set __0 2
        print __0
        set R 2
        print R
        "#
    };
}

#[test]
fn in_const_const_label_rename_test() {
    let parser = TopLevelParser::new();

    let ast = parse!(parser, r#"
    const X = (
        const X = (
            i = 0;
            do {
                op i i + 1;
            } while i < 10;
            if a {
                a1;
            } elif b {
                b1;
            } elif c {
                c1;
            } else {
                d1;
            }
            if a {
                a1;
            } elif b {
                b1;
            } elif c {
                c1;
            }
            if a {
                a1;
            } else {
                d1;
            }
        );
        take __ = X;
        take __ = X;
    );
    take __ = X;
    take __ = X;
    "#).unwrap();
    let meta = CompileMeta::new();
    let tag_codes = meta.compile(ast);
    let _logic_lines = tag_codes.compile().unwrap();

    let ast = parse!(parser, r#"
    const Duplicate = (
        take _0 _0;
    );
    const C = (
        x = if c ? 1 : 2;
        y = 1+(if c ? 3 : 4);
    );
    Duplicate! C;
    Duplicate! C;
    "#).unwrap();
    let meta = CompileMeta::new();
    let tag_codes = meta.compile(ast);
    let _logic_lines = tag_codes.compile().unwrap();
}

#[test]
fn take_default_result_test() {
    let parser = TopLevelParser::new();

    let ast = parse!(parser, "take 2;").unwrap();
    assert_eq!(ast, Expand(vec![
        Take(ConstKey::Unused(IdxBox::new(5, "__".into())), "2".into()).into()
    ]));
}

#[test]
fn const_value_leak_test() {
    let ast: Expand = vec![
        Expand(vec![
            LogicLine::Other(vec!["print".into(), "N".into()].into()),
            Const("N".into(), "2".into(), Vec::new()).into(),
            LogicLine::Other(vec!["print".into(), "N".into()].into()),
        ]).into(),
        LogicLine::Other(vec!["print".into(), "N".into()].into()),
    ].into();
    let meta = CompileMeta::new();
    let tag_codes = meta.compile(ast);
    let logic_lines = tag_codes.compile().unwrap();
    assert_eq!(logic_lines, vec![
               "print N",
               "print 2",
               "print N",
    ]);

    let ast: Expand = vec![
        Expand(vec![
            LogicLine::Other(vec!["print".into(), "N".into()].into()),
            Const("N".into(), "2".into(), Vec::new()).into(),
            LogicLine::Other(vec!["print".into(), "N".into()].into()),
            LogicLine::ConstLeak("N".into()),
        ]).into(),
        LogicLine::Other(vec!["print".into(), "N".into()].into()),
    ].into();
    let meta = CompileMeta::new();
    let tag_codes = meta.compile(ast);
    let logic_lines = tag_codes.compile().unwrap();
    assert_eq!(logic_lines, vec![
               "print N",
               "print 2",
               "print 2",
    ]);
}

#[test]
fn take2_test() {
    let parser = TopLevelParser::new();

    let ast = parse!(parser, "take X;").unwrap();
    assert_eq!(ast, Expand(vec![
        Take(ConstKey::Unused(IdxBox::new(5, "__".into())), "X".into()).into()
    ]));

    let ast = parse!(parser, "take R = X;").unwrap();
    assert_eq!(ast, Expand(vec![
        Take("R".into(), "X".into()).into()
    ]));

    let ast = parse!(parser, "take[] X;").unwrap();
    assert_eq!(ast, Expand(vec![
        Take(ConstKey::Unused(IdxBox::new(7, "__".into())), "X".into()).into()
    ]));

    let ast = parse!(parser, "take[] R = X;").unwrap();
    assert_eq!(ast, Expand(vec![
        Take("R".into(), "X".into()).into()
    ]));

    let ast = parse!(parser, "take[1 2] R = X;").unwrap();
    assert_eq!(ast, Expand(vec![
        Expand(vec![
                LogicLine::SetArgs(vec!["1".into(), "2".into()].into()),
                Take("R".into(), "X".into()).into(),
                LogicLine::ConstLeak("R".into()),
        ]).into()
    ]));

    let ast = parse!(parser, "take[1 2] X;").unwrap();
    assert_eq!(ast, Expand(vec![
        Expand(vec![
                LogicLine::SetArgs(vec!["1".into(), "2".into()].into()),
                Take(ConstKey::Unused(IdxBox::new(10, "__".into())), "X".into()).into(),
        ]).into()
    ]));
}

#[test]
fn take_args_test() {
    let parser = TopLevelParser::new();

    check_compile!{parser,
        r#"
        const M = (
            print _0 _1 _2;
            set $ 3;
        );
        take[1 2 3] M;
        take[4 5 6] R = M;
        print R;
        "#,
        r#"
        print 1
        print 2
        print 3
        set __3 3
        print 4
        print 5
        print 6
        set __7 3
        print __7
        "#
    };

    check_compile!{parser,
        r#"
        const DO = (
            print _0 "start";
            take _1;
            print _0 "start*2";
            take _1;
            printflush message1;
        );
        # 这里赋给一个常量再使用, 因为直接使用不会记录label, 无法重复被使用
        # 而DO中, 会使用两次传入的参数1
        const F = (
            i = 0;
            while i < 10 {
                print i;
                op i i + 1;
            }
        );
        take["loop" F] DO;
        "#,
        r#"
        print "loop"
        print "start"
        set i 0
        jump 7 greaterThanEq i 10
        print i
        op add i i 1
        jump 4 lessThan i 10
        print "loop"
        print "start*2"
        set i 0
        jump 14 greaterThanEq i 10
        print i
        op add i i 1
        jump 11 lessThan i 10
        printflush message1
        "#
    };

    check_compile!{parser,
        r#"
        const F = (y:print _0 $;);
        take (x:
            F! *$;
        );
        "#,
        r#"
        print x
        print y
        "#
    };
}

#[test]
fn const_value_clone_test() {
    let parser = TopLevelParser::new();

    check_compile!{parser,
        r#"
        const A = 1;
        const B = A;
        const A = 2;
        print A B;
        "#,
        r#"
               print 2
               print 1
        "#
    };

    check_compile!{parser,
        r#"
        const A = 1;
        const B = A;
        const A = 2;
        const C = B;
        const B = 3;
        const B = B;
        print A B C;
        "#,
        r#"
               print 2
               print 3
               print 1
        "#
    };

    check_compile!{parser,
        r#"
        const A = B;
        const B = 2;
        print A;
        "#,
        r#"
               print B
        "#
    };

    check_compile!{parser,
        r#"
        const A = B;
        const B = 2;
        const A = A;
        print A;
        "#,
        r#"
               print B
        "#
    };

    check_compile!{parser,
        r#"
        const A = B;
        const B = 2;
        {
            const A = A;
            print A;
        }
        "#,
        r#"
               print B
        "#
    };

    check_compile!{parser,
        r#"
        const A = B;
        {
            const B = 2;
            const A = A;
            print A;
        }
        "#,
        r#"
               print B
        "#
    };

    check_compile!{parser,
        r#"
        const A = B;
        {
            const B = 2;
            print A;
        }
        "#,
        r#"
               print B
        "#
    };

    check_compile!{parser,
        r#"
        const A = B;
        const B = C;
        const C = A;
        print C;
        "#,
        r#"
               print B
        "#
    };

    check_compile!{parser,
        r#"
        const A = C;
        const C = 2;
        const B = A;
        const A = 3;
        const C = B;
        print C;
        "#,
        r#"
               print C
        "#
    };
}

#[test]
fn cmptree_test() {
    let parser = TopLevelParser::new();

    let ast = parse!(parser, r#"
    goto :end a && b && c;
    foo;
    :end
    end;
    "#).unwrap();
    assert_eq!(
        ast[0].as_goto().unwrap().1,
        CmpTree::And(
            CmpTree::And(
                Box::new(JumpCmp::NotEqual("a".into(), Value::ReprVar(FALSE_VAR.into()).into()).into()),
                Box::new(JumpCmp::NotEqual("b".into(), Value::ReprVar(FALSE_VAR.into()).into()).into()),
            ).into(),
            Box::new(JumpCmp::NotEqual("c".into(), Value::ReprVar(FALSE_VAR.into()).into()).into()),
        ).into()
    );

    let ast = parse!(parser, r#"
    goto :end a || b || c;
    foo;
    :end
    end;
    "#).unwrap();
    assert_eq!(
        ast[0].as_goto().unwrap().1,
        CmpTree::Or(
            CmpTree::Or(
                Box::new(JumpCmp::NotEqual("a".into(), Value::ReprVar(FALSE_VAR.into()).into()).into()),
                Box::new(JumpCmp::NotEqual("b".into(), Value::ReprVar(FALSE_VAR.into()).into()).into()),
            ).into(),
            Box::new(JumpCmp::NotEqual("c".into(), Value::ReprVar(FALSE_VAR.into()).into()).into()),
        ).into()
    );

    let ast = parse!(parser, r#"
    goto :end a && b || c && d;
    foo;
    :end
    end;
    "#).unwrap();
    assert_eq!(
        ast[0].as_goto().unwrap().1,
        CmpTree::Or(
            CmpTree::And(
                Box::new(JumpCmp::NotEqual("a".into(), Value::ReprVar(FALSE_VAR.into()).into()).into()),
                Box::new(JumpCmp::NotEqual("b".into(), Value::ReprVar(FALSE_VAR.into()).into()).into()),
            ).into(),
            CmpTree::And(
                Box::new(JumpCmp::NotEqual("c".into(), Value::ReprVar(FALSE_VAR.into()).into()).into()),
                Box::new(JumpCmp::NotEqual("d".into(), Value::ReprVar(FALSE_VAR.into()).into()).into()),
            ).into(),
        ).into()
    );

    let ast = parse!(parser, r#"
    goto :end a && (b || c) && d;
    foo;
    :end
    end;
    "#).unwrap();
    assert_eq!(
        ast[0].as_goto().unwrap().1,
        CmpTree::And(
            CmpTree::And(
                Box::new(JumpCmp::NotEqual("a".into(), Value::ReprVar(FALSE_VAR.into()).into()).into()),
                CmpTree::Or(
                    Box::new(JumpCmp::NotEqual("b".into(), Value::ReprVar(FALSE_VAR.into()).into()).into()),
                    Box::new(JumpCmp::NotEqual("c".into(), Value::ReprVar(FALSE_VAR.into()).into()).into()),
                ).into(),
            ).into(),
            Box::new(JumpCmp::NotEqual("d".into(), Value::ReprVar(FALSE_VAR.into()).into()).into()),
        ).into()
    );

    check_compile!{parser,
        r#"
        goto :end a && b;
        foo;
        :end
        end;
        "#,
        r#"
               jump 2 equal a false
               jump 3 notEqual b false
               foo
               end
        "#
    };

    check_compile!{parser,
        r#"
        goto :end (a || b) && c;
        foo;
        :end
        end;
        "#,
        r#"
               jump 2 notEqual a false
               jump 3 equal b false
               jump 4 notEqual c false
               foo
               end
        "#
    };

    check_compile!{parser,
        r#"
        goto :end (a || b) && (c || d);
        foo;
        :end
        end;
        "#,
        r#"
               jump 2 notEqual a false
               jump 4 equal b false
               jump 5 notEqual c false
               jump 5 notEqual d false
               foo
               end
        "#
    };

    check_compile!{parser,
        r#"
        goto :end a || b || c || d || e;
        foo;
        :end
        end;
        "#,
        r#"
               jump 6 notEqual a false
               jump 6 notEqual b false
               jump 6 notEqual c false
               jump 6 notEqual d false
               jump 6 notEqual e false
               foo
               end
        "#
    };

    check_compile!{parser,
        r#"
        goto :end a && b && c && d && e;
        foo;
        :end
        end;
        "#,
        r#"
               jump 5 equal a false
               jump 5 equal b false
               jump 5 equal c false
               jump 5 equal d false
               jump 6 notEqual e false
               foo
               end
        "#
    };

    check_compile!{parser,
        r#"
        goto :end (a && b && c) && d && e;
        foo;
        :end
        end;
        "#,
        r#"
               jump 5 equal a false
               jump 5 equal b false
               jump 5 equal c false
               jump 5 equal d false
               jump 6 notEqual e false
               foo
               end
        "#
    };

    check_compile!{parser,
        r#"
        goto :end a && b && (c && d && e);
        foo;
        :end
        end;
        "#,
        r#"
               jump 5 equal a false
               jump 5 equal b false
               jump 5 equal c false
               jump 5 equal d false
               jump 6 notEqual e false
               foo
               end
        "#
    };

    check_compile!{parser,
        r#"
        goto :end a && (op $ b && c;);
        foo;
        :end
        end;
        "#,
        r#"
               jump 3 equal a false
               op land __0 b c
               jump 4 notEqual __0 false
               foo
               end
        "#
    };

    check_compile!{parser,
        r#"
        goto :end a && b || c && d;
        foo;
        :end
        end;
        "#,
        r#"
               jump 2 equal a false
               jump 5 notEqual b false
               jump 4 equal c false
               jump 5 notEqual d false
               foo
               end
        "#
    };

    check_compile!{parser,
        r#"
        goto :end !a && b || c && d;
        foo;
        :end
        end;
        "#,
        r#"
               jump 2 notEqual a false
               jump 5 notEqual b false
               jump 4 equal c false
               jump 5 notEqual d false
               foo
               end
        "#
    };

    check_compile!{parser,
        r#"
        goto :end (a && b) || !(c && d);
        foo;
        :end
        end;
        "#,
        r#"
               jump 2 equal a false
               jump 5 notEqual b false
               jump 5 equal c false
               jump 5 equal d false
               foo
               end
        "#
    };

    check_compile!{parser,
        r#"
        goto :end (a && b && c) || (d && e);
        foo;
        :end
        end;
        "#,
        r#"
               jump 3 equal a false
               jump 3 equal b false
               jump 6 notEqual c false
               jump 5 equal d false
               jump 6 notEqual e false
               foo
               end
        "#
    };

    check_compile!{parser,
        r#"
        goto :end (a && b || c) || (d && e);
        foo;
        :end
        end;
        "#,
        r#"
               jump 2 equal a false
               jump 6 notEqual b false
               jump 6 notEqual c false
               jump 5 equal d false
               jump 6 notEqual e false
               foo
               end
        "#
    };

    check_compile!{parser,
        r#"
        goto :end ((a && b) || c) || (d && e);
        foo;
        :end
        end;
        "#,
        r#"
               jump 2 equal a false
               jump 6 notEqual b false
               jump 6 notEqual c false
               jump 5 equal d false
               jump 6 notEqual e false
               foo
               end
        "#
    };

    check_compile!{parser,
        r#"
        goto :end (a && (b || c)) || (d && e);
        foo;
        :end
        end;
        "#,
        r#"
               jump 3 equal a false
               jump 6 notEqual b false
               jump 6 notEqual c false
               jump 5 equal d false
               jump 6 notEqual e false
               foo
               end
        "#
    };

    check_compile!{parser,
        r#"
        goto :end (op $ a + 2;) && (op $ b + 2;);
        foo;
        :end
        end;
        "#,
        r#"
               op add __0 a 2
               jump 4 equal __0 false
               op add __1 b 2
               jump 5 notEqual __1 false
               foo
               end
        "#
    };

    check_sugar! {parser,
        r#"
        :x
        goto :x a < b and c > d or not e != f;
        "#,
        r#"
        :x
        goto :x a < b && c > d || ! e != f;
        "#,
    };

    check_sugar! {parser,
        r#"
        :x
        goto :x ++a < b and c > d or not e != f;
        "#,
        r#"
        :x
        goto :x (`__`:setres a;$=$+`1`) < b && c > d || ! e != f;
        "#,
    };

    check_sugar! {parser,
        r#"
        :x
        goto :x --a < b and c > d or not e != f;
        "#,
        r#"
        :x
        goto :x (`__`:setres a;$=$-`1`) < b && c > d || ! e != f;
        "#,
    };

}

#[test]
fn set_res_test() {
    let parser = TopLevelParser::new();

    check_compile!{parser,
        r#"
        print (setres (x: op $ 1 + 2;););
        "#,
        r#"
               op add x 1 2
               print x
        "#
    };

    check_compile!{parser,
        r#"
        print (setres m;);
        "#,
        r#"
               print m
        "#
    };
}

#[test]
fn repr_var_test() {
    let parser = TopLevelParser::new();

    check_compile!{parser,
        r#"
        print a;
        print `a`;
        const a = b;
        print a;
        print `a`;
        print `print`;
        print `op`;
        print `_`;
        print len;
        "#,
        r#"
               print a
               print a
               print b
               print a
               print print
               print op
               print _
               print len
        "#
    };
}

#[test]
fn select_test() {
    let parser = TopLevelParser::new();

    check_compile!{parser,
        r#"
        select 1 {
            print 0;
            print;
        }
        "#,
        r#"
        op add @counter @counter 1
        print 0
        "#
    };

    check_compile!{parser,
        r#"
        select 1 {
            print 0;
            print;
            print;
        }
        "#,
        r#"
        op add @counter @counter 1
        print 0
        jump 0 always 0 0
        "#
    };

    check_compile!{parser,
        r#"
        select 1 {
            print 0;
            print;
            print 2;
        }
        "#,
        r#"
        op add @counter @counter 1
        print 0
        jump 3 always 0 0
        print 2
        "#
    };

    check_compile!{parser,
        r#"
        select 1 {
            print 0;
            print 1 " is one!";
            print 2;
        }
        "#,
        r#"
        op mul __0 1 2
        op add @counter @counter __0
        print 0
        jump 4 always 0 0
        print 1
        print " is one!"
        print 2
        "#
    };

    check_compile!{parser,
        r#"
        select x {
            print 0;
            print 1;
            print 2;
        }
        "#,
        r#"
        op add @counter @counter x
        print 0
        print 1
        print 2
        "#
    };

    check_compile!{parser,
        r#"
        select (y: op $ x + 2;) {}
        "#,
        r#"
        op add y x 2
        "#
    };

    check_compile!{parser,
        r#"
        select x {}
        "#,
        r#"
        "#
    };

    check_compile!{parser,
        r#"
        select m {
            print 0;
            print 1 " is one!" ", one!!" "\n";
            print 2;
        }
        "#,
        r#"
        op add @counter @counter m
        jump 4 always 0 0
        jump 5 always 0 0
        jump 9 always 0 0
        print 0
        print 1
        print " is one!"
        print ", one!!"
        print "\n"
        print 2
        "#
    };
}

#[test]
fn switch_catch_test() {
    let parser = TopLevelParser::new();

    check_compile_eq!(parser,
        r#"
        switch (op $ x + 2;) {
            end;
        case <:
            print "Underflow";
            stop;
        case ! e:
            print "Misses: " e;
            stop;
        case > n:
            print "Overflow: " n;
            stop;
        case 1:
            print 1;
        case 3:
            print 3 "!";
        }
        "#,
        r#"
        take tmp = (op $ x + 2;);
        skip tmp >= 0 {
            print "Underflow";
            stop;
        }
        skip _ {
            :mis
            const e = tmp;
            print "Misses: " e;
            stop;
        }
        skip tmp <= 3 {
            const n = tmp;
            print "Overflow: " n;
            stop;
        }
        select tmp {
            goto :mis _;
            {
                print 1;
                end;
            }
            goto :mis _;
            {
                print 3 "!";
                end;
            }
        }
        "#
    );

    check_compile_eq!{parser,
        r#"
        switch (op $ x + 2;) {
            end;
        case <!>:
            stop;
        case 1:
            print 1;
        case 3:
            print 3 "!";
        }
        "#,
        r#"
        take tmp = (op $ x + 2;);
        skip tmp >= 0 && tmp <= 3 {
            :mis
            stop;
        }
        select tmp {
            goto :mis _;
            {
                print 1;
                end;
            }
            goto :mis _;
            {
                print 3 "!";
                end;
            }
        }
        "#
    };

    check_compile_eq!{parser,
        r#"
        switch (op $ x + 2;) {
            end;
        case <!>:
            stop;
        case (a < b):
            foo;
        case 1:
            print 1;
        case 3:
            print 3 "!";
        }
        "#,
        r#"
        take tmp = (op $ x + 2;);
        skip tmp >= 0 && tmp <= 3 {
            :mis
            stop;
        }
        skip !a < b {
            foo;
        }
        select tmp {
            goto :mis _;
            {
                print 1;
                end;
            }
            goto :mis _;
            {
                print 3 "!";
                end;
            }
        }
        "#
    };

    check_compile_eq!{parser,
        r#"
        switch (op $ x + 2;) {
            end;
        case !!!: # 捕获多个未命中也可以哦, 当然只有最后一个生效
            stop;
        case 1:
            print 1;
        case 3:
            print 3 "!";
        }
        "#,
        r#"
        take tmp = (op $ x + 2;);
        skip _ {
            :mis
            stop;
        }
        select tmp {
            goto :mis _;
            {
                print 1;
                end;
            }
            goto :mis _;
            {
                print 3 "!";
                end;
            }
        }
        "#
    };

    check_compile_eq!{parser,
        r#"
        switch (op $ x + 2;) {
            end;
        case !!!: # 捕获多个未命中也可以哦, 当然只有最后一个生效
            stop;
        case !:
            foo; # 最后一个
        case 1:
            print 1;
        case 3:
            print 3 "!";
        }
        "#,
        r#"
        take tmp = (op $ x + 2;);
        skip _ {
            # 可以看出, 这个是一个没用的捕获, 也不会被跳转
            # 所以不要这么玩, 浪费跳转和行数
            :mis
            stop;
        }
        skip _ {
            :mis1
            foo;
        }
        select tmp {
            goto :mis1 _;
            {
                print 1;
                end;
            }
            goto :mis1 _;
            {
                print 3 "!";
                end;
            }
        }
        "#
    };

    check_sugar! {parser,
        r#"
        # align...
        switch (op $ x + 2;) {
            end;
        case <!> e:
            `e` = e;
            stop;
        case (e < x) e:
            foo;
        case 1:
            print 1;
        case 3:
            print 3;
        }
        "#,
        r#"
        {
            take ___0 = (op $ x + 2;);
            {
                {
                    const e = ___0;
                    goto :___1 ___0 >= `0` && ___0 <= `3`;
                    :___0
                    {
                        `e` = e;
                        stop;
                    }
                    :___1
                }
                {
                    const e = ___0;
                    goto :___2 ! e < x;
                    {
                        foo;
                    }
                    :___2
                }
            }
            select ___0 {
                goto :___0 _;
                {
                    print 1;
                    end;
                }
                goto :___0 _;
                {
                    print 3;
                    end;
                }
            }
        }
        "#
    }

    check_sugar! {parser,
        r#"
        # align...
        switch (op $ x + 2;) {
            end;
        case <> e:
            `e` = e;
            stop;
        case (e < x) e:
            foo;
        case 1:
            print 1;
        case 3:
            print 3;
        }
        "#,
        r#"
        {
            take ___0 = (op $ x + 2;);
            {
                {
                    const e = ___0;
                    goto :___0 ___0 >= `0` && ___0 <= `3`;
                    {
                        `e` = e;
                        stop;
                    }
                    :___0
                }
                {
                    const e = ___0;
                    goto :___1 ! e < x;
                    {
                        foo;
                    }
                    :___1
                }
            }
            select ___0 {
                { end; }
                {
                    print 1;
                    end;
                }
                { end; }
                {
                    print 3;
                    end;
                }
            }
        }
        "#
    }

    check_compile_eq!{parser,
        r#"
        # align...
        switch (op $ x + 2;) {
        case !:
            stop;
        case 1:
        case 3:
        }
        "#,
        r#"
        take tmp = (op $ x + 2;);
        skip _ {
            :mis
            stop;
        }
        select tmp {
            goto :mis;
            {}
            goto :mis;
            {}
        }
        "#
    };
}

#[test]
fn switch_ignore_append_test() {
    let parser = TopLevelParser::new();

    check_compile_eq!{parser,
        r#"
        switch i {
            end;
        case*0: print 0;
        case 1: print 1;
        case*3: print 3;
        }
        "#,
        r#"
        select i {
            { print 0; }
            { print 1; end; }
            { end; }
            { print 3; }
        }
        "#
    };
}

#[test]
fn quick_dexp_take_test() {
    let parser = TopLevelParser::new();

    assert_eq!(
        parse!(parser, r#"
            print Foo[1 2];
        "#).unwrap(),
        vec![LogicLine::Other(vec![
            Value::ReprVar("print".into()),
            DExp::new_notake("__".into(), vec![
                LogicLine::SetArgs(vec!["1".into(), "2".into()].into()),
                LogicLine::SetResultHandle("Foo".into(), Some(IdxBox::new(19, ()))),
            ].into()).into(),
        ].into())].into(),
    );


    check_compile!{parser,
        r#"
        const Add = (
            take A = _0;
            take B = _1;
            op $ A + B;
        );
        print Add[1 2];
        "#,
        r#"
               op add __2 1 2
               print __2
        "#
    };

    check_compile!{parser,
        r#"
        const Add = (
            take A = _0;
            take B = _1;
            op $ A + B;
        );
        const Do = (_unused:
            const Fun = _0;

            print enter Fun;
        );
        take[Add[1 2]] Do;
        "#,
        r#"
               print enter
               op add __3 1 2
               print __3
        "#
    };

    check_sugar! {parser,
        r#"
        const V = F->[A B C @]->V;
        "#,
        r#"
        const V = F[A B C @]->$->V;
        "#,
    };

    check_sugar! {parser,
        r#"
                        Foo! a b c @ d;
        "#,
        r#"
        take[a b c @ d] Foo;
        "#,
    };

    check_sugar! {parser,
        r#"
        # align*************************************************
        Foo! a b c d++;
        "#,
        r#"
        inline {
            take ___0 = d;
            take[a b c ___0] Foo;
            ___0 = ___0 + `1`;
        }
        "#,
    };

    check_sugar! {parser,
        r#"
        # align*******************************************
        Foo! d++;
        "#,
        r#"
        inline {
            take ___0 = d;
            take[___0] Foo;
            ___0 = ___0 + `1`;
        }
        "#,
    };

    check_sugar! {parser,
        r#"
        # align***************************************************
        Foo! a b c @ d++;
        "#,
        r#"
        inline {
            take ___0 = d;
            take[a b c @ ___0] Foo;
            ___0 = ___0 + `1`;
        }
        "#,
    };

    check_sugar! {parser,
        r#"
        # align***************************************************
        Foo! @ a b c d++;
        "#,
        r#"
        inline {
            take ___0 = d;
            take[@ a b c ___0] Foo;
            ___0 = ___0 + `1`;
        }
        "#,
    };

    check_sugar! {parser,
        r#"
        # align***************************************************
        Foo! a++ b c @ d;
        "#,
        r#"
        inline {
            take ___0 = a;
            take[___0 b c @ d] Foo;
            ___0 = ___0 + `1`;
        }
        "#,
    };

    check_sugar! {parser,
        r#"
        # align*******************************************
        Foo! a++;
        "#,
        r#"
        inline {
            take ___0 = a;
            take[___0] Foo;
            ___0 = ___0 + `1`;
        }
        "#,
    };

    check_sugar! {parser,
        r#"
        foo a++;
        "#,
        r#"
        inline {
            take ___0 = a;
            foo ___0;
            ___0 = ___0 + `1`;
        }
        "#,
    };

    check_sugar! {parser,
        r#"
        foo @ a++;
        "#,
        r#"
        inline {
            take ___0 = a;
            foo @ ___0;
            ___0 = ___0 + `1`;
        }
        "#,
    };

    check_sugar! {parser,
        r#"
        foo x @ a++;
        "#,
        r#"
        inline {
            take ___0 = a;
            foo x @ ___0;
            ___0 = ___0 + `1`;
        }
        "#,
    };

    check_sugar! {parser,
        r#"
        foo a++ @ x;
        "#,
        r#"
        inline {
            take ___0 = a;
            foo ___0 @ x;
            ___0 = ___0 + `1`;
        }
        "#,
    };

    check_sugar! {parser,
        r#"
        foo a++ @;
        "#,
        r#"
        inline {
            take ___0 = a;
            foo ___0 @;
            ___0 = ___0 + `1`;
        }
        "#,
    };

    check_sugar! {parser,
        r#"
        foo (bar a++;) x++;
        "#,
        r#"
        inline {
            take ___1 = x;
            foo (inline {
                take ___0 = a;
                bar ___0;
                ___0 = ___0 + `1`;
            }) ___1;
            ___1 = ___1 + `1`;
        }
        "#,
    };

    check_sugar! {parser,
        r#"
        print a++;
        "#,
        r#"
        inline {
            take ___0 = a;
            print ___0;
            ___0 = ___0 + `1`;
        }
        "#,
    };

    check_sugar! {parser,
        r#"
        # align................................
        print @ a++;
        "#,
        r#"
        inline {
            take ___0 = a;
            print @ ___0;
            ___0 = ___0 + `1`;
        }
        "#,
    };

    check_sugar! {parser,
        r#"
        # align................................
        print x @ a++;
        "#,
        r#"
        inline {
            take ___0 = a;
            print x @ ___0;
            ___0 = ___0 + `1`;
        }
        "#,
    };

    check_sugar! {parser,
        r#"
        # align................................
        print a++ @;
        "#,
        r#"
        inline {
            take ___0 = a;
            print ___0 @;
            ___0 = ___0 + `1`;
        }
        "#,
    };

    check_sugar! {parser,
        r#"
        # align................................
        print a++ @ x;
        "#,
        r#"
        inline {
            take ___0 = a;
            print ___0 @ x;
            ___0 = ___0 + `1`;
        }
        "#,
    };

    check_sugar! {parser,
        r#"
        print (print a++;) x++;
        "#,
        r#"
        inline {
            take ___1 = x;
            print (inline {
                take ___0 = a;
                print ___0;
                ___0 = ___0 + `1`;
            }) ___1;
            ___1 = ___1 + `1`;
        }
        "#,
    };

    // 因为利用了命令做 op-expr 的返回, 所以多返回时也可以应用++
    check_sugar! {parser,
        r#"
        a, b++ = 2;
        "#,
        r#"
        inline {
            take ___0 = b;
            {
                take ___1 = a;
                ___1 = 2;
                ___0 = ___1;
            }
            ___0 = ___0 + `1`;
        }
        "#,
    };

    check_sugar! {parser,
        r#"
        # align*************************
        Foo! +A 2;
        "#,
        r#"
        inline {
            take+A;
            Foo! A 2;
        }
        "#,
    };

    check_sugar! {parser,
        r#"
        read {X} cell1 i;
        "#,
        r#"
        inline {
            take X = ();
            read X cell1 i;
        }
        "#,
    };

    check_sugar! {parser,
        r#"
        read {X}++ cell1 i;
        "#,
        r#"
        inline {
            take X = ();
            read X cell1 i;
            X = X + `1`;
        }
        "#,
    };

    check_sugar! {parser,
        r#"
        +X = 2;
        "#,
        r#"
        inline {
            take X = ();
            X = 2;
        }
        "#,
    };

    check_sugar! {parser,
        r#"
        +X+Y = 2;
        "#,
        r#"
        inline {
            take X = ();
            take Y = ();
            X Y = 2;
        }
        "#,
    };

    check_sugar! {parser,
        r#"
        +X Y = 2;
        "#,
        r#"
        inline {
            take X = ();
            X Y = 2;
        }
        "#,
    };

    check_sugar! {parser,
        r#"
        +X,Y = 2;
        "#,
        r#"
        inline {
            take X = ();
            X Y = 2;
        }
        "#,
    };
}

#[test]
fn value_bind_test() {
    let parser = TopLevelParser::new();

    check_compile!{parser,
        r#"
        const Jack = jack;
        Jack, Jack.age = "jack", 18;
        print Jack Jack.age;
        "#,
        r#"
               set jack "jack"
               set __0 18
               print jack
               print __0
        "#
    };

    check_compile!{parser,
        r#"
        print a.b.c;
        print a.b;
        "#,
        r#"
               print __1
               print __0
        "#
    };

    check_sugar! {parser,
        r#"
        take Foo = (%x).y;
        "#,
        r#"
        take Foo = x.y;
        "#,
    };

    check_sugar! {parser,
        r#"
        print (%(print 1 2;)).x;
        "#,
        r#"
        print (%print 1 2;%).x;
        "#,
    };

    check_sugar! {parser,
        r#"
        print (%(x: print 1 2;)).x;
        "#,
        r#"
        print  (%x: print 1 2;%).x;
        "#,
    };

    check_compile!{parser,
        r#"
        print (%()).x;
        "#,
        r#"
               print __1
        "#
    };

    check_sugar! {parser,
        r#"
        print (%(%(%(print 2;))));
        "#,
        r#"
        print (print 2;);
        "#,
    };

    check_sugar! {parser,
        r#"
        print (%(%(%(print 2;)))).x;
        "#,
        r#"
        print (print 2;).x;
        "#,
    };

    check_sugar! {parser,
        r#"
        print (%(?2)).x;
        "#,
        r#"
        print (?2).x;
        "#,
    };
}

#[test]
fn no_string_var_test() {
    let parser = VarParser::new();

    assert!(parse!(parser, r#"1"#).is_ok());
    assert!(parse!(parser, r#"1.5"#).is_ok());
    assert!(parse!(parser, r#"sbosb"#).is_ok());
    assert!(parse!(parser, r#"0x1b"#).is_ok());
    assert!(parse!(parser, r#"@abc"#).is_ok());
    assert!(parse!(parser, r#"'My_name"s'"#).is_ok());
    assert!(parse!(parser, r#"'"no_str"'"#).is_ok());
}

#[test]
fn jumpcmp_from_str_test() {
    let datas = [
        (args!("always"), Err(JumpCmpRParseError::ArgsCountError(
            vec!["always".into()]
        ).into())),
        (args!("always", "0"), Err(JumpCmpRParseError::ArgsCountError(
            vec!["always".into(), "0".into()]
        ).into())),
        (args!("add", "1", "2"), Err(JumpCmpRParseError::UnknownComparer(
            "add".into(),
            ["1".into(), "2".into()]
        ).into())),
        (args!("equal", "a", "b"), Ok(JumpCmp::Equal("a".into(), "b".into()))),
        (args!("lessThan", "a", "b"), Ok(JumpCmp::LessThan("a".into(), "b".into()))),
        (args!("always", "0", "0"), Ok(JumpCmp::Always)),
    ];

    for (src, expect) in datas {
        assert_eq!(JumpCmp::from_mdt_args(src), expect)
    }
}

#[test]
fn logic_line_from() {
    type Error = (usize, LogicLineFromTagError);
    let datas: [(&str, Result<Vec<LogicLine>, Error>); 2] = [
        (
            "op add i i 1",
            Ok(vec![
               Op::Add("i".into(), "i".into(), "1".into()).into(),
            ])
        ),
        (
            "op add i i 1\njump 0 lessThan i 10",
            Ok(vec![
               LogicLine::Label("0".into()).into(),
               Op::Add("i".into(), "i".into(), "1".into()).into(),
               Goto("0".into(), JumpCmp::LessThan("i".into(), "10".into()).into()).into(),
            ])
        ),
    ];
    for (src, lines2) in datas {
        let logic_lines = logic_parser::parser::lines(src).unwrap();
        assert_eq!(
            Expand::try_from(logic_lines).map_err(|e| (e.index, e.value)),
            lines2.map(Expand)
        );
    }
}

#[test]
fn op_expr_test() {
    let parser = TopLevelParser::new();

    check_compile!{parser,
        r#"
        a, b, c = 1, 2, ({}op $ 2 + 1;);
        "#,
        r#"
            set a 1
            set b 2
            op add __0 2 1
            set c __0
        "#
    };

    assert!(parse!(parser, r#"
    a, b, c = 1 2;
    "#).is_err());

    assert!(parse!(parser, r#"
    a = 1 2;
    "#).is_err());

    assert!(parse!(parser, r#"
     = 1 2;
    "#).is_err());

    assert!(parse!(parser, r#"
    a, b, c = 1, 2;
    "#).is_err());

    assert!(parse!(parser, r#"
    a = 1, 2;
    "#).is_err());

    assert!(parse!(parser, r#"
     = 1, 2;
    "#).is_err());

    check_sugar! {parser,
        r#"
        x = max(1, 2);
        y = max(max(1, 2), max(3, max(4, 5)));
        "#,
        r#"
        op x max 1 2;
        op y max (op $ max 1 2;) (op $ max 3 (op $ max 4 5;););
        "#,
    };

    check_sugar! {parser,
        r#"
        x = round(8);
        y = sign(6);
        z = logn(4, 2);
        t = log(4, 2);
        m = 8 >>> 3;
        "#,
        r#"
        op round x 8;
        op sign y 6;
        op logn z 4 2;
        op logn t 4 2;
        op ushr m 8 3;
        "#,
    };

    check_sugar! {parser,
        r#"
        x = 1+2*3;
        y = (1+2)*3;
        z = 1+2+3;
        "#,
        r#"
        op x 1 + (op $ 2 * 3;);
        op y (op $ 1 + 2;) * 3;
        op z (op $ 1 + 2;) + 3;
        "#,
    };

    check_sugar! {parser,
        r#"
        x = 1*max(2, 3);
        y = a & b | c & d & e | f;
        "#,
        r#"
        op x 1 * (op $ max 2 3;);
        op y (op $ (op $ a & b;) | (op $ (op $ c & d;) & e;);) | f;
        "#,
    };

    check_sugar! {parser,
        r#"
        x = a**b**c; # pow的右结合
        y = -x;
        z = ~y;
        e = a !== b;
        "#,
        r#"
        op x a ** (op $ b ** c;);
        op y `0` - x;
        op z ~y;
        op e (op $ a === b;) == `false`;
        "#,
    };

    check_sugar! {parser,
        r#"
        a, b, c = x, -y, z+2*3;
        "#,
        r#"
        {
            a = x;
            b = -y;
            c = z+2*3;
        }
        "#,
    };

    check_sugar! {parser,
        r#"
        a b c = x, -y, z+2*3;
        "#,
        r#"
        {
            a = x;
            b = -y;
            c = z+2*3;
        }
        "#,
    };

    check_sugar! {parser,
        r#"
        a, b, c = 1;
        "#,
        r#"
        {
            take ___0 = a;
            ___0 = 1;
            b = ___0;
            c = ___0;
        }
        "#,
    };

    check_sugar! {parser,
        r#"
        a b c = 1;
        "#,
        r#"
        {
            take ___0 = a;
            ___0 = 1;
            b = ___0;
            c = ___0;
        }
        "#,
    };

    check_sugar! {parser,
        r#"
        x = a < b == c > d;
        x = a < b != c > d;
        x = a < b === c > d;
        x = a < b !== c > d;
        "#,
        r#"
        op x (op $ a < b;) == (op $ c > d;);
        op x (op $ a < b;) != (op $ c > d;);
        op x (op $ a < b;) === (op $ c > d;);
        op x (op $ a < b;) !== (op $ c > d;);
        "#,
    };

    check_sugar! {parser,
        r#"
        a = if x ? y : y+z;
        "#,
        r#"
        {
            take ___0 = a;
            goto :___0 x;
                ___0 = y + z;
            goto :___1;
            :___0
                ___0 = y;
            :___1
        }
        "#,
    };

    check_sugar! {parser,
        r#"
        a = select x ? y : y+z;
        "#,
        r#"
        `select` a `notEqual` x `false` y (?y+z);
        "#,
    };

    assert!(
        parse!(parser, r#"
        x = a == b == c;
        "#).is_err(),
    );

    assert!(
        parse!(parser, r#"
        x = a < b < c;
        "#).is_err(),
    );

    assert!(
        parse!(parser, r#"
        x = a === b === c;
        "#).is_err(),
    );

    check_sugar! {parser,
        r#"
        x = (a < b) < c;
        "#,
        r#"
        op x (op $ a < b;) < c;
        "#,
    };

    check_sugar! {parser,
        r#"
        x += 2;
        "#,
        r#"
        {
            take ___0 = x;
            op ___0 ___0 + 2;
        }
        "#,
    };

    check_sugar! {parser,
        r#"
        x += y*z;
        "#,
        r#"
        {
            take ___0 = x;
            op ___0 ___0 + (op $ y * z;);
        }
        "#,
    };

    check_sugar! {parser,
        r#"
        take Foo = (?a+b);
        "#,
        r#"
        take Foo = ($ = a+b;);
        "#,
    };

    check_sugar! {parser,
        r#"
        take Foo = (?m: a+b);
        "#,
        r#"
        take Foo =  (m: $ = a+b;);
        "#,
    };

    check_sugar! {parser,
        r#"
        a, b += 2;
        "#,
        r#"
        {
            take ___0 = 2;
            {take ___1 = a; ___1 = ___1 + ___0;}
            {take ___2 = b; ___2 = ___2 + ___0;}
        }
        "#,
    };

    check_sugar! {parser,
        r#"
        a, b += 2, 3;
        "#,
        r#"
        {
            a += 2;
            b += 3;
        }
        "#,
    };

    check_sugar! {parser,
        r#"
        a, b min= 2, 3;
        "#,
        r#"
        {
            a min= 2;
            b min= 3;
        }
        "#,
    };

    check_sugar! {parser,
        r#"
        a, b max= 2, 3;
        "#,
        r#"
        {
            a max= 2;
            b max= 3;
        }
        "#,
    };

    check_sugar! {parser,
        r#"
        x min= 2;
        "#,
        r#"
        {
            take ___0 = x;
            op ___0 min ___0 2;
        }
        "#,
    };

    check_sugar! {parser,
        r#"
        a, b min= 2;
        "#,
        r#"
        {
            take ___0 = 2;
            {take ___1 = a; op ___1 min ___1 ___0;}
            {take ___2 = b; op ___2 min ___2 ___0;}
        }
        "#,
    };

    check_sugar! {parser,
        r#"
        x = ++i;
        "#,
        r#"
        x = (`__`:
            setres i;
            $ = $ + `1`;
        );
        "#,
    };

    check_sugar! {parser,
        r#"
        x = --i;
        "#,
        r#"
        x = (`__`:
            setres i;
            $ = $ - `1`;
        );
        "#,
    };

    check_sugar! {parser,
        r#"
        x = i++;
        "#,
        r#"
        {
            take ___0 = i;
            x = ___0;
            ___0 = ___0 + `1`;
        }
        "#,
    };

    check_sugar! {parser,
        r#"
        x = 2 + ++i;
        "#,
        r#"
        x = 2 + (`__`:
            setres i;
            $ = $ + `1`;
        );
        "#,
    };

    check_sugar! {parser,
        r#"
        x = 2 + --i;
        "#,
        r#"
        x = 2 + (`__`:
            setres i;
            $ = $ - `1`;
        );
        "#,
    };

    check_sugar! {parser,
        r#"
        x = 2 + i++;
        "#,
        r#"
        x = 2 + (
            take ___0 = i;
            $ = ___0;
            ___0 = ___0 + `1`;
        );
        "#,
    };

    check_sugar! {parser,
        r#"
        x = i++(2+_);
        "#,
        r#"
        {
            take ___0 = i;
            x = 2 + ___0;
            ___0 = ___0 + `1`;
        }
        "#,
    };

    check_sugar! {parser,
        r#"
        x = 8+i++(2+_);
        "#,
        r#"
        x = 8 + (
            take ___0 = i;
            $ = 2 + ___0;
            ___0 = ___0 + `1`;
        );
        "#,
    };

    check_sugar! {parser,
        r#"
        x = 8+i++(j++(_) + _);
        "#,
        r#"
        x = 8 + (
            take ___0 = i;
            $ = (
                take ___1 = j;
                $ = ___1;
                ___1 = ___1 + `1`;
            ) + ___0;
            ___0 = ___0 + `1`;
        );
        "#,
    };

    check_sugar! {parser,
        r#"
        print (?++i);
        "#,
        r#"
        print ($ = (`__`: setres i; $ = $ + `1`;););
        "#,
    };

    check_sugar! {parser,
        r#"
        print (*++i);
        "#,
        r#"
        print (`__`: setres i; $ = $ + `1`;);
        "#,
    };

    check_sugar! {parser,
        r#"
        take*A, B = x+y, i++;
        take*C = j--;
        "#,
        r#"
        take A = (*x+y) B = (*i++);
        take C = (*j--);
        "#,
    };

    check_sugar! {parser,
        r#"
        take*A, B = M;
        "#,
        r#"
        inline {
            take*A = M;
            take*B = A;
        }
        "#,
    };

    check_sugar! {parser,
        r#"
        take*A.V, B.V = M;
        "#,
        r#"
        inline {
            take ___0 = A;
            take*___0.V = M;
            take*B.V = ___0.V;
        }
        "#,
    };

    check_sugar! {parser,
        r#"
        take*A, B = [x+y, i++];
        take*C = j--;
        "#,
        r#"
        take A = (*x+y) B = (*i++);
        take C = (*j--);
        "#,
    };

    // 连续运算, 返回者是最左边的
    check_sugar! {parser,
        r#"
        a = b = c;
        "#,
        r#"
        inline {
            take ___0 = a;
            {
                ___0 = b;
                ___0 = c;
            }
        }
        "#,
    };

    check_sugar! {parser,
        r#"
        a = b += c;
        "#,
        r#"
        inline {
            take ___0 = a;
            {
                ___0 = b;
                {
                    take ___1 = ___0;
                    ___1 = ___1 + c;
                }
            }
        }
        "#,
    };

    check_sugar! {parser,
        r#"
        a += b *= c;
        "#,
        r#"
        inline {
            take ___0 = a;
            {
                {
                    take ___1 = ___0;
                    ___1 = ___1 + b;
                }
                {
                    take ___2 = ___0;
                    ___2 = ___2 * c;
                }
            }
        }
        "#,
    };

    check_sugar! {parser,
        r#"
        a, b += c *= d;
        "#,
        r#"
        inline {
            take ___0 = a;
            take ___1 = b;
            {
                {
                    take ___2 = c;
                    {
                        take ___3 = ___0;
                        op ___3 ___3 + ___2;
                    }
                    {
                        take ___4 = ___1;
                        op ___4 ___4 + ___2;
                    }
                }
                {
                    take ___5 = d;
                    {
                        take ___6 = ___0;
                        op ___6 ___6 * ___5;
                    }
                    {
                        take ___7 = ___1;
                        op ___7 ___7 * ___5;
                    }
                }
            }
        }
        "#,
    };

    check_sugar! {parser,
        r#"
        print (=x);
        "#,
        r#"
        print ({$=x;});
        "#,
    };

    check_sugar! {parser,
        r#"
        print (y:=x);
        "#,
        r#"
        print (y:{$=x;});
        "#,
    };

    check_sugar! {parser,
        r#"
        print (y: =x);
        "#,
        r#"
        print (y:{$=x;});
        "#,
    };

    check_sugar! {parser,
        r#"
        x = abs((?x,));
        "#,
        r#"
        x = abs(?x,);
        "#,
    };

    check_sugar! {parser,
        r#"
        x = abs((?x,),);
        "#,
        r#"
        x = abs(?x,);
        "#,
    };

    check_sugar! {parser,
        r#"
        x = abs((?x));
        "#,
        r#"
        x = abs(?x);
        "#,
    };

    check_sugar! {parser,
        r#"
        x = abs((?x+1));
        "#,
        r#"
        x = abs(?x+1);
        "#,
    };

    check_sugar! {parser,
        r#"
        x = abs((?m:x+1));
        "#,
        r#"
        x =  abs(?m:x+1);
        "#,
    };

    check_sugar! {parser,
        r#"
        x = abs((?m:x));
        "#,
        r#"
        x =  abs(?m:x);
        "#,
    };

    check_sugar! {parser,
        r#"
        x = abs((?`m`:x));
        "#,
        r#"
        x = abs(?`m`:x);
        "#,
    };

    check_sugar! {parser,
        r#"
        x = abs((=x));
        "#,
        r#"
        x = abs(=x);
        "#,
    };

    check_sugar! {parser,
        r#"
        x = abs((m:=x));
        "#,
        r#"
        x =  abs(m:=x);
        "#,
    };

    check_sugar! {parser,
        r#"
        x = abs((m:=x,));
        "#,
        r#"
        x =  abs(m:=x,);
        "#,
    };

    check_sugar! {parser,
        r#"
        x = abs((m:=x,));
        "#,
        r#"
        x =  abs(m:=x);
        "#,
    };

    check_sugar! {parser,
        r#"
        x = abs((`m`:=x));
        "#,
        r#"
        x = abs(`m`:=x);
        "#,
    };

    check_sugar! {parser,
        r#"
        x = abs((`m`:=x+1));
        "#,
        r#"
        x = abs(`m`:=x+1);
        "#,
    };

    check_sugar! {parser,
        r#"
        x =? abs((`m`:=x+1));
        "#,
        r#"
        x = abs(`m`:=x+1);
        "#,
    };

    check_sugar! {parser,
        r#"
        x =* abs((`m`:=x+1));
        "#,
        r#"
        x = abs(`m`:=x+1);
        "#,
    };

    check_sugar! {parser,
        r#"
        x = *abs((`m`:=x+1));
        "#,
        r#"
        x = abs(`m`:=x+1);
        "#,
    };

    check_sugar! {parser,
        r#"
        print (x:+=2);
        "#,
        r#"
        print (x:{$+=2});
        "#,
    };

    check_sugar! {parser,
        r#"
        print (`x`:+=2);
        "#,
        r#"
        print (`x`:{$+=2});
        "#,
    };

    check_sugar! {parser,
        r#"
        print (`x`:+=2*=2);
        "#,
        r#"
        print (`x`:{$+=2;$*=2});
        "#,
    };

    check_sugar! {parser,
        r#"
        a, b, c = x*2, -y*2, (z+3)*2;
        "#,
        r#"
        a, b, c = [x, -y, z+3]*2;
        "#,
    };

    check_sugar! {parser,
        r#"
        a, b, c = x*2, -y*2, z+3;
        "#,
        r#"
        a, b, c = [x, -y]*2, z+3;
        "#,
    };

    check_sugar! {parser,
        r#"
        a, b, c = z+3, x*2, -y*2;
        "#,
        r#"
        a, b, c = z+3, [x, -y]*2;
        "#,
    };

    check_sugar! {parser,
        r#"
        a, b, c = z+3, x*2, -y*2;
        "#,
        r#"
        a, b, c = z+3, [x, -y,]*2;
        "#,
    };

    check_sugar! {parser,
        r#"
        a, b, c, d = a*c, a*d, b*c, b*d;
        "#,
        r#"
        a, b, c, d = [a,b]*[c,d];
        "#,
    };

    check_sugar! {parser,
        r#"
        a, b = if x ? [1, 2] : 3;
        "#,
        r#"
        a, b = if x ? 1 : 3, if x ? 2 : 3;
        "#,
    };

    check_sugar! {parser,
        r#"
        a, b = select x ? [1, 2] : 3;
        "#,
        r#"
        a, b = select x ? 1 : 3, select x ? 2 : 3;
        "#,
    };

    check_sugar! {parser,
        r#"
        a = sign abs x.y;
        "#,
        r#"
        a = sign(abs(x.y));
        "#,
    };

    check_sugar! {parser,
        r#"
        a = sign abs angle(x, y);
        "#,
        r#"
        a = sign(abs(angle(x, y)));
        "#,
    };

    check_sugar! {parser,
        r#"
        a = sign abs x;
        "#,
        r#"
        a = sign(abs(x));
        "#,
    };

    check_sugar! {parser,
        r#"
        a = sign x;
        "#,
        r#"
        a = sign(x);
        "#,
    };

    check_sugar! {parser,
        r#"
        a = sign x++;
        "#,
        r#"
        a = sign(x++);
        "#,
    };

    check_sugar! {parser,
        r#"
        a = sign x++(_+1);
        "#,
        r#"
        a = sign(x++(_+1));
        "#,
    };

    check_sugar! {parser,
        r#"
        a = sign x.y++;
        "#,
        r#"
        a = sign(x.y++);
        "#,
    };

    check_sugar! {parser,
        r#"
        a = sign ++x.y;
        "#,
        r#"
        a = sign(++x.y);
        "#,
    };

    check_sugar! {parser,
        r#"
        a = abs sign (x:=2);
        "#,
        r#"
        a = abs sign((x:=2));
        "#,
    };

    check_sugar! {parser,
        r#"
        a, b = abs sign [x, y];
        "#,
        r#"
        a, b = abs sign x, abs sign y;
        "#,
    };

    assert!(
        parse!(parser, r#"
        a, b, c = z+3, []*2;
        "#).is_err()
    );

    check_sugar! {parser,
        r#"
        a, b = if a < b ? [x, y : i, j];
        "#,
        r#"
        {
            a = if a < b ? x : i;
            b = if a < b ? y : j;
        }
        "#,
    };

    check_sugar! {parser,
        r#"
        a, b = select a < b ? [x, y : i, j];
        "#,
        r#"
        {
            a = select a < b ? x : i;
            b = select a < b ? y : j;
        }
        "#,
    };
}

#[test]
fn op_test() {
    let parser = TopLevelParser::new();

    check_sugar! {parser,
        r#"
        op x a !== b;
        "#,
        r#"
        op x (op $ a === b;) == `false`;
        "#,
    };

    check_sugar! {parser,
        r#"
        op x a %% b;
        "#,
        r#"
        op x a emod b;
        "#,
    };

    check_sugar! {parser,
        r#"
        op x round a b;
        "#,
        r#"
        op round x a b;
        "#,
    };

    check_sugar! {parser,
        r#"
        i++;
        "#,
        r#"
        {
            take ___0 = i;
            op ___0 ___0 + `1`;
        }
        "#,
    };

    check_sugar! {parser,
        r#"
        i--;
        "#,
        r#"
        {
            take ___0 = i;
            op ___0 ___0 - `1`;
        }
        "#,
    };

    check_sugar! {parser,
        r#"
        ++i;
        "#,
        r#"
        {
            take ___0 = i;
            op ___0 ___0 + `1`;
        }
        "#,
    };

    check_sugar! {parser,
        r#"
        --i;
        "#,
        r#"
        {
            take ___0 = i;
            op ___0 ___0 - `1`;
        }
        "#,
    };
}

#[test]
fn inline_block_test() {
    let parser = TopLevelParser::new();

    assert_eq!(
        parse!(parser, r#"
        inline {
            foo;
        }
        "#).unwrap(),
        Expand(vec![
            InlineBlock(vec![
                LogicLine::Other(vec!["foo".into()].into())
            ]).into()
        ]).into()
    );

    check_compile!{parser,
        r#"
        print A;
        inline {
            const A = 2;
            print A;
        }
        print A;
        "#,
        r#"
               print A
               print 2
               print 2
        "#
    };
}

#[test]
fn consted_dexp() {
    let parser = TopLevelParser::new();

    assert_eq!(
        parse!(parser, r#"
        foo const(:x bar;);
        "#).unwrap(),
        Expand(vec![
            LogicLine::Other(vec![
                "foo".into(),
                DExp::new_notake(
                    "__".into(),
                    vec![
                        Const(
                            "___0".into(),
                            DExp::new_nores(vec![
                                LogicLine::Label("x".into()),
                                LogicLine::Other(vec!["bar".into()].into())
                            ].into()).into(),
                            vec!["x".into()],
                        ).into(),
                        LogicLine::SetResultHandle("___0".into(), Some(IdxBox::new(13, ()))),
                    ].into()
                ).into()
            ].into()),
        ]).into()
    );

    check_compile!{parser,
        r#"
        const Do2 = (
            const F = _0;
            take F;
            take F;
        );
        take[
            const(
                if a < b {
                    print 1;
                } else {
                    print 2;
                }
            )
        ] Do2;
        "#,
        r#"
               jump 3 lessThan a b
               print 2
               jump 4 always 0 0
               print 1
               jump 7 lessThan a b
               print 2
               jump 0 always 0 0
               print 1
        "#
    };

    check_compile!{parser,
        r#"
        const Do2 = (
            const F = _0;
            take F;
            take F;
        );
        take[
            const!(
                if a < b {
                    print 1;
                } else {
                    print 2;
                }
            )
        ] Do2;
        "#,
        r#"
               jump 3 lessThan a b
               print 2
               jump 4 always 0 0
               print 1
               jump 7 lessThan a b
               print 2
               jump 0 always 0 0
               print 1
        "#
    };

    assert!(CompileMeta::new().compile(parse!(parser, r#"
    const Do2 = (
        const F = _0;
        take F;
        take F;
    );
    take[
        (
            if a < b {
                print 1;
            } else {
                print 2;
            }
        )
    ] Do2;
    "#).unwrap()).compile().is_err());

    check_sugar! {parser,
        r#"
        foo const(:x bar;);
        "#,
        r#"
        foo const!(:x bar;);
        "#,
    };

    check_compile!{parser,
        r#"
        const x.Y = 2;
        print const!(%setres x;%).Y;
        "#,
        r#"
               print 2
        "#
    };
}

#[test]
fn inline_cmp_op_test() {
    let parser = TopLevelParser::new();

    check_compile!{parser,
        r#"
        :0 goto :0 a < b;
        "#,
        r#"
            jump 0 lessThan a b
        "#,
    };

    check_compile_eq!{parser,
        r#"
        :0 goto :0 a < b;
        "#,
        r#"
        :0 goto :0 a < b;
        "#,
    };

    check_compile_eq!{parser,
        r#"
        :0 goto :0 (op $ a < b;);
        "#,
        r#"
        :0 goto :0 a < b;
        "#,
    };

    check_compile_eq!{parser,
        r#"
        :0 goto :0 (op $ a === b;);
        "#,
        r#"
        :0 goto :0 a === b;
        "#,
    };

    check_compile_eq!{parser,
        r#"
        :0 goto :0 (x: op x a === b;);
        "#,
        r#"
        :0 goto :0 (x: op x a === b;);
        "#,
    };

    check_compile_eq!{parser,
        r#"
        :0 goto :0 (op x a === b;);
        "#,
        r#"
        :0 goto :0 (op x a === b;);
        "#,
    };

    check_compile_eq!{parser,
        r#"
        :0 goto :0 (x: op $ a === b;);
        "#,
        r#"
        :0 goto :0 (x: op $ a === b;);
        "#,
    };

    check_compile_eq!{parser,
        r#"
        :0 goto :0 !(op $ a < b;);
        "#,
        r#"
        :0 goto :0 !a < b;
        "#,
    };

    check_compile_eq!{parser,
        r#"
        :0 goto :0 !!!(op $ a < b;);
        "#,
        r#"
        :0 goto :0 !!!a < b;
        "#,
    };

    // 暂未实现直接到StrictNotEqual, 目前这就算了吧, 反正最终编译产物一样
    check_compile_eq!{parser,
        r#"
        :0 goto :0 !(op $ a === b;);
        "#,
        r#"
        :0 goto :0 a !== b;
        "#,
    };

    check_compile_eq!{parser,
        r#"
        :0 goto :0 (noop; op $ a < b;);
        "#,
        r#"
        :0 goto :0 (noop; op $ a < b;);
        "#,
    };

    check_compile_eq!{parser,
        r#"
        :0 goto :0 (op $ a < b; noop;);
        "#,
        r#"
        :0 goto :0 (op $ a < b; noop;);
        "#,
    };

    // 连续内联的作用
    check_compile_eq!{parser,
        r#"
        :0 goto :0 (op $ !(op $ a < b;););
        "#,
        r#"
        :0 goto :0 !a < b;
        "#,
    };

    // 连续内联的作用
    check_compile_eq!{parser,
        r#"
        :0 goto :0 (op $ !(op $ !(op $ a < b;);););
        "#,
        r#"
        :0 goto :0 a < b;
        "#,
    };

    // 强化内联

    check_compile_eq!{parser,
        r#"
        const F = false;
        do {} while (op $ a < b;) != F;
        do {} while (op $ a < b;) == F;
        "#,
        r#"
        do {} while a < b;
        do {} while !a < b;
        "#,
    };

    check_compile_eq!{parser,
        r#"
        const F = 0;
        do {} while (op $ a < b;) != F;
        do {} while (op $ a < b;) == F;
        "#,
        r#"
        do {} while a < b;
        do {} while !a < b;
        "#,
    };

    check_compile_eq!{parser,
        r#"
        const F = 0;
        const Op = (op $ a < b;);
        do {} while Op != F;
        do {} while Op == F;
        "#,
        r#"
        do {} while a < b;
        do {} while !a < b;
        "#,
    };

    check_compile_eq!{parser,
        r#"
        const Op = (op $ a < b;);
        do {} while Op != (0:);
        do {} while Op == (0:);
        "#,
        r#"
        do {} while a < b;
        do {} while !a < b;
        "#,
    };

    check_compile_eq!{parser,
        r#"
        const F = (0:);
        const Op = (op $ a < b;);
        do {} while Op != F;
        do {} while Op == F;
        "#,
        r#"
        do {} while a < b;
        do {} while !a < b;
        "#,
    };

    check_compile_eq!{parser,
        r#"
        const F = (false:);
        const Op = (op $ a < b;);
        do {} while Op != F;
        do {} while Op == F;
        "#,
        r#"
        do {} while a < b;
        do {} while !a < b;
        "#,
    };

    check_compile_eq!{parser,
        r#"
        const F = 0;
        const Op = (op $ (op $ a < b;) != F;);
        do {} while Op != F;
        do {} while Op == F;
        "#,
        r#"
        do {} while a < b;
        do {} while !a < b;
        "#,
    };

    check_compile_eq!{parser,
        r#"
        const F = 0;
        const Op = (op $ (op $ a < b;) == F;);
        do {} while Op != F;
        do {} while Op == F;
        "#,
        r#"
        do {} while !a < b;
        do {} while a < b;
        "#,
    };

    check_compile_eq!{parser,
        r#"
        const false = 2;
        const Cmp = goto(a < b);
        do {} while Cmp != (`false`:);
        "#,
        r#"
        const Cmp = goto(a < b);
        do {} while Cmp != (`false`:);
        "#,
    };

    check_compile!{parser,
        r#"
        const false = 2;
        const Cmp = (?a < b);
        break Cmp != (`false`:);
        "#,
        r#"
            jump 0 lessThan a b
        "#,
    };

    check_compile!{parser,
        r#"
        const false = 2;
        const Cmp = (?a < b);
        break Cmp != (false:);
        "#,
        r#"
            op lessThan __0 a b
            jump 0 notEqual __0 2
        "#,
    };

    check_compile!{parser,
        r#"
        const false = 2;
        const Cmp = (?m: a < b);
        break Cmp != (`false`:);
        "#,
        r#"
            op lessThan m a b
            jump 0 notEqual m false
        "#,
    };

    check_compile!{parser,
        r#"
        const false = 2;
        const Cmp = (?a < b);
        break Cmp != (`false`: {});
        "#,
        r#"
            op lessThan __0 a b
            jump 0 notEqual __0 false
        "#,
    };

    check_compile!{parser,
        r#"
        const false = 2;
        const Cmp = (?a < b);
        break Cmp == (`false`: {});
        "#,
        r#"
            op lessThan __0 a b
            jump 0 equal __0 false
        "#,
    };

    check_compile!{parser,
        r#"
        break (?a<b);
        break (?a<=b);
        break (?a>b);
        break (?a>=b);
        "#,
        r#"
            jump 0 lessThan a b
            jump 0 lessThanEq a b
            jump 0 greaterThan a b
            jump 0 greaterThanEq a b
        "#,
    };

    check_compile!{parser,
        r#"
        const false = 2;
        const Cmp = (?a < b);
        break Cmp == (`false`:);
        "#,
        r#"
            jump 0 greaterThanEq a b
        "#,
    };
}

#[test]
fn top_level_break_and_continue_test() {
    let parser = TopLevelParser::new();

    check_compile!{parser,
        r#"
        foo;
        continue;
        bar;
        "#,
        r#"
            foo
            jump 0 always 0 0
            bar
        "#,
    };

    check_compile!{parser,
        r#"
        foo;
        continue _;
        bar;
        "#,
        r#"
            foo
            jump 0 always 0 0
            bar
        "#,
    };

    check_compile!{parser,
        r#"
        foo;
        continue a < b;
        bar;
        "#,
        r#"
            foo
            jump 0 lessThan a b
            bar
        "#,
    };

    check_compile!{parser,
        r#"
        foo;
        continue a < b || c < d;
        bar;
        "#,
        r#"
            foo
            jump 0 lessThan a b
            jump 0 lessThan c d
            bar
        "#,
    };

    check_compile!{parser,
        r#"
        foo;
        continue;
        bar;
        "#,
        r#"
            foo
            jump 0 always 0 0
            bar
        "#,
    };

    check_compile!{parser,
        r#"
        foo;
        continue _;
        bar;
        "#,
        r#"
            foo
            jump 0 always 0 0
            bar
        "#,
    };

    check_compile!{parser,
        r#"
        foo;
        continue a < b;
        bar;
        "#,
        r#"
            foo
            jump 0 lessThan a b
            bar
        "#,
    };

    check_compile!{parser,
        r#"
        foo;
        continue a < b || c < d;
        bar;
        "#,
        r#"
            foo
            jump 0 lessThan a b
            jump 0 lessThan c d
            bar
        "#,
    };

}

#[test]
fn control_stmt_break_and_continue_test() {
    let parser = TopLevelParser::new();

    check_compile!{parser,
        r#"
        foo;
        while a < b {
            foo1;
            while c < d {
                foo2;
                break;
            }
            bar1;
            break;
        }
        bar;
        break;
        "#,
        r#"
            foo
            jump 10 greaterThanEq a b
            foo1
            jump 7 greaterThanEq c d
            foo2
            jump 7 always 0 0
            jump 4 lessThan c d
            bar1
            jump 10 always 0 0
            jump 2 lessThan a b
            bar
            jump 0 always 0 0
        "#,
    };

    check_compile!{parser,
        r#"
        foo;
        gwhile a < b {
            foo1;
            gwhile c < d {
                foo2;
                break;
            }
            bar1;
            break;
        }
        bar;
        break;
        "#,
        r#"
            foo
            jump 9 always 0 0
            foo1
            jump 6 always 0 0
            foo2
            jump 7 always 0 0
            jump 4 lessThan c d
            bar1
            jump 10 always 0 0
            jump 2 lessThan a b
            bar
            jump 0 always 0 0
        "#,
    };

    check_compile!{parser,
        r#"
        foo;
        xxx;
        do {
            foo1;
            xxx;
            do {
                foo2;
                break;
            } while c < d;
            bar1;
            break;
        } while a < b;
        bar;
        break;
        "#,
        r#"
            foo
            xxx
            foo1
            xxx
            foo2
            jump 7 always 0 0
            jump 4 lessThan c d
            bar1
            jump 10 always 0 0
            jump 2 lessThan a b
            bar
            jump 0 always 0 0
        "#,
    };

    check_compile!{parser,
        r#"
        switch a {
        case 0: foo;
        case 1: break;
        case 2: bar;
        }
        end;
        break;
        "#,
        r#"
            op add @counter @counter a
            foo
            jump 4 always 0 0
            bar
            end
            jump 0 always 0 0
        "#,
    };

    check_compile!{parser,
        r#"
        select a {
            foo;
            break;
            bar;
        }
        end;
        break;
        "#,
        r#"
            op add @counter @counter a
            foo
            jump 4 always 0 0
            bar
            end
            jump 0 always 0 0
        "#,
    };

    check_compile!{parser,
        r#"
        foo;
        while a < b {
            foo1;
            while c < d {
                foo2;
                continue;
            }
            bar1;
            continue;
        }
        bar;
        continue;
        "#,
        r#"
            foo
            jump 10 greaterThanEq a b
            foo1
            jump 7 greaterThanEq c d
            foo2
            jump 6 always 0 0
            jump 4 lessThan c d
            bar1
            jump 9 always 0 0
            jump 2 lessThan a b
            bar
            jump 0 always 0 0
        "#,
    };

    // 4 -> 6
    check_compile!{parser,
        r#"
        foo;
        while a < b {
            foo1;
            while c < d {
                continue;
                foo2;
            }
            bar1;
            continue;
        }
        bar;
        continue;
        "#,
        r#"
            foo
            jump 10 greaterThanEq a b
            foo1
            jump 7 greaterThanEq c d
            jump 6 always 0 0
            foo2
            jump 6 lessThan c d
            bar1
            jump 9 always 0 0
            jump 2 lessThan a b
            bar
            jump 0 always 0 0
        "#,
    };

    check_compile!{parser,
        r#"
        foo;
        gwhile a < b {
            foo1;
            gwhile c < d {
                foo2;
                continue;
            }
            bar1;
            continue;
        }
        bar;
        continue;
        "#,
        r#"
            foo
            jump 9 always 0 0
            foo1
            jump 6 always 0 0
            foo2
            jump 6 always 0 0
            jump 4 lessThan c d
            bar1
            jump 9 always 0 0
            jump 2 lessThan a b
            bar
            jump 0 always 0 0
        "#,
    };

    check_compile!{parser,
        r#"
        foo;
        xxx;
        do {
            foo1;
            xxx;
            do {
                foo2;
                continue;
            } while c < d;
            bar1;
            continue;
        } while a < b;
        bar;
        continue;
        "#,
        r#"
            foo
            xxx
            foo1
            xxx
            foo2
            jump 6 always 0 0
            jump 4 lessThan c d
            bar1
            jump 9 always 0 0
            jump 2 lessThan a b
            bar
            jump 0 always 0 0
        "#,
    };

    check_compile!{parser,
        r#"
        switch a {
        case 0: foo;
        case 1: continue;
        case 2: bar;
        }
        end;
        continue;
        "#,
        r#"
            op add @counter @counter a
            foo
            jump 0 always 0 0
            bar
            end
            jump 0 always 0 0
        "#,
    };

    check_compile!{parser,
        r#"
        select a {
            foo;
            continue;
            bar;
        }
        end;
        continue;
        "#,
        r#"
            op add @counter @counter a
            foo
            jump 0 always 0 0
            bar
            end
            jump 0 always 0 0
        "#,
    };

    check_compile!{parser,
        r#"
        end;
        switch a {
        case 0: foo;
        case 1: continue;
        case 2: bar;
        }
        end;
        continue;
        "#,
        r#"
            end
            op add @counter @counter a
            foo
            jump 1 always 0 0
            bar
            end
            jump 0 always 0 0
        "#,
    };

    check_compile!{parser,
        r#"
        end;
        select a {
            foo;
            continue;
            bar;
        }
        end;
        continue;
        "#,
        r#"
            end
            op add @counter @counter a
            foo
            jump 1 always 0 0
            bar
            end
            jump 0 always 0 0
        "#,
    };

}

#[test]
fn op_expr_if_else_test() {
    let parser = TopLevelParser::new();

    check_sugar! {parser,
        r#"
        a = if b < c ? b + 2 : c;
        "#,
        r#"
        {
            take ___0 = a;
            goto :___0 b < c;
            `set` ___0 c;
            goto :___1 _;
            :___0
            op ___0 b + 2;
            :___1
        }
        "#,
    };

    check_sugar! {parser,
        r#"
        a = (if b < c ? b + 2 : c);
        "#,
        r#"
        {
            take ___0 = a;
            goto :___0 b < c;
            `set` ___0 c;
            goto :___1 _;
            :___0
            op ___0 b + 2;
            :___1
        }
        "#,
    };

    check_sugar! {parser,
        r#"
        a = if b < c ? b + 2 : if d < e ? 8 : c - 2;
        "#,
        r#"
        {
            take ___1 = a;
            goto :___2 b < c;
            {
                take ___0 = ___1;
                goto :___0 d < e;
                op ___0 c - 2;
                goto :___1 _;
                :___0
                `set` ___0 8;
                :___1
            }
            goto :___3 _;
            :___2
            op ___1 b + 2;
            :___3
        }
        "#,
    };

    check_sugar! {parser,
        r#"
        a = 1 + (if b ? c : d);
        "#,
        r#"
        op a 1 + (
            take ___0 = $;
            goto :___0 b;
            `set` ___0 d;
            goto :___1 _;
            :___0
            `set` ___0 c;
            :___1
        );
        "#,
    };

}

#[test]
fn optional_jumpcmp_test() {
    let parser = TopLevelParser::new();

    check_sugar! {parser,
        r#"
        :x
        goto :x;
        "#,
        r#"
        :x
        goto :x _;
        "#,
    };

    check_sugar! {parser,
        r#"
        do {
            foo;
        } while;
        "#,
        r#"
        do {
            foo;
        } while _;
        "#,
    };

}

#[test]
fn control_block_test() {
    let parser = TopLevelParser::new();

    check_compile!{parser,
        r#"
        a;
        break {
            b;
            break;
            c;
        }
        d;
        break;
        "#,
        r#"
            a
            b
            jump 4 always 0 0
            c
            d
            jump 0 always 0 0
        "#,
    };

    check_compile!{parser,
        r#"
        a;
        break! {
            b;
            break;
            c;
        }
        d;
        break;
        "#,
        r#"
            a
            b
            jump 1 always 0 0
            c
            d
            jump 0 always 0 0
        "#,
    };

    check_compile!{parser,
        r#"
        a;
        continue {
            b;
            continue;
            c;
        }
        d;
        continue;
        "#,
        r#"
            a
            b
            jump 1 always 0 0
            c
            d
            jump 0 always 0 0
        "#,
    };

    check_compile!{parser,
        r#"
        a;
        continue! {
            b;
            continue;
            c;
        }
        d;
        continue;
        "#,
        r#"
            a
            b
            jump 4 always 0 0
            c
            d
            jump 0 always 0 0
        "#,
    };

    check_compile!{parser,
        r#"
        a;
        continue {
            b;
            break;
            c;
        }
        d;
        continue;
        "#,
        r#"
            a
            b
            jump 0 always 0 0
            c
            d
            jump 0 always 0 0
        "#,
    };

    check_compile!{parser,
        r#"
        a;
        break {
            b;
            continue;
            c;
        }
        d;
        break;
        "#,
        r#"
            a
            b
            jump 0 always 0 0
            c
            d
            jump 0 always 0 0
        "#,
    };

    check_compile!{parser,
        r#"
        a;
        break continue {
            b;
            continue;
            break;
            c;
        }
        d;
        "#,
        r#"
            a
            b
            jump 1 always 0 0
            jump 5 always 0 0
            c
            d
        "#,
    };

    check_compile!{parser,
        r#"
        a;
        continue break {
            b;
            continue;
            break;
            c;
        }
        d;
        "#,
        r#"
            a
            b
            jump 1 always 0 0
            jump 5 always 0 0
            c
            d
        "#,
    };

    check_compile!{parser,
        r#"
        a;
        continue! break! {
            b;
            break;
            continue;
            c;
        }
        d;
        "#,
        r#"
            a
            b
            jump 1 always 0 0
            jump 5 always 0 0
            c
            d
        "#,
    };
}

#[test]
fn number_test() {
    let parser = NumberParser::new();
    let mut meta = Meta::new();

    let nums: &[[&'static str; 2]] = { &[
        ["0", "0"],
        ["1", "1"],
        ["10", "10"],
        ["123.456", "123.456"],
        ["-123.456", "-123.456"],
        ["-12_3__.4_5_6_", "-123.456"],
        ["-10", "-10"],
        ["0x1b", "0x1b"],
        ["0x-2c", "0x-2c"],
        ["0b1001", "0b1001"],
        ["0b-1101", "0b-1101"],
        ["1e9", "1e9"],
        ["1e10", "1e10"],
        ["1e+10", "1e+10"],
        ["1e-10", "1e-10"],
        ["19e9", "19e9"],
        ["19e10", "19e10"],
        ["19e+10", "19e+10"],
        ["19e-10", "19e-10"],
        ["0_", "0"],
        ["1_", "1"],
        ["1_0", "10"],
        ["-1_0__", "-10"],
        ["0x1_b", "0x1b"],
        ["0x-2_c", "0x-2c"],
        ["0b10_01", "0b1001"],
        ["0b-11_01", "0b-1101"],
        ["1e9_", "1e9"],
        ["1e1_0", "1e10"],
        ["1e+1_0", "1e+10"],
        ["1e-1_0", "1e-10"],
        ["19e9_", "19e9"],
        ["19e1_0", "19e10"],
        ["19e+1_0", "19e+10"],
        ["19e-1_0", "19e-10"],
        ["1_e9_", "1e9"],
        ["1_e1_0", "1e10"],
        ["1_e+1_0", "1e+10"],
        ["1_e-1_0", "1e-10"],
        ["1_9__e9_", "19e9"],
        ["1_9__e1_0", "19e10"],
        ["1_9__e+1_0", "19e+10"],
        ["1_9__e-1_0", "19e-10"],
    ] };
    let fails: &[&'static str] = { &[
        "0o25",
        "1e\\3",
        "0o-25",
        "- 5",
        "_23",
        "1._5",
        "_1.5",
        "0b12",
        "1.2e9",
        "1e",
        "0x1be+9",
        "0x1be+0x9",
        "2e8.5",
    ] };

    for &[src, dst] in nums {
        assert_eq!(
            parser.parse(&mut meta, src),
            Ok(dst.into()), // alloc, 懒得优化
            "assert failed. ({:?} -> {:?})",
            src,
            dst,
        );
    }

    for &src in fails {
        assert!(
            parser.parse(&mut meta, src).is_err(),
            "assert failed. ({:?})",
            src
        );
    }
}

#[test]
fn cmper_test() {
    let parser = TopLevelParser::new();

    check_compile_eq!{parser,
        r#"
        const Cmp = goto(_0 < _1);
        do {} while ({const _0 = a; const _1 = b;} => Cmp);
        "#,
        r#"
        do {} while a < b;
        "#,
    };

    check_compile_eq!{parser,
        r#"
        const Cmp = goto(_0 < _1);
        do {} while !({const _0 = a; const _1 = b;} => Cmp);
        "#,
        r#"
        do {} while !a < b;
        "#,
    };

    check_compile_eq!{parser,
        r#"
        const Cmp = goto(_0 < _1);
        do {} while ({const _0 = a; const _1 = b;} => !Cmp);
        "#,
        r#"
        do {} while !a < b;
        "#,
    };

    check_sugar! {parser,
        r#"
        const Cmp = goto(_0 < _1);
        do {} while ({const _0 = a; const _1 = b;} => !Cmp);
        "#,
        r#"
        const Cmp = goto(_0 < _1);
        do {} while {const _0 = a; const _1 = b;} => !Cmp;
        "#,
    };

    check_compile_eq!{parser,
        r#"
        const Cmp = goto(!_0 < _1);
        do {} while ({const _0 = a; const _1 = b;} => Cmp);
        "#,
        r#"
        do {} while !a < b;
        "#,
    };

    check_compile_eq!{parser,
        r#"
        const Cmp = goto(!_0 < _1);
        do {} while ({const _0 = a; const _1 = b;} => !Cmp);
        "#,
        r#"
        do {} while a < b;
        "#,
    };

    check_compile_eq!{parser,
        r#"
        const Cmp = goto(_0 < _1);
        do {} while !(x && ({const _0 = a; const _1 = b;} => Cmp));
        "#,
        r#"
        do {} while !(x && a < b);
        "#,
    };

    check_compile_eq!{parser,
        r#"
        do {} while !(x && ({const _0 = a; const _1 = b;} => goto(_0 < _1)));
        "#,
        r#"
        do {} while !(x && a < b);
        "#,
    };

    check_compile_eq!{parser,
        r#"
        do {} while !(x && ({const _0 = a; const _1 = b;} => !goto(_0 < _1)));
        "#,
        r#"
        do {} while !(x && !a < b);
        "#,
    };

    check_compile_eq!{parser,
        r#"
        const x.Cmp = goto(.. < 2);
        const Cmp = x->Cmp;
        do {} while Cmp;
        "#,
        r#"
        do {} while x < 2;
        "#,
    };

    check_compile_eq!{parser,
        r#"
        const x.New = (
            setres ..;
            const $.Cmp = goto(.. < 2);
        );
        const Cmp = x->New->Cmp;
        do {} while Cmp;
        "#,
        r#"
        do {} while x < 2;
        "#,
    };

    check_compile_eq!{parser,
        r#"
        const x.New = (
            setres ..;
            const $.Cmp = goto({print ..;} => .. < 2);
        );
        const Cmp = x->New->Cmp;
        do {} while Cmp;
        "#,
        r#"
        do {print x;} while x < 2;
        "#,
    };

    check_compile_eq!{parser,
        r#"
        const x.Cmp = goto(.. < 2);
        const Cmp = x->Cmp;
        do {} while Cmp;
        do {} while Cmp;
        "#,
        r#"
        do {} while x < 2;
        do {} while x < 2;
        "#,
    };

    check_compile_eq!{parser,
        r#"
        const x.Cmp = goto({
            do {} while;
        } => .. < 2);
        const Cmp = x->Cmp;
        do {} while Cmp;
        do {} while Cmp;
        "#,
        r#"
        do {do {} while;} while x < 2;
        do {do {} while;} while x < 2;
        "#,
    };

    check_compile_eq!{parser,
        r#"
        const x.Cmp = goto({
            do {} while;
        } => .. < 2);
        const Cmp = x->Cmp;
        do {} while Cmp;
        do {} while Cmp;
        print ..;
        "#,
        r#"
        do {do {} while;} while x < 2;
        do {do {} while;} while x < 2;
        print __;
        "#,
    };

    check_compile!{parser,
        r#"
        break ({match 1 2 3 => @ {}} => _);
        print _0 @;
        "#,
        r#"
            jump 0 always 0 0
            print _0
        "#,
    };

    check_compile!{parser,
        r#"
        break (=>[1 2 3] _);
        print _0 @;
        "#,
        r#"
            jump 0 always 0 0
            print _0
        "#,
    };

    check_compile!{parser,
        r#"
        break (=> a);
        print _0 @;
        "#,
        r#"
            jump 0 notEqual a false
            print _0
        "#,
    };

    check_compile!{parser,
        r#"
        break ( [1 2 3] a);
        print _0 @;
        "#,
        r#"
            jump 0 notEqual a false
            print _0
        "#,
    };

    check_compile!{parser,
        r#"
        break [1 2 3] a;
        print _0 @;
        "#,
        r#"
            jump 0 notEqual a false
            print _0
        "#,
    };

    check_compile!{parser,
        r#"
        break [1 2 3] [4 5] _0;
        print _0 @;
        "#,
        r#"
            jump 0 notEqual 4 false
            print _0
        "#,
    };

    check_compile!{parser,
        r#"
        break ( [1 2 3] [4 5] _0);
        print _0 @;
        "#,
        r#"
            jump 0 notEqual 4 false
            print _0
        "#,
    };

    check_compile!{parser,
        r#"
        break (=>[1 2 3] a);
        print _0 @;
        "#,
        r#"
            jump 0 notEqual a false
            print _0
        "#,
    };

    check_compile!{parser,
        r#"
        break (=>[a] _0 && _0);
        print _0 @;
        "#,
        r#"
            jump 2 equal a false
            jump 0 notEqual _0 false
            print _0
        "#,
    };

    check_compile!{parser,
        r#"
        break (=>[a] (_0 && _0));
        print _0 @;
        "#,
        r#"
            jump 2 equal a false
            jump 0 notEqual a false
            print _0
        "#,
    };

    check_compile!{parser,
        r#"
        break (=>[1 2 3] _);
        print _0 @;
        "#,
        r#"
            jump 0 always 0 0
            print _0
        "#,
    };

    check_compile!{parser,
        r#"
        break ( [1 2 3]_);
        print _0 @;
        "#,
        r#"
            jump 0 always 0 0
            print _0
        "#,
    };

    check_compile!{parser,
        r#"
        break [1 2 3]_;
        print _0 @;
        "#,
        r#"
            jump 0 always 0 0
            print _0
        "#,
    };

    check_sugar! {parser,
        r#"
            break (=>[1 2 3] _);
        "#,
        r#"
            break (=>[1 2 3] _);
        "#,
    };

    check_sugar! {parser,
        r#"
            break [1 2 3] _;
        "#,
        r#"
            break (=> [1 2 3] _);
        "#,
    };

    check_sugar! {parser,
        r#"
            goto( [a b] c);
        "#,
        r#"
            goto(=>[a b] c);
        "#,
    };
}

#[test]
fn mul_takes_test() {
    let parser = TopLevelParser::new();

    check_sugar! {parser,
        r#"
        take A=B C=D E=F G=I;
        "#,
        r#"
        inline {
            take A = B;
            take C = D;
            take E = F;
            take G = I;
        }
        "#,
    };
}

#[test]
fn mul_consts_test() {
    let parser = TopLevelParser::new();

    check_sugar! {parser,
        r#"
        const A=B C=D E=F G=I;
        "#,
        r#"
        inline {
            const A = B;
            const C = D;
            const E = F;
            const G = I;
        }
        "#,
    };

    // label
    check_sugar! {parser,
        r#"
        const A=(:m goto :m;) C=(if x {});
        "#,
        r#"
        inline {
            const A = (:m goto :m;);
            const C = (if x {});
        }
        "#,
    };
}

#[test]
fn switch_ignored_id_test() {
    let parser = TopLevelParser::new();

    check_sugar! {parser,
        r#"
        switch x {
            case: foo;
            case: bar;
            case: baz;
        }
        "#,
        r#"
        switch x {
            case 0: foo;
            case 1: bar;
            case 2: baz;
        }
        "#,
    };

    check_sugar! {parser,
        r#"
        switch x {
            case 1: foo;
            case: bar;
            case: baz;
        }
        "#,
        r#"
        switch x {
            case 1: foo;
            case 2: bar;
            case 3: baz;
        }
        "#,
    };

    check_sugar! {parser,
        r#"
        switch x {
            case: foo;
            case 2: bar;
            case: baz;
        }
        "#,
        r#"
        switch x {
            case 0: foo;
            case 2: bar;
            case 3: baz;
        }
        "#,
    };

    check_sugar! {parser,
        r#"
        switch x {
            case 0 2 4: foo;
            case: bar;
            case: baz;
        }
        "#,
        r#"
        switch x {
            case 0 2 4: foo;
            case 5: bar;
            case 6: baz;
        }
        "#,
    };
}

#[test]
fn switch_append_tail_once_test() {
    let parser = TopLevelParser::new();

    // switch填充行仅最后一个进行填充

    check_sugar! {parser,
        r#"
        switch x {
            break;
            case 6:
                foo;
            case 3:
                bar;
        }
        "#,
        r#"
        select x {
            print; # ignore
            print;
            { break; } # switch的封装以限制作用域
            { bar; break; }
            print;
            { break; }
            { foo; break; }
        }
        "#,
    };
}

#[test]
fn const_expr_eval_test() {
    let parser = TopLevelParser::new();

    check_compile_eq!{parser,
        r#"
        print 1.00;
        print `1.00`;
        print (1.00:);
        "#,
        r#"
        print 1.00;
        print 1.00;
        print 1.00;
        "#,
    };

    check_compile_eq!{parser,
        r#"
        print ($ = 1.00;);
        "#,
        r#"
        print 1;
        "#,
    };

    check_compile_eq!{parser,
        r#"
        take N = (op $ (op $ (op $ 1 + 2;) + 3;) + 4;);
        print N;
        "#,
        r#"
        print `10`;
        "#,
    };

    check_compile_eq!{parser,
        r#"
        take N = ($ = 1 + 2 + 3 + 4;);
        print N;
        "#,
        r#"
        print `10`;
        "#,
    };

    check_compile_eq!{parser,
        r#"
        take N = ($ = 1 << 10;);
        print N;
        "#,
        r#"
        take N = 1024;
        print N;
        "#,
    };

    check_compile_eq!{parser,
        r#"
        take N = ($ = 1.0 == 1;);
        print N;
        "#,
        r#"
        take N = 1;
        print N;
        "#,
    };

    check_compile_eq!{parser,
        r#"
        take N = (m: $ = 1.0 == 1;);
        print N;
        "#,
        r#"
        m = 1.0 == 1;
        print m;
        "#,
    };

    check_compile_eq!{parser,
        r#"
        print ($ = (`set` $ ($ = 1 + 1;);););
        "#,
        r#"
        print 2;
        "#,
    };

    // 非匿名句柄不优化
    check_compile_eq!{parser,
        r#"
        print (x: $ = 1 + 1;);
        "#,
        r#"
        op x 1 + 1;
        print x;
        "#,
    };

    check_compile_eq!{parser,
        r#"
        print ($ = log(0););
        "#,
        r#"
        print null;
        "#,
    };

    check_compile_eq!{parser,
        r#"
        print ($ = acos(0.5););
        print ($ = cos(60););
        "#,
        r#"
        print 60.00000000000001;
        print 0.5000000000000001;
        "#,
    };

    check_compile_eq!{parser,
        r#"
        print ($ = -3 // 2;);
        "#,
        r#"
        print -2;
        "#,
    };

    check_compile_eq!{parser,
        r#"
        const N=($ = -3 // 2;);
        const N1=($ = -N;);
        print N1;
        "#,
        r#"
        print 2;
        "#,
    };

    check_compile_eq!{parser,
        r#"
        const N=($ = -3 // 2;);
        take N1=($ = -N;);
        print N1;
        "#,
        r#"
        print 2;
        "#,
    };

    check_compile_eq!{parser,
        r#"
        const N=($ = -3 // 2;);
        take N=($ = -N;);
        print N;
        "#,
        r#"
        print 2;
        "#,
    };

    check_compile_eq!{parser,
        r#"
        print ($ = -2 - 3;);
        print ($ = max(3, 5););
        print ($ = min(0, -2););
        print ($ = min(null, -2););
        print ($ = abs(null););
        print ($ = null;);
        "#,
        r#"
        print -5;
        print 5;
        print -2;
        print -2;
        print 0;
        print null;
        "#,
    };

    check_compile_eq!{parser,
        r#"
        print ($ = 999999+0;);
        print ($ = 999999+1;);
        print ($ = -999999 - 0;);
        print ($ = -999999 - 1;);
        print ($ = 1 - 1;);
        "#,
        r#"
        print 999999;
        print 0xF4240;
        print -999999;
        print 0x-F4240;
        print 0;
        "#,
    };

    check_compile_eq!{parser,
        r#"
        print (*select 2 < 4 ? 5 : 8);
        print (*select 4 < 2 ? 5 : 8);
        "#,
        r#"
        print 5;
        print 8;
        "#,
    };

    check_compile_eq!{parser,
        r#"
        print (*select 2 < 4 ? a : 8);
        "#,
        r#"
        print ('select' $ lessThan 2 4 a 8);
        "#,
    };
}

#[test]
fn string_escape_test() {
    let parser = VarParser::new();

    let true_case = [
        ("\n", r"\n"),
        ("\r\n", r"\n"),
        ("\r\nab", r"\nab"),
        ("ab\ncd", r"ab\ncd"),
        ("ab  \ncd", r"ab  \ncd"),
        ("ab  \n  cd", r"ab  \n  cd"),
        ("ab  \\\ncd", r"ab  cd"),
        ("ab\\\n  cd", r"abcd"),
        ("ab\\\n \\ cd", r"ab cd"),
        ("ab\\\r\n \\ cd", r"ab cd"),
        ("ab\\\n \\  cd", r"ab  cd"),
        ("ab\\\n\\ cd", r"ab cd"),
        ("ab\\\n\n\\ cd", r"ab\n cd"),
        ("ab\\\n\\\n\\ cd", r"ab cd"),
        ("\nab", r"\nab"),
        ("\\\nab", r"ab"),
        ("a\\\\b", r"a\b"),
        ("m\\\\n", r"m\[]n"),
        ("[red]\\[red]", r"[red][[red]"),
        ("你好", r"你好"),
    ];
    let false_case = [
        "a\rb",
        "a\n\rb",
        "a\n\\ b",
        r"ab\r",
        r"ab\t",
        r"ab\\\",
        r"\ ab",
        r" \ ab",
        r" \  ab",
        r"a\bb",
    ];
    let quoted = |s| format!("\"{s}\"");
    for (src, dst) in true_case {
        assert_eq!(parse!(parser, &quoted(src)), Ok(quoted(dst).into()));
    }
    for src in false_case {
        let src = quoted(src);
        let res = parse!(parser, &src);
        assert!(res.is_err(), "fail: {:?} -> {:?}", res, src)
    }
}

#[test]
fn match_test() {
    let parser = TopLevelParser::new();

    check_compile!{parser,
        r#"
        const Foo = (
            inline@{
                print @;
            }
        );
        take Foo[a b c];
        "#,
        r#"
            print a
            print b
            print c
        "#,
    };

    check_compile!{parser,
        r#"
        const Foo = (
            print @;
        );
        take Foo[a b c];
        "#,
        r#"
            print a
            print b
            print c
        "#,
    };

    check_compile!{@with_source parser,
        r#"
        take Foo[a b c];
        print @;
        "#,
        r#""#
    }.hit_log(1);

    check_compile!{parser,
        r#"
        match a b c {}
        print @;
        "#,
        r#"
        "#,
    };

    check_compile!{parser,
        r#"
        match a b c { @{} }
        print @;
        "#,
        r#"
            print a
            print b
            print c
        "#,
    };

    // 作用域测试
    check_compile!{parser,
        r#"
        {
            match a b c { @{} }
        }
        print @;
        "#,
        r#"
        "#,
    };

    // 作用域测试
    check_compile!{parser,
        r#"
        match a b c { @{} }
        inline@{
            print @;
        }
        print end;
        print @;
        "#,
        r#"
            print a
            print b
            print c
            print end
            print a
            print b
            print c
        "#,
    };

    // 作用域测试
    check_compile!{parser,
        r#"
        match a b c { @{} }
        inline 2@{
            foo @;
        }
        print end;
        print @;
        "#,
        r#"
            foo a b
            foo c
            print end
            print a
            print b
            print c
        "#,
    };

    check_compile!{parser,
        r#"
        const C = 1;
        match a b c { @{} }
        inline*C@{
            foo @;
        }
        print end;
        print @;
        "#,
        r#"
            foo a
            foo b
            foo c
            print end
            print a
            print b
            print c
        "#,
    };

    check_compile!{parser,
        r#"
        const C = 1;
        match a b c { @{} }
        inline*C@{
            foo @;
            Builtin.StopRepeat!;
        }
        print end;
        print @;
        "#,
        r#"
            foo a
            print end
            print a
            print b
            print c
        "#,
    };

    check_compile!{parser,
        r#"
        const C = 2;
        match a b c { @{} }
        inline*C@{
            foo @;
        }
        print end;
        print @;
        "#,
        r#"
            foo a b
            foo c
            print end
            print a
            print b
            print c
        "#,
    };

    check_compile!{parser,
        r#"
        match a b c { __ @{} }
        print @;
        "#,
        r#"
            print b
            print c
        "#,
    };

    check_compile!{parser,
        r#"
        match a b c {
            X Y {}
            @{}
        }
        print @;
        "#,
        r#"
            print a
            print b
            print c
        "#,
    };

    check_compile!{parser,
        r#"
        match a b {
            X Y {}
            @{}
        }
        print @;
        "#,
        r#"
        "#,
    };

    check_compile!{parser,
        r#"
        match a b {
            X Y {}
            @{}
        }
        print X Y;
        "#,
        r#"
            print a
            print b
        "#,
    };

    check_compile!{parser,
        r#"
        const Foo = (
            match @ {
                Fst @ {
                    print Fst;
                    take Foo[@];
                }
                {
                    print end;
                }
            }
        );
        take Foo[a b c];
        "#,
        r#"
            print a
            print b
            print c
            print end
        "#,
    };

    check_compile!{parser,
        r#"
        const Foo = (
            match @ {
                @ Lst {
                    print Lst;
                    take Foo[@];
                }
                {
                    print end;
                }
            }
        );
        take Foo[a b c];
        "#,
        r#"
            print c
            print b
            print a
            print end
        "#,
    };

    check_compile!{parser,
        r#"
        const Foo = (
            match @ {
                Fst @ Lst {
                    print Fst;
                    print Lst;
                    take Foo[@];
                }
                Mid {
                    print Mid;
                }
                {
                    print end;
                }
            }
        );
        take Foo[a b c d e];
        "#,
        r#"
            print a
            print e
            print b
            print d
            print c
        "#,
    };

    check_compile!{parser,
        r#"
        const Foo = (
            match @ {
                Fst @ Lst {
                    print Fst;
                    print Lst;
                    take Foo[@];
                }
                Mid {
                    print Mid;
                }
                {
                    print end;
                }
            }
        );
        take Foo[a b c d e f];
        "#,
        r#"
            print a
            print f
            print b
            print e
            print c
            print d
            print end
        "#,
    };

    check_compile!{parser,
        r#"
        const Foo = ( # 循环展开版本
            inline@{
                match @ {
                    [1] {
                        print one;
                    }
                    [2] {
                        print two;
                    }
                    N:[3 4] {
                        print three_or_four N;
                    }
                    N {
                        print other N;
                    }
                }
            }
        );
        take Foo[1 2 3 4 5 6];
        "#,
        r#"
            print one
            print two
            print three_or_four
            print 3
            print three_or_four
            print 4
            print other
            print 5
            print other
            print 6
        "#,
    };

    check_compile!{parser,
        r#"
        const Foo = ( # 右递归版本
            match @ {
                [1] @ {
                    print one;
                    take Foo[@];
                }
                [2] @ {
                    print two;
                    take Foo[@];
                }
                N:[3 4] @ {
                    print three_or_four N;
                    take Foo[@];
                }
                N @ {
                    print other N;
                    take Foo[@];
                }
            }
        );
        take Foo[1 2 3 4 5 6];
        "#,
        r#"
            print one
            print two
            print three_or_four
            print 3
            print three_or_four
            print 4
            print other
            print 5
            print other
            print 6
        "#,
    };

    check_compile!{parser,
        r#"
        const Foo = ( # 左递归版本
            match @ {
                @ [1] {
                    take Foo[@];
                    print one;
                }
                @ [2] {
                    take Foo[@];
                    print two;
                }
                @ N:[3 4] {
                    take Foo[@];
                    print three_or_four N;
                }
                @ N {
                    take Foo[@];
                    print other N;
                }
            }
        );
        take Foo[1 2 3 4 5 6];
        "#,
        r#"
            print one
            print two
            print three_or_four
            print 3
            print three_or_four
            print 4
            print other
            print 5
            print other
            print 6
        "#,
    };

    check_compile!{parser,
        r#"
        const Eq = ( # 引用前部匹配
            match @ {
                A B {
                    match B {
                        [A] {
                            print 'equal' A;
                        }
                        __ {
                            print not_equal A B;
                        }
                    }
                }
            }
        );
        take Eq[a a];
        take Eq[a b];
        "#,
        r#"
            print equal
            print a
            print not_equal
            print a
            print b
        "#,
    };

    check_compile!{parser,
        r#"
        const Foo = (
            # 验证滞后const,
            # 对于args应该在传参时就进行const而不是使用时才进行处理
            # 所以应该得到两个5而不是一个5一个10
            take X = 10;

            take A = _0;
            match @ { B {} }
            print A B;
        );
        const X = 5;
        take Foo[X];
        "#,
        r#"
            print 5
            print 5
        "#,
    };

    check_compile!{parser,
        r#"
        # 验证其并不会连锁追溯
        const Foo = (
            take A = _0;
            print _0 A;
        );
        const X = 2;
        const Y = `X`;
        print Y;
        take Foo[Y];
        "#,
        r#"
            print X
            print X
            print X
        "#,
    };

    check_compile!{parser,
        r#"
        # 验证其并不会连锁追溯
        const Foo = (
            take A = _0;
            print A;
            match @ { B {} }
            print A B;
        );
        const X = 2;
        const Y = `X`;
        print Y;
        take Foo[Y];
        "#,
        r#"
            print X
            print X
            print X
            print X
        "#,
    };

    check_compile!{parser,
        r#"
        # 验证其并不会连锁追溯
        const X = 2;
        match `X` {
            Y:[`X`] {
                print Y;
            }
            Y {
                print "fail" Y;
            }
        }
        "#,
        r#"
            print X
        "#,
    };

    check_compile!{parser,
        r#"
        const 3 = 0;
        const X = 2;
        const Y = `3`;
        match X Y 4 {
            A:[`2`] B:[`3`] C:[4] {
                print A B C;
            }
        }
        "#,
        r#"
            print 2
            print 3
            print 4
        "#,
    };

    check_compile!{parser,
        r#"
        # 关于二次追溯
        {
            const X = 2;
            const Y = `X`;
            const F = (
                print _0 @;
            );
            take F[Y];
        }
        {
            const X = 2;
            const F = (
                const Y = `X`;
                print _0 @;
            );
            take F[Y];
        }
        "#,
        r#"
            print X
            print X
            print Y
            print Y
        "#,
    };

    check_compile!{parser,
        r#"
        # 关于二次追溯
        {
            const X = 2;
            const Y = `X`;
            const F = (
                print _0;
                match @ {
                    V { print V; }
                }
            );
            take F[Y];
        }
        {
            const X = 2;
            const F = (
                const Y = `X`;
                print _0;
                match @ {
                    V { print V; }
                }
            );
            take F[Y];
        }
        "#,
        r#"
            print X
            print X
            print Y
            print Y
        "#,
    };

    check_compile!{parser,
        r#"
        # 关于参数展开在match和repeat中对绑定者的丢失
        const a.F = (nouse:
            print ..;
        );
        const Do = (
            const F = _0;

            take Res = $;
            const Res.F1 = (
                match @ {
                    V {
                        print V;
                    }
                }
                print @;
                print _0;
            );
            take Res.F1[F];
        );
        take F = Builtin.BindHandle2[`a` `F`];
        take Builtin.Const[`F` F];
        take Do[F];
        "#,
        r#"
            print a
            print nouse
            print a
            print nouse
            print a
            print nouse
        "#,
    };

    check_sugar! {parser,
        r#"
         inline@ # align .....
            A B *C
        {
            print A B C;
        }
        "#,
        r#"
        inline 3@{
            const match @ {
                A B *C {
                    print A B C;
                }
            }
        }
        "#,
    };

    check_sugar! {parser,
        r#"
        match x @ y => a @ b {
            body;
        }
        "#,
        r#"
        match x @ y {
            a @ b {
                body;
            }
        }
        "#,
    };

    check_sugar! {parser,
        r#"
        const match x @ y => a @ b {
            body;
        }
        "#,
        r#"
        const match x @ y {
            a @ b {
                body;
            }
        }
        "#,
    };

    check_sugar! {parser,
        r#"
        const match x @ y => *a @ [b] {
            body;
        }
        "#,
        r#"
        const match x @ y {
            *a @ [b] {
                body;
            }
        }
        "#,
    };

    check_compile!{parser,
        r#"
        match 1 2 => @ {
            print _0 _1 @;
        }
        print _0 _1 @;
        "#,
        r#"
            print 1
            print 2
            print 1
            print 2
            print 1
            print 2
            print 1
            print 2
        "#,
    };

    check_compile!{parser,
        r#"
        const match 1 2 => @ {
            print _0 _1 @;
        }
        print _0 _1 @;
        "#,
        r#"
            print 1
            print 2
            print 1
            print 2
            print 1
            print 2
            print 1
            print 2
        "#,
    };

    check_compile!{parser,
        r#"
        const Foo = (match @ {
            { print 0; }
            _ { print 1; }
            _ _ { print 2; }
        });
        take Foo[] Foo[3] Foo[3 3];
        "#,
        r#"
            print 0
            print 1
            print 2
        "#,
    };

    check_compile!{parser,
        r#"
        const Foo = (match @ {
            $_ { print 1; }
        });
        print Foo[6];
        "#,
        r#"
            print 1
            print 6
        "#,
    };

    check_compile!{parser,
        r#"
        const Foo = (match @ {
            $X { print 8 X 8; }
        });
        print Foo[6];
        "#,
        r#"
            print 8
            print 6
            print 8
            print 6
        "#,
    };

    check_compile!{parser,
        r#"
        const Foo = (match @ {
            $X $Y { print 8 X 8; }
        });
        print Foo[6 9];
        "#,
        r#"
            print 8
            print 6
            print 8
            print 9
        "#,
    };

    check_compile!{parser,
        r#"
        const Foo = (match @ {
            X $Y { print 8 X 8; }
        });
        print Foo[6 9];
        "#,
        r#"
            print 8
            print 6
            print 8
            print 9
        "#,
    };

    check_compile!{parser,
        r#"
        const Foo = (match @ {
            $X Y { print 8 X 8; }
        });
        print Foo[6 9];
        "#,
        r#"
            print 8
            print 6
            print 8
            print 6
        "#,
    };

    check_compile!{parser,
        r#"
        const Foo = (match @ {
            X Y { print 8 X 8; }
        });
        print Foo[6 9];
        "#,
        r#"
            print 8
            print 6
            print 8
            print __2
        "#,
    };

    check_compile!{parser,
        r#"
        const match (arg;) => @ {}
        match (a;) @ (b;) => @ {}
        "#,
        r#"
            a
            arg
            b
        "#,
    };

    // 空语句不添加
    check_compile!{parser,
        r#"
        match a b c => @ {}
        @;
        match => @ {}
        @;
        match d e f => @ {}
        @;
        "#,
        r#"
            a b c
            d e f
        "#,
    };
}

#[test]
fn param_comma_sugar_test() {
    let parser = TopLevelParser::new();

    check_sugar! {parser,
        "print a, b;",
        "print a b;",
    };
    check_sugar! {parser,
        "print a, b,;",
        "print a b;",
    };
    check_sugar! {parser,
        "print, a, b,;",
        "print a b;",
    };
    check_sugar! {parser,
        "take Foo[1, 2, 3];",
        "take Foo[1 2 3];",
    };
    check_sugar! {parser,
        "take Foo[1 2, 3];",
        "take Foo[1 2 3];",
    };
    check_sugar! {parser,
        "take {A &B:C} = (c;)  M;",
        "take {A &B:C} = (c;), M,;",
    };
    check_sugar! {parser,
        "take {A &B:C} = (c;),;",
        "take {A &B:C} = (c;);",
    };
    check_sugar! {parser,
        "take Foo[1 @ 2, 3];",
        "take Foo[1 @ 2 3];",
    };
    check_sugar! {parser,
        "take Foo[1, @ 2, 3];",
        "take Foo[1 @ 2 3];",
    };
    check_sugar! {parser,
        "take Foo[1, @, 2, 3];",
        "take Foo[1 @ 2 3];",
    };
    check_sugar! {parser,
        "take Foo[1, @, 2, 3,];",
        "take Foo[1 @ 2 3];",
    };
    check_sugar! {parser,
        "take Foo[1, @,];",
        "take Foo[1 @];",
    };
    check_sugar! {parser,
        "take Foo[1,];",
        "take Foo[1];",
    };
    check_sugar! {parser,
        "Foo! @,;",
        "Foo! @;",
    };
    check_sugar! {parser,
        "Foo! 1,;",
        "Foo! 1;",
    };
    check_sugar! {parser,
        "Foo! 1,@;",
        "Foo! 1 @;",
    };
    check_sugar! {parser,
        "Foo! 1,@,;",
        "Foo! 1 @;",
    };
    check_sugar! {parser,
        "Foo! *1,@,;",
        "Foo! *1 @;",
    };
    check_sugar! {parser,
        "Foo! *1++,@,;",
        "Foo! *1++ @;",
    };
    check_sugar! {parser,
        "Foo! 1++,@,;",
        "Foo! 1++ @;",
    };
    check_sugar! {parser,
        "Foo! @,1++,;",
        "Foo! @,1++;",
    };
    assert!(parse!(parser, "Foo!;").is_ok());
    assert!(parse!(parser, "take Foo[];").is_ok());

    assert!(parse!(parser, "Foo!,;").is_err());
    assert!(parse!(parser, "take Foo[,];").is_err());
    assert!(parse!(parser, "Foo!,@;").is_err());
    assert!(parse!(parser, "take Foo[,@];").is_err());
    assert!(parse!(parser, "Foo!,2;").is_err());
    assert!(parse!(parser, "take Foo[,2];").is_err());
    assert!(parse!(parser, "Foo!,2 @;").is_err());
    assert!(parse!(parser, "take Foo[,2 @];").is_err());
}

#[test]
fn const_match_test() {
    let parser = TopLevelParser::new();

    check_compile!{parser,
        r#"
        const match {
            {
                print empty;
            }
        }
        "#,
        r#"
            print empty
        "#,
    };

    check_compile!{parser,
        r#"
        match 1 2 { @ {} }
        const match {
            @ {
                print @;
            }
        }
        "#,
        r#"
        "#,
    };

    check_compile!{parser,
        r#"
        const Val = (res:
            print taked;
        );
        const match Val {
            V {
                print 1 V;
            }
        }
        "#,
        r#"
            print 1
            print taked
            print res
        "#,
    };

    check_compile!{parser,
        r#"
        const Val = (res:
            print taked;
        );
        const match Val {
            *V {
                print 1 V;
            }
        }
        "#,
        r#"
            print taked
            print 1
            print res
        "#,
    };

    check_compile!{parser,
        r#"
        const Val = 2;
        const match `Val` {
            *V {
                print 1 V;
            }
        }
        "#,
        r#"
            print 1
            print Val
        "#,
    };

    check_compile!{parser,
        r#"
        const Val = 2;
        const match Val {
            *V:[1] {
                print 1 V;
            }
            *V:[(1:print take1;) 2 (3: print err;)] { # lazy
                print x2 V;
            }
        }
        "#,
        r#"
            print take1
            print x2
            print 2
        "#,
    };

    check_compile!{parser,
        r#"
        const Val = 2;
        const match Val {
            V:[1] {
                print 1 V;
            }
            V:[(1:print take1;) 2 (3: print err;)] { # lazy
                print x2 V;
            }
        }
        "#,
        r#"
            print take1
            print x2
            print 2
        "#,
    };

    check_compile!{parser,
        r#"
        const Val = 2;
        const match Val {
            [?(0: print _0;)] {
                print unreachable;
            }
        }
        "#,
        r#"
            print 2
        "#,
    };

    check_compile!{parser,
        r#"
        const Val = 2;
        const match Val {
            [?(`__`: print _0;)] {
                print x;
            }
        }
        "#,
        r#"
            print 2
            print x
        "#,
    };

    check_compile!{parser,
        r#"
        const Val = 2;
        const match Val {
            [?(1: print _0;)] {
                print x;
            }
        }
        "#,
        r#"
            print 2
            print x
        "#,
    };

    check_compile!{parser,
        r#"
        const Val = 2;
        const match Val {
            [?(false: print _0;)] {
                print x;
            }
        }
        "#,
        r#"
            print 2
            print x
        "#,
    };

    check_compile!{parser,
        r#"
        const match 2 {
            [2] {
                print only;
            }
            [2] {
                print unreachable;
            }
        }
        "#,
        r#"
            print only
        "#,
    };

    check_compile!{parser,
        r#"
        const match (print err;) {
            [2] {
                print only;
            }
            [2] {
                print unreachable;
            }
            __ {
                print default;
            }
        }
        "#,
        r#"
            print default
        "#,
    };

    check_compile!{parser,
        r#"
        const match (%2: print x;%)->$ {
            [2] {
                print only;
            }
            [2] {
                print unreachable;
            }
        }
        "#,
        r#"
            print x
            print only
        "#,
    };

    check_compile!{parser,
        r#"
        const match (2: print x;) {
            _ {
                print only;
            }
            [2] {
                print unreachable;
            }
        }
        "#,
        r#"
            print only
        "#,
    };

    check_compile!{parser,
        r#"
        const match (2: print x;) {
            *_ {
                print only;
            }
            [2] {
                print unreachable;
            }
        }
        "#,
        r#"
            print x
            print only
        "#,
    };

    check_compile!{parser,
        r#"
        const match (2: print x;) {
            [*] {
                print only;
            }
            [2] {
                print unreachable;
            }
        }
        "#,
        r#"
            print x
            print only
        "#,
    };

    check_compile!{parser,
        r#"
        const match (2: print x;) {
            [*3] {
                print unreachable;
            }
            [*2 4] {
                print only;
            }
            [2] {
                print unreachable;
            }
        }
        "#,
        r#"
            print x
            print x
            print only
        "#,
    };

    check_compile!{parser,
        r#"
        const match (2: print x;) 3 {
            [*2] [2] {
                print unreachable;
            }
            [*2] [3] {
                print only;
            }
        }
        "#,
        r#"
            print x
            print x
            print only
        "#,
    };

    check_compile!{parser,
        r#"
        const match (2: print x;) 3 {
            *_ [2] {
                print unreachable;
            }
            *_ [3] {
                print only;
            }
        }
        "#,
        r#"
            print x
            print only
        "#,
    };

    check_compile!{parser,
        r#"
        match A { @ {} }
        const A = 2;
        const match @ {
            [`A`] {
                print yes;
            }
            @ {
                print no @;
            }
        }
        "#,
        r#"
            print yes
        "#,
    };

    check_compile!{parser,
        r#"
        foo (const match (h: print taked;) {
            $_ {
                print body;
            }
        });
        "#,
        r#"
            print taked
            print body
            foo h
        "#,
    };

    check_compile!{parser,
        r#"
        foo (const match (h: print taked;) {
            $*M {
                print body M;
            }
        });
        "#,
        r#"
            print taked
            print body
            print h
            foo h
        "#,
    };

    check_compile!{parser,
        r#"
        foo (const match (h: print taked;) {
            $*M {
                setres M;
                print body M;
            }
        });
        "#,
        r#"
            print taked
            print body
            print h
            foo h
        "#,
    };

    check_compile!{parser,
        r#"
        foo (const match (h: print taked;) {
            $*M:[h] {
                print body M;
            }
        });
        "#,
        r#"
            foo __0
        "#,
    };

    check_compile!{parser,
        r#"
        foo (const match (h: print taked;) {
            $*M:[*h] {
                setres M;
                print body M;
            }
        });
        "#,
        r#"
            print taked
            print taked
            print body
            print h
            foo h
        "#,
    };

    check_compile!{parser,
        r#"
        foo (const match (h: print taked;) {
            $M:[*h] {
                setres M;
                print body M;
            }
        });
        "#,
        r#"
            print taked
            print taked
            print taked
            print body
            print taked
            print h
            foo h
        "#,
    };

    check_compile!{parser,
        r#"
        foo (const match (h: print taked;) {
            $M {
                setres M;
                print body M;
            }
        });
        "#,
        r#"
            print taked
            print taked
            print body
            print taked
            print h
            foo h
        "#,
    };

    check_compile!{parser,
        r#"
        const match (arg;) => @ {}
        const match (a;) @ (b;) => A @ *B {}
        "#,
        r#"
            b
        "#,
    };

    check_compile!{parser,
        r#"
        const match (arg;) => @ {}
        const match (a;) @ (b;) => *A @ *B {}
        "#,
        r#"
            a
            b
        "#,
    };

    check_compile!{parser,
        r#"
        const match (arg;) => @ {}
        const match (a;) @ (b;) => *A *@ *B {}
        "#,
        r#"
            a
            arg
            b
        "#,
    };

    check_compile!{parser,
        r#"
        const match (arg;) => @ {}
        const match (a;) @ (b;) => A *@ *B {}
        "#,
        r#"
            arg
            b
        "#,
    };

    check_compile!{parser,
        r#"
        const match (arg;) => @ {}
        const match (a;) @ (b;) => A *@ *B { end; }
        "#,
        r#"
            arg
            b
            end
        "#,
    };

    check_compile!{parser,
        r#"
        const match (8: arg;) => @ {}
        const match (a @;) @ (b @;) => *A *@ *B { end; }
        "#,
        r#"
            arg
            a 8
            arg
            arg
            b 8
            end
        "#,
    };

    // 测试守卫作用域
    check_compile!{parser,
        r#"
        const match 1 { [?_0 == 1] { x; } _ { y; }}
        print _0 @;
        "#,
        r#"
            x
            print _0
        "#,
    };

    // 无限重复块
    check_compile!{parser,
        r#"
        take*I = 0;
        inline 0@ {
            match I { [5] { Builtin.StopRepeat!; } _ {
                print I;
                take*I = I + 1;
            } }
        }
        "#,
        r#"
            print 0
            print 1
            print 2
            print 3
            print 4
        "#,
    };

    // 不重设参数情况
    check_compile!{parser,
        r#"
        take*I = 0;
        inline 0@ {
            match I { [3] { Builtin.StopRepeat!; } _ {
                match @ I => @ {}
                foo @;
                take*I = I + 1;
            } }
        }
        bar @; # 都保留了参数作用域
        baz I; # 并不具有常量作用域
        "#,
        r#"
            foo 0
            foo 0 1
            foo 0 1 2
            bar
            baz 3
        "#,
    };

    // 重设参数情况
    check_compile!{parser,
        r#"
        take*I = 0;
        inline*0@ {
            match I { [3] { Builtin.StopRepeat!; } _ {
                match @ I => @ {}
                foo @;
                take*I = I + 1;
            } }
        }
        bar @; # 都保留了参数作用域
        baz I; # 并不具有常量作用域
        "#,
        r#"
            foo 0
            foo 1
            foo 2
            bar
            baz 3
        "#,
    };
}

#[test]
fn value_bind_of_constkey_test() {
    let parser = TopLevelParser::new();

    check_compile!{parser,
        r#"
        take X = ();
        take X.Y = 2;
        print X.Y;
        "#,
        r#"
            print 2
        "#,
    };

    check_compile!{parser,
        r#"
        take X = ();
        {
            take X.Y = 2; # to global
        }
        print X.Y;
        "#,
        r#"
            print 2
        "#,
    };

    check_compile!{parser,
        r#"
        take X = ();
        {
            const X.Y = (
                :x
                goto :x;
            );
        }
        take X.Y;
        take X.Y;
        "#,
        r#"
            jump 0 always 0 0
            jump 1 always 0 0
        "#,
    };
}

#[test]
fn const_value_expand_binder_test() {
    let parser = TopLevelParser::new();

    check_compile!{parser,
        r#"
        print ..;
        "#,
        r#"
            print __
        "#,
    };

    check_compile!{parser,
        r#"
        const X.Y = (
            print ..;
        );
        take X.Y;
        "#,
        r#"
            print X
        "#,
    };

    check_compile!{parser,
        r#"
        const X.Y = (
            const Foo = (
                print ..;
            );
            take Foo;
        );
        take X.Y;
        "#,
        r#"
            print X
        "#,
    };

    check_compile!{parser,
        r#"
        const X.Y = (
            const Foo.Bar = (
                print ..;
            );
            take Foo.Bar;
        );
        take X.Y;
        "#,
        r#"
            print Foo
        "#,
    };

    check_compile!{parser,
        r#"
        const X.Y = (
            const Foo.Bar = (
                print ..;
            );
            const F = Foo.Bar;
            take F;
        );
        take X.Y;
        "#,
        r#"
            print Foo
        "#,
    };

    check_compile!{parser,
        r#"
        const Box = (
            $.val = _0;
            const $.Add = (
                ...val = ...val + _0.val;
            );
        );
        take N = Box[2];
        take N1 = Box[3];
        take N.Add[N1];
        print ..;
        "#,
        r#"
            set __2 2
            set __6 3
            op add __2 __2 __6
            print __
        "#,
    };

    check_compile!{parser,
        r#"
        const Box = (
            $.val = _0;
            const $.Add = (
                const Self = ..;
                Self.val = Self.val + _0.val;
            );
        );
        take N = Box[2];
        take N1 = Box[3];
        take N.Add[N1];
        "#,
        r#"
            set __2 2
            set __6 3
            op add __2 __2 __6
        "#,
    };

    check_compile!{parser,
        r#"
        const Box = (
            $.val = _0;
            const $.Add = (
                take Self = ..;
                Self.val = Self.val + _0.val;
            );
        );
        take N = Box[2];
        take N1 = Box[3];
        take N.Add[N1];
        "#,
        r#"
            set __2 2
            set __6 3
            op add __2 __2 __6
        "#,
    };

    check_compile!{parser,
        r#"
        take a.B = 1;
        take a.B = 2;
        print a.B;
        "#,
        r#"
            print 2
        "#,
    };

    check_compile!{parser,
        r#"
        take a.N = 1;
        take b.N = 2;
        # 故意不实现的常量求值
        take X = ($ = a.N + b.N;);
        print X;
        "#,
        r#"
            op add __2 1 2
            print __2
        "#,
    };

    check_compile!{parser,
        r#"
        take a.N = 1;
        take b.N = 2;
        take A=a.N B=b.N;
        take X = ($ = A + B;);
        print X;
        "#,
        r#"
            print 3
        "#,
    };

    check_compile!{parser,
        r#"
        const a.X = (
            print ..;
        );
        const b.X = a.X;
        take a.X;
        take b.X;
        "#,
        r#"
            print a
            print a
        "#,
    };

    check_compile!{parser,
        r#"
        const A = a;
        const A.X = (
            print ..;
        );
        take A.X;
        "#,
        r#"
            print a
        "#,
    };

    check_compile!{parser,
        r#"
        const a.X = (
            print ..;
        );
        take Handle = Builtin.BindHandle2[`a` `X`];
        take Builtin.Const[`Handle` Handle];
        const b.X = Handle;
        take a.X;
        const a.X = 2;
        take b.X;
        "#,
        r#"
            print a
            print a
        "#,
    };
}

#[test]
fn builtin_func_test() {
    let parser = TopLevelParser::new();

    check_compile!{parser,
        r#"
        print Builtin.Type[x];
        "#,
        r#"
            print var
        "#,
    };

    check_compile!{parser,
        r#"
        print Builtin.Stringify[2];
        print Builtin.Stringify[x];
        print Builtin.Stringify["x"];
        "#,
        r#"
            print "2"
            print "x"
            print "x"
        "#,
    };

    check_compile!{@with_source parser,
        r#"
        print Builtin.Concat["abc" "def"];
        print Builtin.Status;
        print Builtin.Concat["abc" def];
        print Builtin.Status;
        "#,
        r#"
            print "abcdef"
            print 0
            print __
            print 2
        "#,
    }.hit_log(1);

    check_compile!{parser,
        r#"
        print Builtin.Type[()];
        print Builtin.Type[`m`];
        "#,
        r#"
            print dexp
            print var
        "#,
    };

    check_compile!{parser,
        r#"
        const A = ();
        const B = `emmm`;
        const C = A.B;
        const D = $;
        const E = ..;
        print Builtin.Type[A];
        print Builtin.Type[B];
        print Builtin.Type[C];
        print Builtin.Type[D];
        print Builtin.Type[E];
        "#,
        r#"
            print dexp
            print var
            print valuebind
            print resulthandle
            print binder
        "#,
    };

    check_compile!{parser,
        r#"
        const A.B = 2;
        print Builtin.Type[A.B];
        "#,
        r#"
            print valuebind
        "#,
    };

    check_compile!{@with_source parser,
        r#"
        const A = x;
        print Builtin.Info[A];
        print Builtin.Info[y];
        print Builtin.Err[A];
        print Builtin.Err[y];
        "#,
        r#"
            print x
            print y
            print x
            print y
        "#,
    }.hit_log(4);

    check_compile!{parser,
        r#"
        const A = x.y;
        print Builtin.Unbind[A];
        print Builtin.Unbind[x.y];
        "#,
        r#"
            print y
            print y
        "#,
    };

    check_compile!{parser,
        r#"
        {
            const Name = `X`;
            const Value = (h:);
            take Builtin.Const[Name Value];
            print X;
        }

        {
            { # 击穿
                const Name = `Y`;
                const Value = (i:);
                match Name Value { @ {} }
                take Builtin.Const;
                print Y;
            }
            print Y;
        }
        "#,
        r#"
            print h
            print i
            print i
        "#,
    };

    check_compile!{parser,
        r#"
        const Value = (x:);
        const Binded = Value.x;
        print Builtin.Type[Binded];
        take Builtin.Binder[Res Binded];
        print Builtin.Type[Res];
        print Res;
        "#,
        r#"
            print valuebind
            print dexp
            print x
        "#,
    };

    check_compile!{parser,
        r#"
        const Foo = (
            take Builtin.SliceArgs[1 4];
            print @;
        );
        take Foo[0 1 2 3 4 5];
        "#,
        r#"
            print 1
            print 2
            print 3
        "#,
    };

    check_compile!{parser,
        r#"
        const Foo = (
            print Builtin.ArgsLen[];
        );
        print Builtin.ArgsLen[];
        match a b { @ {} }
        take Foo[0 1 2 3 4 5];
        print Builtin.ArgsLen[];
        "#,
        r#"
            print 0
            print 6
            print 2
        "#,
    };

    check_compile!{parser,
        r#"
        const Foo = (
            take Handle = Builtin.ArgsHandle[1];
            take Builtin.Const[Value Handle];
            print Handle Value;
        );
        take Foo[0 1 2 3 4 5];
        "#,
        r#"
            print __1
            print 1
        "#,
    };

    check_compile!{parser,
        r#"
        const F = ($ = 1+2;);
        print Builtin.EvalNum[F];
        "#,
        r#"
            print 3
        "#,
    };

    check_compile!{parser,
        r#"
        const F = ($ = 1+x;);
        print Builtin.EvalNum[F];
        "#,
        r#"
            print __
        "#,
    };

    check_compile!{parser,
        r#"
        const N = 2 S = "s" D = ();
        print Builtin.IsString[N];
        print Builtin.IsString[2];
        print Builtin.IsString[D];
        print Builtin.IsString[()];
        print Builtin.IsString[S];
        print Builtin.IsString["s"];
        "#,
        r#"
            print 0
            print 0
            print 0
            print 0
            print 1
            print 1
        "#,
    };

    check_compile!{parser,
        r#"
        const F = (
            {
                take Ref = Builtin.RefArg[0];
                match Ref { [113] { take Builtin.Exit[2]; } }
                take Builtin.Const[`N` Ref];
                print N;
            }
        );
        take F[113];
        "#,
        r#"
            print 113
        "#,
    };

    check_compile!{parser,
        r#"
        take Builtin.SetNoOp["set noop \\'noop\n\\'"];
        noop;
        "#,
        r#"
            set noop "noop\n"
        "#,
    };

    check_compile!{parser,
        r#"
        take Builtin.SetNoOp["set noop \\'noop\n\\'"];
        select x {
            print 1 2 3 4 5;
            print 1 2 3;
            print 1 2 3 4 5;
            print 1 2 3 4 5;
            print 1 2 3 4 5;
        }
        "#,
        r#"
            op mul __1 x 5
            op add @counter @counter __1
            print 1
            print 2
            print 3
            print 4
            print 5
            print 1
            print 2
            print 3
            jump 12 always 0 0
            set noop "noop\n"
            print 1
            print 2
            print 3
            print 4
            print 5
            print 1
            print 2
            print 3
            print 4
            print 5
            print 1
            print 2
            print 3
            print 4
            print 5
        "#,
    };

    check_compile!{parser,
        r#"
        take Builtin.SetNoOp['\"str\"'];
        select x {
            print 1 2 3 4 5;
            print 1 2 3;
            print 1 2 3 4 5;
            print 1 2 3 4 5;
            print 1 2 3 4 5;
        }
        "#,
        r#"
            op mul __1 x 5
            op add @counter @counter __1
            print 1
            print 2
            print 3
            print 4
            print 5
            print 1
            print 2
            print 3
            jump 12 always 0 0
            "str"
            print 1
            print 2
            print 3
            print 4
            print 5
            print 1
            print 2
            print 3
            print 4
            print 5
            print 1
            print 2
            print 3
            print 4
            print 5
        "#,
    };

    check_compile!{parser,
        r#"
        take Builtin.BindSep[x];
        print a.b;
        const a.b = 2;
        print a.b;
        print axb;
        take Builtin.BindSep[""];
        print axb;
        print c.d;
        const c.d = 3;
        print c.d;
        print cxd;
        print __2;
        "#,
        r#"
            print axb
            print 2
            print 2
            print 2
            print __2
            print 3
            print cxd
            print 3
        "#,
    };

    check_compile!{parser,
        r#"
        const match (a;) (b;) => @ {}
        start;
        Builtin.MakeSelect! (i:c;);
        end;
        "#,
        r#"
            start
            c
            op add @counter @counter i
            a
            b
            end
        "#,
    };

    check_compile!{parser,
        r#"
        print Builtin.Ord[a];
        "#,
        r#"
            print 97
        "#,
    };

    check_compile!{parser,
        r#"
        print Builtin.Ord["a"];
        "#,
        r#"
            print 97
        "#,
    };

    check_compile!{@with_source parser,
        r#"
        print Builtin.Ord[ab];
        "#,
        r#"
            print __
        "#,
    }.hit_log(1);

    check_compile!{parser,
        r#"
        print Builtin.Ord['\n'];
        "#,
        r#"
            print 10
        "#,
    };

    check_compile!{parser,
        r#"
        print Builtin.Ord['\t'];
        "#,
        r#"
            print 9
        "#,
    };

    check_compile!{parser,
        r#"
        print Builtin.Ord['\r'];
        "#,
        r#"
            print 13
        "#,
    };

    check_compile!{parser,
        r#"
        print Builtin.Ord['\e'];
        "#,
        r#"
            print 27
        "#,
    };

    check_compile!{parser,
        r#"
        print Builtin.Ord['"'];
        "#,
        r#"
            print 34
        "#,
    };

    check_compile!{parser,
        r#"
        print Builtin.Ord["'"];
        "#,
        r#"
            print 39
        "#,
    };

    check_compile!{@with_source parser,
        r#"
        print Builtin.Ord[""];
        "#,
        r#"
            print __
        "#,
    }.hit_log(1);

    check_compile!{@with_source parser,
        r#"
        print Builtin.Ord["ab"];
        "#,
        r#"
            print __
        "#,
    }.hit_log(1);

    check_compile!{parser,
        r#"
        print Builtin.Chr[32];
        "#,
        r#"
            print " "
        "#,
    };

    check_compile!{parser,
        r#"
        print Builtin.Chr[97];
        "#,
        r#"
            print "a"
        "#,
    };

    check_compile!{@with_source parser,
        r#"
        print Builtin.Chr[10];
        "#,
        r#"
            print __
        "#,
    }.hit_log(1);

    check_compile!{@with_source parser,
        r#"
        print Builtin.Chr[34];
        "#,
        r#"
            print __
        "#,
    }.hit_log(1);

    check_compile!{parser,
        r#"
        print Builtin.Chr[0x61];
        "#,
        r#"
            print "a"
        "#,
    };
}

#[test]
fn closure_value_test() {
    let parser = TopLevelParser::new();

    check_sugar! {parser,
        r#"
        const X = ([A &B]2);
        "#,
        r#"
        const X = ([A:A &B:B]2);
        "#,
    };

    check_compile!{parser,
        r#"
        const A = (a: print "makeA";);
        const B = (b: print "makeB";);
        const F = ([A &B](
            print "Do"A B"End";
        ));
        const A="eA" B="eB";
        take F;
        "#,
        r#"
            print "makeA"
            print "Do"
            print a
            print "makeB"
            print b
            print "End"
        "#,
    };

    check_compile!{parser,
        r#"
        const A = 2;
        const B = `A`;
        const F = ([B](
            print B;
        ));
        take F;
        "#,
        r#"
            print A
        "#,
    };

    check_compile!{parser,
        r#"
        const A = 2;
        const B = `A`;
        const V = ([]`B`);
        const B = "e";
        print V;
        "#,
        r#"
            print B
        "#,
    };

    check_compile!{parser,
        r#"
        const Do = (
            take _0 _0;
        );
        take Do[([](
            :x
            print 2;
            goto :x a < b;
        ))];
        :x
        goto :x;
        "#,
        r#"
            print 2
            jump 0 lessThan a b
            print 2
            jump 2 lessThan a b
            jump 4 always 0 0
        "#,
    };

    check_compile!{parser,
        r#"
        const Do = (
            take _0 _0;
        );
        take Do[([&F:(
            :x
            print 2;
            goto :x a < b;
        )](
            take F;
        ))];
        :x
        goto :x;
        "#,
        r#"
            print 2
            jump 0 lessThan a b
            print 2
            jump 2 lessThan a b
            jump 4 always 0 0
        "#,
    };

    check_compile!{parser,
        r#"
        print ([X:1](
            print "foo";
            setres X;
        ));
        "#,
        r#"
            print "foo"
            print 1
        "#,
    };

    check_compile!{parser,
        r#"
        const x.Clos = ([N:2](
            print .. __Binder;
        ));
        x.Clos!;
        "#,
        r#"
            print __0
            print x
        "#,
    };

    check_compile!{parser,
        r#"
        const x.Clos = ([N:2 ..B](
            print .. B;
        ));
        x.Clos!;
        "#,
        r#"
            print __0
            print x
        "#,
    };

    check_compile!{parser,
        r#"
        # 污染
        const F = ([N:2](match @ => F {
            foo F;
        }));
        const N = 3;
        F! (m:print N;);
        "#,
        r#"
            print 2
            foo m
        "#,
    };

    check_compile!{parser,
        r#"
        # 懒闭包
        const F = ([N:2]match @ => F {
            foo F;
        });
        const N = 3;
        F! (m:print N;);
        "#,
        r#"
            print 3
            foo m
        "#,
    };

    check_compile!{parser,
        r#"
        const F = ([N:2]match @ {
            {}
            F {
                foo F;
            }
        });
        const N = 3;
        F! (m:print N;);
        "#,
        r#"
            print 3
            foo m
        "#,
    };

    check_compile!{parser,
        r#"
        const F = ([N:2](const match @ {
            {}
            F {
                mid;
                foo F;
            }
        }));
        const N = 3;
        F! (m:print N;);
        "#,
        r#"
            mid
            print 2
            foo m
        "#,
    };

    check_compile!{parser,
        r#"
        const F = ([N:2]const match @ {
            {}
            F {
                mid;
                foo F;
            }
        });
        const N = 3;
        F! (m:print N;);
        "#,
        r#"
            mid
            print 2
            foo m
        "#,
    };

    check_compile!{parser,
        r#"
        const F = ([N:2]const match @ {
            {}
            *F {
                mid;
                foo F;
            }
        });
        const N = 3;
        F! (m:print N;);
        "#,
        r#"
            print 3
            mid
            foo m
        "#,
    };

    check_compile!{parser,
        r#"
        const F = ([N:2]r:const match @ {
            {}
            *F {
                mid;
                foo F;
            }
        });
        const N = 3;
        ret F[(m:print N;)];
        "#,
        r#"
            print 3
            mid
            foo m
            ret r
        "#,
    };

    check_compile!{parser,
        r#"
        const F = ([N:2]r:match @ {
            {}
            F {
                mid;
                foo F;
            }
        });
        const N = 3;
        ret F[(m:print N;)];
        "#,
        r#"
            print 3
            mid
            foo m
            ret r
        "#,
    };

    check_compile!{parser,
        r#"
        const F = ([N:2]r:const match @ {
            [?(0:const match @ {})] {}
            *F {
                mid;
                foo F;
            }
        });
        const N = 3;
        ret F[(m:print N;)];
        "#,
        r#"
            print 3
            mid
            foo m
            ret r
        "#,
    };

    check_compile!{parser,
        r#"
        const F = ([N:2]r:const match @ {
            [?(0:const match @ {})] {}
            *F {
                mid;
                foo F;
            }
        });
        const N = 3;
        ret F[(m:print N;)];
        "#,
        r#"
            print 3
            mid
            foo m
            ret r
        "#,
    };

    check_compile!{parser,
        r#"
        const F = ([N:2]r:const match @ {
            [?(0:const match @ {})] {}
            F {
                mid;
                foo F;
            }
        });
        const N = 3;
        ret F[(m:print N;)];
        "#,
        r#"
            mid
            print 2
            foo m
            ret r
        "#,
    };
}

#[test]
fn value_bind_ref_test() {
    let parser = TopLevelParser::new();

    check_compile!{parser,
        r#"
        const bind.V = (
            print "take";
            setres "finish";
        );
        print 1;
        const V = bind->V;
        print 2;
        const bind.V = (
            print "fail";
        );
        print V;
        "#,
        r#"
            print 1
            print 2
            print "take"
            print "finish"
        "#,
    };

    check_compile!{parser,
        r#"
        const Bind = bind;
        const Bind.V = (
            print "take";
            setres "finish";
        );
        print 1;
        const V = Bind->V;
        const V1 = bind->V;
        print 2;
        const Bind.V = (
            print "fail";
        );
        print V V1;
        "#,
        r#"
            print 1
            print 2
            print "take"
            print "finish"
            print "take"
            print "finish"
        "#,
    };

    check_compile!{parser,
        r#"
        const bind.V = (
            print "take";
            setres "finish";
        );
        print 1;
        const V = bind->V;
        print 2;
        const bind.V = (
            print "fail";
        );
        print V->..;
        "#,
        r#"
            print 1
            print 2
            print bind
        "#,
    };

    check_compile!{parser,
        r#"
        const bind.V = (
            print "take";
            setres "finish";
        );
        print 1;
        const B = bind->V->..;
        print 2;
        const bind.V = (
            print "fail";
        );
        print B;
        "#,
        r#"
            print 1
            print 2
            print bind
        "#,
    };

    check_compile!{parser,
        r#"
        const Res = (%
            print "maked";
            const $.M = 2;
        %)->$;
        print 1 Res.M;
        "#,
        r#"
            print "maked"
            print 1
            print 2
        "#,
    };

    check_compile!{parser,
        r#"
        const F = (print 2;);
        print 1;
        const Attr = F->X;
        print 3 Attr;
        "#,
        r#"
            print 1
            print 2
            print 3
            print __1
        "#,
    };

    check_compile!{@with_source parser,
        r#"
        const bind.next.X = 2;
        const F = bind->next->X;
        print bind.next F->.. F->..->..;
        "#,
        r#"
        print __0
        print __0
        print __
        "#
    }.hit_log(2);

    check_compile!{parser,
        r#"
        const bind.X = (x: print "makeX";);
        const bind.Y = (y: print "makeY";);
        print 1;
        const X = bind->X;
        print 2 X;
        const Y = X->..->Y;
        print 3 Y;
        print X->.. Y->..;
        "#,
        r#"
            print 1
            print 2
            print "makeX"
            print x
            print 3
            print "makeY"
            print y
            print bind
            print bind
        "#,
    };

    check_compile!{parser,
        r#"
        const F = ($ = 1+2;);
        print F->op;
        "#,
        r#"
            print 3
        "#,
    };

    check_compile!{parser,
        r#"
        const F = ($ = 1+x;);
        print F->op;
        "#,
        r#"
            print __
        "#,
    };

    check_compile!{parser,
        r#"
        const N = 1;
        const F = ($ = N+2;);
        const R = F->op;
        const N = 2;
        print R;
        "#,
        r#"
            print 3
        "#,
    };

    check_compile!{parser,
        r#"
        const N = 1;
        const F = ($ = N+x;);
        const R = F->op;
        const N = 2;
        print R;
        "#,
        r#"
            print __
        "#,
    };
}

#[test]
fn gswitch_test() {
    let parser = TopLevelParser::new();

    check_compile!{parser,
        r#"
        gswitch x {
        case: print 1;
        case: print 2;
        }
        "#,
        r#"
            op add @counter @counter x
            jump 3 always 0 0
            jump 4 always 0 0
            print 1
            print 2
        "#,
    };

    check_compile!{parser,
        r#"
        gswitch x {
            break;
        case: print 1;
        case: print 2;
        }
        "#,
        r#"
            op add @counter @counter x
            jump 3 always 0 0
            jump 5 always 0 0
            print 1
            jump 0 always 0 0
            print 2
            jump 0 always 0 0
        "#,
    };

    check_compile!{parser,
        r#"
        gswitch x {
            break;
        case 1: print 1;
        case: print 2;
        }
        "#,
        r#"
            op add @counter @counter x
            jump 0 always 0 0
            jump 4 always 0 0
            jump 6 always 0 0
            print 1
            jump 0 always 0 0
            print 2
            jump 0 always 0 0
        "#,
    };

    check_compile!{parser,
        r#"
        gswitch x {
            break;
        case 1: print 1;
        case !: print mis;
        case: print 2;
        }
        "#,
        r#"
            op add @counter @counter x
            jump 6 always 0 0
            jump 4 always 0 0
            jump 8 always 0 0
            print 1
            jump 0 always 0 0
            print mis
            jump 0 always 0 0
            print 2
            jump 0 always 0 0
        "#,
    };

    check_compile!{parser,
        r#"
        gswitch x {
            break;
        case 1: print 1;
        case ! MAX: print mis MAX;
        case: print 2;
        }
        "#,
        r#"
            op add @counter @counter x
            jump 6 always 0 0
            jump 4 always 0 0
            jump 9 always 0 0
            print 1
            jump 0 always 0 0
            print mis
            print x
            jump 0 always 0 0
            print 2
            jump 0 always 0 0
        "#,
    };

    check_compile!{parser,
        r#"
        gswitch x {
            break;
        case 1: print 1;
        case ! MAX: print mis MAX;
        case: print 2;
        }
        end;
        "#,
        r#"
            op add @counter @counter x
            jump 6 always 0 0
            jump 4 always 0 0
            jump 9 always 0 0
            print 1
            jump 11 always 0 0
            print mis
            print x
            jump 11 always 0 0
            print 2
            jump 11 always 0 0
            end
        "#,
    };

    check_compile!{parser,
        r#"
        gswitch x {
            break;
        case 1: print 1;
        case <: print less;
        case: print 2;
        }
        end;
        "#,
        r#"
            jump 7 lessThan x 0
            op add @counter @counter x
            jump 11 always 0 0
            jump 5 always 0 0
            jump 9 always 0 0
            print 1
            jump 11 always 0 0
            print less
            jump 11 always 0 0
            print 2
            jump 11 always 0 0
            end
        "#,
    };

    check_compile!{parser,
        r#"
        gswitch x {
            break;
        case 1: print 1;
        case >: print 'greaterThan';
        case: print 2;
        }
        end;
        "#,
        r#"
            jump 7 greaterThan x 2
            op add @counter @counter x
            jump 11 always 0 0
            jump 5 always 0 0
            jump 9 always 0 0
            print 1
            jump 11 always 0 0
            print greaterThan
            jump 11 always 0 0
            print 2
            jump 11 always 0 0
            end
        "#,
    };

    check_compile!{parser,
        r#"
        gswitch x {
            break;
        case 1: print 1;
        case !>: print hit;
        case: print 2;
        }
        end;
        "#,
        r#"
            jump 7 greaterThan x 2
            op add @counter @counter x
            jump 7 always 0 0
            jump 5 always 0 0
            jump 9 always 0 0
            print 1
            jump 11 always 0 0
            print hit
            jump 11 always 0 0
            print 2
            jump 11 always 0 0
            end
        "#,
    };

    check_compile!{parser,
        r#"
        gswitch x {
            break;
        case 1: print 1;
        case <!>: print hit;
        case: print 2;
        }
        end;
        "#,
        r#"
            jump 8 lessThan x 0
            jump 8 greaterThan x 2
            op add @counter @counter x
            jump 8 always 0 0
            jump 6 always 0 0
            jump 10 always 0 0
            print 1
            jump 12 always 0 0
            print hit
            jump 12 always 0 0
            print 2
            jump 12 always 0 0
            end
        "#,
    };

    check_compile!{parser,
        r#"
        const I = 1;
        gswitch x {
            break;
        case I: print `I`;
        }
        end;
        "#,
        r#"
            op add @counter @counter x
            jump 5 always 0 0
            jump 3 always 0 0
            print I
            jump 5 always 0 0
            end
        "#,
    };

    check_compile!{parser,
        r#"
        const I = 1;
        gswitch x {
            break;
        case I if y: print `I`;
        }
        end;
        "#,
        r#"
            op add @counter @counter x
            jump 8 always 0 0
            jump 4 always 0 0
            jump 8 always 0 0
            jump 6 notEqual y false
            jump 8 always 0 0
            print I
            jump 8 always 0 0
            end
        "#,
    };

    check_compile!{parser,
        r#"
        match 1 2 { @ {} }
        gswitch x {
            break;
        case @: print x;
        }
        end;
        "#,
        r#"
            op add @counter @counter x
            jump 6 always 0 0
            jump 4 always 0 0
            jump 4 always 0 0
            print x
            jump 6 always 0 0
            end
        "#,
    };

    check_compile!{parser,
        r#"
        const match (1:) 2 { @ {} }
        gswitch x {
            break;
        case @: print x;
        }
        end;
        "#,
        r#"
            op add @counter @counter x
            jump 6 always 0 0
            jump 4 always 0 0
            jump 4 always 0 0
            print x
            jump 6 always 0 0
            end
        "#,
    };

    check_compile!{parser,
        r#"
        const match (1:) (2: print start;) { @ {} }
        gswitch x {
            break;
        case @: print x;
        }
        end;
        "#,
        r#"
            print start
            op add @counter @counter x
            jump 7 always 0 0
            jump 5 always 0 0
            jump 5 always 0 0
            print x
            jump 7 always 0 0
            end
        "#,
    };

    check_compile!{parser,
        r#"
        gswitch x {
        case: print 1;
        case: print 2 3 4;
        }
        "#,
        r#"
            op add @counter @counter x
            jump 3 always 0 0
            jump 4 always 0 0
            print 1
            print 2
            print 3
            print 4
        "#,
    };

    check_compile!{parser,
        r#"
        gswitch x {
        case 0: print 0;
        case 1 2 3: print 1 2 3;
        }
        "#,
        r#"
            op add @counter @counter x
            jump 5 always 0 0
            jump 6 always 0 0
            jump 6 always 0 0
            jump 6 always 0 0
            print 0
            print 1
            print 2
            print 3
        "#,
    };

    check_compile!{parser,
        r#"
        gswitch x {
            break;
        case 1 2: print x;
        case*3: print end;
        }
        end;
        "#,
        r#"
            op add @counter @counter x
            jump 8 always 0 0
            jump 5 always 0 0
            jump 5 always 0 0
            jump 7 always 0 0
            print x
            jump 8 always 0 0
            print end
            end
        "#,
    };

    check_compile!{parser,
        r#"
        gswitch x {
            break;
        case*1 2: print x;
        case 3: print end;
        }
        end;
        "#,
        r#"
            op add @counter @counter x
            jump 8 always 0 0
            jump 5 always 0 0
            jump 5 always 0 0
            jump 6 always 0 0
            print x
            print end
            jump 8 always 0 0
            end
        "#,
    };
}

#[test]
fn closure_catch_label_test() {
    let parser = TopLevelParser::new();

    check_compile!{parser,
        r#"
        const Run = (
            :x
            print unexpected;
            take _0;
        );
        :x
        print expected;
        take Run[(
            goto :x;
        )];
        "#,
        r#"
            print expected
            print unexpected
            jump 1 always 0 0
        "#,
    };

    check_compile!{parser,
        r#"
        const Run = (
            :x
            print unexpected;
            take _0;
        );
        :x
        print expected;
        take Run[([| :x](
            goto :x;
        ))];
        "#,
        r#"
            print expected
            print unexpected
            jump 0 always 0 0
        "#,
    };

    check_compile!{parser,
        r#"
        const F = (
            const Run = (
                :x
                print unexpected;
                take _0;
            );
            :x
            print expected;
            take Run[([| :x](
                goto :x;
            ))];
        );
        take F;
        "#,
        r#"
            print expected
            print unexpected
            jump 0 always 0 0
        "#,
    };

    check_compile!{parser,
        r#"
        const F = (
            const Run = (
                :x
                print unexpected;
                take _0;
            );
            :x
            print expected;
            take Run[([| :x](
                goto :x;
            ))];
        );
        take F F;
        "#,
        r#"
            print expected
            print unexpected
            jump 0 always 0 0
            print expected
            print unexpected
            jump 3 always 0 0
        "#,
    };

    check_compile!{parser,
        r#"
        const F = (
            const Run = (
                :x
                print unexpected;
                take _0 _0;
            );
            :x
            print expected;
            take Run[([| :x](
                goto :x;
            ))];
        );
        take F F;
        "#,
        r#"
            print expected
            print unexpected
            jump 0 always 0 0
            jump 0 always 0 0
            print expected
            print unexpected
            jump 4 always 0 0
            jump 4 always 0 0
        "#,
    };

    check_compile!{parser,
        r#"
        const F = (
            const Run = (
                :x
                print unexpected;
                take _0 _0;
            );
            take Run[([| :x](
                goto :x;
            ))];
            :x
            print expected;
        );
        take F F;
        "#,
        r#"
            print unexpected
            jump 3 always 0 0
            jump 3 always 0 0
            print expected
            print unexpected
            jump 7 always 0 0
            jump 7 always 0 0
            print expected
        "#,
    };

    check_compile!{parser,
        r#"
        const Run = (
            :x
            print unexpected;
            take _0 _0;
        );
        take Run[([| :x](
            goto :x;
        ))];
        :x
        print expected;
        "#,
        r#"
            print unexpected
            jump 3 always 0 0
            jump 3 always 0 0
            print expected
        "#,
    };
}

#[test]
fn closure_binder_test() {
    let parser = TopLevelParser::new();

    check_compile!{parser,
        r#"
        const Foo = ([N:2](match => {
            print N, ..->N;
        }));
        take Foo;
        "#,
        r#"
            print 2
            print 2
        "#,
    };

    check_compile!{parser,
        r#"
        const Foo = ([N:2]match => {
            print N, ..->N;
        });
        take Foo;
        "#,
        r#"
            print 2
            print 2
        "#,
    };

    check_compile!{parser,
        r#"
        const handle.Foo = ([N:2 ..B](match => {
            print N, ..->N, B;
        }));
        take handle.Foo;
        "#,
        r#"
            print 2
            print 2
            print handle
        "#,
    };

    check_compile!{parser,
        r#"
        const handle.Foo = ([N:2 ..B]match => {
            print N, ..->N, B;
        });
        take handle.Foo;
        "#,
        r#"
            print 2
            print 2
            print handle
        "#,
    };
}

#[test]
fn non_take_result_handle_dexp_test() {
    let parser = TopLevelParser::new();

    check_compile!{parser,
        r#"
        const H = 2;
        print (H: print pre;);
        "#,
        r#"
            print pre
            print 2
        "#,
    };

    check_compile!{parser,
        r#"
        const H = 2;
        print (`H`: print pre;);
        "#,
        r#"
            print pre
            print H
        "#,
    };

    check_compile!{parser,
        r#"
        const H = (2:);
        print (`H`: print pre;);
        "#,
        r#"
            print pre
            print H
        "#,
    };

    check_compile!{parser,
        r#"
        print (?`n`: 2);
        "#,
        r#"
            set n 2
            print n
        "#,
    };
}

#[test]
fn to_label_code_test() {
    let parser = TopLevelParser::new();

    let mut lines = CompileMeta::new().compile(parse!(parser, r#"
        :2
        goto :2;
        goto :end;
        :end
        :end1
    "#).unwrap());

    assert_eq!(
        lines.to_string().lines().collect::<Vec<_>>(),
        vec![
            r#":2:"#,
            r#"jump :2 always 0 0"#,
            r#"jump end always 0 0"#,
            r#"end:"#,
            r#"end1:"#,
        ],
    );
    lines.index_label_popup();
    assert_eq!(
        lines.to_string().lines().collect::<Vec<_>>(),
        vec![
            r#"end1:"#,
            r#"end:"#,
            r#":2:"#,
            r#"jump :2 always 0 0"#,
            r#"jump end always 0 0"#,
        ],
    );
}

#[test]
fn closure_catch_args_test() {
    let parser = TopLevelParser::new();

    check_compile!{parser,
        r#"
        match a b c => @ {}
        const Clos = ([@](
            print @;
        ));
        take Clos[];
        "#,
        r#"
            print a
            print b
            print c
        "#,
    };

    check_compile!{parser,
        r#"
        match a b c => @ {}
        const Clos = ([@](
            print @;
        ));
        match d e f => @ {}
        take Clos[];
        "#,
        r#"
            print a
            print b
            print c
        "#,
    };

    check_compile!{parser,
        r#"
        match a b c => @ {}
        const Clos = ([@](
            print @;
        ));
        match d e f => @ {}
        take Clos[];
        print @;
        "#,
        r#"
            print a
            print b
            print c
            print d
            print e
            print f
        "#,
    };

    check_compile!{parser,
        r#"
        match a b c => @ {}
        const Clos = ([@](
            print @;
        ));
        const a = 2;
        take Clos[];
        "#,
        r#"
            print a
            print b
            print c
        "#,
    };

    check_compile!{parser,
        r#"
        match a b c => @ {}
        const Clos = ([@](
            print @;
        ));
        take Clos[1 2];
        "#,
        r#"
            print a
            print b
            print c
        "#,
    };

    // moved value owned test
    check_compile!{parser,
        r#"
        const Builder = (
            const $.F = ([@](
                print @;
            ));
        );
        match 1 2 3 => @ {}
        const Clos = Builder[a b c]->F;
        match 4 5 6 => @ {}
        take Clos;
        take Clos[];
        "#,
        r#"
            print a
            print b
            print c
            print a
            print b
            print c
        "#,
    };

    check_compile!{parser,
        r#"
        const Builder = (
            const $.F = ([@](
                print @;
            ));
        );
        const Clos = Builder[a b c]->F;
        const a = 1;
        take Clos;
        const b = 2;
        take Clos[];
        "#,
        r#"
            print a
            print b
            print c
            print a
            print b
            print c
        "#,
    };

    check_compile!{parser,
        r#"
        const Builder = (
            const $.F = ([@](
                print @;
            ));
        );
        const Clos = Builder[(x:
            print run;
        )]->F;
        print split;
        take Clos[];
        "#,
        r#"
            print split
            print run
            print x
        "#,
    };

    check_compile!{parser,
        r#"
        const Builder = (
            const $.F = ([P:(print pre;) @](
                print @;
            ));
        );
        const Clos = Builder[(x:
            print run;
        )]->F;
        print split;
        take Clos[];
        "#,
        r#"
            print pre
            print split
            print run
            print x
        "#,
    };

    check_compile!{parser,
        r#"
        const Builder = (
            const $.F = ([@](
                print @ _0;
            ));
        );
        const Clos = Builder[m]->F;
        print split;
        take Clos[n];
        "#,
        r#"
            print split
            print m
            print m
        "#,
    };

    check_compile!{parser,
        r#"
        const Builder = (
            const $.F = ([@](
                print @ _0;
            ));
        );
        const Clos = Builder[m]->F;
        print split;
        const m = 4;
        take Clos[n];
        "#,
        r#"
            print split
            print m
            print m
        "#,
    };
}

#[test]
fn param_deref_test() {
    let parser = TopLevelParser::new();

    check_compile!{parser,
        r#"
        const F = (
            const N = 2;
            print @;
        );
        const N = 1;
        take F[(setres N;)];
        take F[*(setres N;)];
        "#,
        r#"
            print 2
            print 1
        "#,
    };

    check_compile!{parser,
        r#"
        const F = (
            const N = 2;
            print @;
        );
        const match (setres N;) => @ {}
        const N = 1;
        take F[(setres N;) @];
        take F[*(setres N;) @];
        "#,
        r#"
            print 2
            print 2
            print 1
            print 2
        "#,
    };

    check_compile!{parser,
        r#"
        const F = (
            const N = 2;
            print @;
        );
        const match (setres N;) => @ {}
        const N = 1;
        take F[(setres N;) @];
        match @ => @ {}
        take F[*(setres N;) @];
        "#,
        r#"
            print 2
            print 2
            print 1
            print 1
        "#,
    };
}

#[test]
fn param_inf_len_test() {
    let parser = TopLevelParser::new();

    check_compile!{parser,
        r#"
        const F = (
            print @;
        );
        F! 0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15
            16 17 18 19 20 21 22 23 24 25 26 27 28 29 30 31;
        "#,
        r#"
            print 0
            print 1
            print 2
            print 3
            print 4
            print 5
            print 6
            print 7
            print 8
            print 9
            print 10
            print 11
            print 12
            print 13
            print 14
            print 15
            print 16
            print 17
            print 18
            print 19
            print 20
            print 21
            print 22
            print 23
            print 24
            print 25
            print 26
            print 27
            print 28
            print 29
            print 30
            print 31
        "#,
    };

    check_compile!{parser,
        r#"
        (%print @;%)! 0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15
            16 17 18 19 20 21 22 23 24 25 26 27 28 29 30 31;
        "#,
        r#"
            print 0
            print 1
            print 2
            print 3
            print 4
            print 5
            print 6
            print 7
            print 8
            print 9
            print 10
            print 11
            print 12
            print 13
            print 14
            print 15
            print 16
            print 17
            print 18
            print 19
            print 20
            print 21
            print 22
            print 23
            print 24
            print 25
            print 26
            print 27
            print 28
            print 29
            print 30
            print 31
        "#,
    };
}

#[test]
fn keywords_sugar_test() {
    let parser = TopLevelParser::new();

    check_sugar! {parser,
        r#"
        foo _ a;
        foo `_` a;
        "#,
        r#"
        foo '_' a;
        foo `'_'` a;
        "#,
    };

    check_sugar! {parser,
        r#"
        foo op a;
        "#,
        r#"
        foo 'op' a;
        "#,
    };

    check_sugar! {parser,
        r#"
        X! _ b;
          X! `_` b;
        "#,
        r#"
        X! '_' b;
        X! `'_'` b;
        "#,
    };

    check_sugar! {parser,
        r#"
        take F[_ c];
          take F[`_` c];
        "#,
        r#"
        take F['_' c];
        take F[`'_'` c];
        "#,
    };

    check_sugar! {parser,
        r#"
        take F[op c];
        "#,
        r#"
        take F['op' c];
        "#,
    };

    check_sugar! {parser,
        r#"
        if max < 2 && len == 3 { noop }
        "#,
        r#"
        if 'max' < 2 && 'len' == 3 { noop; }
        "#,
    };

    check_sugar! {parser,
        r#"
        print `+`;
        "#,
        r#"
        print `add`;
        "#,
    };
}

#[test]
fn global_bind_test() {
    let parser = TopLevelParser::new();

    check_compile!{parser,
        r#"
        const MakeBind = (
            const _0.Foo = ([N:_1 ..B](
                print B N;
            ));
        );
        MakeBind! a 1;
        a.Foo!;

        MakeBind! __global 2;
        a.Foo!;
        b.Foo!;
        "#,
        r#"
            print a
            print 1
            print a
            print 1
            print b
            print 2
        "#,
    };
}

#[test]
fn lines_end_no_semicolon_test() {
    let parser = TopLevelParser::new();

    check_sugar! {parser,
        r#"
        {x=2}
        "#,
        r#"
        {x=2;}
        "#,
    };

    check_sugar! {parser,
        r#"
        x=2
        "#,
        r#"
        x=2;
        "#,
    };

    check_sugar! {parser,
        r#"
        take X
        "#,
        r#"
        take X;
        "#,
    };

    check_sugar! {parser,
        r#"
        {y=3; x=2}
        "#,
        r#"
        {y=3; x=2;}
        "#,
    };

    check_sugar! {parser,
        r#"
        do {noop} while
        "#,
        r#"
        do {noop;} while;
        "#,
    };

    check_sugar! {parser,
        r#"
        do {noop} while a<b
        "#,
        r#"
        do {noop;} while a<b;
        "#,
    };

    check_sugar! {parser,
        r#"
        print (op add $ x 1);
        "#,
        r#"
        print (op add $ x 1;);
        "#,
    };

    check_sugar! {parser,
        r#"
        print (read $ cell1 0);
        "#,
        r#"
        print (read $ cell1 0;);
        "#,
    };
}

#[test]
fn loop_do_sugar_test() {
    let parser = TopLevelParser::new();

    check_sugar! {parser,
        r#"
        while do a < b {
            print 1;
        case:
            print 2;
        }
        "#,
        r#"
        skip a >= b {
            inline { print 1 }
            do {
                print 2
            } while a < b;
        }
        "#,
    };

    check_sugar! {parser,
        r#"
        while do a < b {
            break; continue;
            print 1;
        case:
            break; continue;
            print 2;
        }
        "#,
        r#"
        skip a >= b {
            inline {
                break; continue;
                print 1;
            }
            do {
                break; continue;
                print 2
            } while a < b;
        }
        "#,
    };

    check_sugar! {parser,
        r#"
        goto do {
            print 1;
        case:
            print 2;
        } while a < b;
        "#,
        r#"
        {
            goto :___0;
            {
                :___1
                {
                    print 1;
                    :___0
                    print 2;
                }
                goto :___1 a < b;
            }
        }
        "#,
    };

    check_sugar! {parser,
        r#"
        goto do {
            print 1;
        case cond:
            print 2;
        } while a < b;
        "#,
        r#"
        {
            goto :___0 cond;
            {
                :___1
                {
                    print 1;
                    :___0
                    print 2;
                }
                goto :___1 a < b;
            }
        }
        "#,
    };

    check_sugar! {parser,
        r#"
        goto do {
            break; continue;
            print 1;
        case cond:
            break; continue;
            print 2;
        } while a < b;
        "#,
        r#"
        {
            goto :___2 cond;
            {
                :___3
                {
                    goto :___0;
                    goto :___1;
                    print 1;
                    :___2
                    goto :___0;
                    goto :___1;
                    print 2;
                }
                :___1
                goto :___3 a < b;
                :___0
            }
        }
        "#,
    };
}

#[test]
fn no_effect_hint_test() {
    let parser = TopLevelParser::new();

    check_compile!{@with_source parser,
        r#"
        Foo[];
        "#,
        r#"
        Foo
        "#
    }.hit_log(1);

    check_compile!{@with_source parser,
        r#"
        const Foo = 2;
        Foo[];
        "#,
        r#"
        2
        "#
    }.hit_log(1);

    check_compile!{@with_source parser,
        r#"
        const Foo.X = 2;
        Foo.X[];
        "#,
        r#"
        2
        "#
    }.hit_log(1);

    check_compile!{@with_source parser,
        r#"
        foo.X[];
        "#,
        r#"
        __0
        "#
    }.hit_log(1);

    check_compile!{@with_source parser,
        r#"
        foo.bar.X[];
        "#,
        r#"
        __1
        "#
    }.hit_log(1);

    check_compile!{@with_source parser,
        r#"
        const foo.bar = m;
        foo.bar.X[];
        "#,
        r#"
        __1
        "#
    }.hit_log(1);

    check_compile!{@with_source parser,
        r#"
        const Foo.X = 2;
        Foo->X[];
        "#,
        r#"
        2
        "#
    }.hit_log(1);

    check_compile!{@with_source parser,
        r#"
        const Foo.X = 2;
        `Foo`->X[];
        "#,
        r#"
        2
        "#
    }.hit_log(1);

    check_compile!{@with_source parser,
        r#"
        const foo.bar.X = 2;
        `foo`.bar->X[];
        "#,
        r#"
        2
        "#
    }.hit_log(1);

    check_compile!{@with_source parser,
        r#"
        const foo.X = ();
        foo.X[];
        "#,
        r#"
        __1
        "#
    }.hit_log(0);

    check_compile!{@with_source parser,
        r#"
        const foo.X = ();
        foo->X[];
        "#,
        r#"
        __1
        "#
    }.hit_log(0);

    check_compile!{@with_source parser,
        r#"
        const foo.X = ();
        `foo`->X[];
        "#,
        r#"
        __1
        "#
    }.hit_log(0);

    check_compile!{@with_source parser,
        r#"
        const foo.bar.X = ();
        foo.bar.X[];
        "#,
        r#"
        __2
        "#
    }.hit_log(0);

    check_compile!{@with_source parser,
        r#"
        const foo.bar.X = ();
        foo.bar->X[];
        "#,
        r#"
        __2
        "#
    }.hit_log(0);

    check_compile!{@with_source parser,
        r#"
        const foo.bar.X = ();
        `foo`.bar->X[];
        "#,
        r#"
        __2
        "#
    }.hit_log(0);
}
