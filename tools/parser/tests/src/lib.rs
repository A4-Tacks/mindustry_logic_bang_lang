#![cfg(test)]
use ::parser::*;
use ::syntax::*;
use ::tag_code::*;
use ::either::Either::{self, Left, Right};
use logic_parser::{ParseLine, IdxBox};

/// 快捷的创建一个新的`Meta`并且`parse`
macro_rules! parse {
    ( $parser:expr, $src:expr ) => {
        ($parser).parse(&mut Meta::new(), $src)
    };
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

#[test]
fn var_test() {
    let parser = VarParser::new();

    assert_eq!(parse!(parser, "_abc").unwrap(), "_abc");
    assert_eq!(parse!(parser, "'ab-cd'").unwrap(), "ab-cd");
    assert_eq!(parse!(parser, "'ab.cd'").unwrap(), "ab.cd");
    assert_eq!(parse!(parser, "0x1_b").unwrap(), "0x1b");
    assert_eq!(parse!(parser, "-4_3_.7_29").unwrap(), "-43.729");
    assert_eq!(parse!(parser, "0b-00_10").unwrap(), "0b-0010");
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
                        Value::ResultHandle,
                        "1".into(),
                        "2".into()
                    ).into(),
                    Op::Mul(
                        Value::ResultHandle,
                        Value::ResultHandle,
                        "2".into()
                    ).into()
                ].into()).into(),
            DExp::new(
                "x".into(),
                vec![
                    Op::Mul(Value::ResultHandle, "2".into(), "3".into()).into()
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
                        Value::ResultHandle,
                        "1".into(),
                        "2".into()
                    ).into(),
                    Op::Mul(
                        Value::ResultHandle,
                        Value::ResultHandle,
                        "2".into()
                    ).into()
                ].into()).into(),
            DExp::new_nores(
                vec![
                    Op::Mul(Value::ResultHandle, "2".into(), "3".into()).into()
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
    let parser = LogicLineParser::new();
    assert_eq!(
        parse!(parser, r#"skip 1 < 2 print "hello";"#).unwrap(),
        Expand(vec![
            Goto("___0".into(), JumpCmp::LessThan("1".into(), "2".into()).into()).into(),
            LogicLine::Other(vec![Value::ReprVar("print".into()), r#""hello""#.into()].into()),
            LogicLine::Label("___0".into()),
        ]).into()
    );

    assert_eq!(
        parse!(parser, r#"
        if 2 < 3 {
            print 1;
        } elif 3 < 4 {
            print 2;
        } elif 4 < 5 {
            print 3;
        } else print 4;
        "#).unwrap(),
        parse!(parser, r#"
        {
            goto :___1 2 < 3;
            goto :___2 3 < 4;
            goto :___3 4 < 5;
            print 4;
            goto :___0 _;
            :___2 {
                print 2;
            }
            goto :___0 _;
            :___3 {
                print 3;
            }
            goto :___0 _;
            :___1 {
                print 1;
            }
            :___0
        }
        "#).unwrap(),
    );

    assert_eq!(
        parse!(parser, r#"
        if 2 < 3 { # 对于没有elif与else的if, 会将条件反转并构建为skip
            print 1;
        }
        "#).unwrap(),
        parse!(parser, r#"
        skip ! 2 < 3 {
            print 1;
        }
        "#).unwrap(),
    );

    assert_eq!(
        parse!(parser, r#"
        while a < b
            print 3;
        "#).unwrap(),
        parse!(parser, r#"
        {
            goto :___0 a >= b;
            :___1
            print 3;
            goto :___1 a < b;
            :___0
        }
        "#).unwrap(),
    );

    assert_eq!(
        parse!(parser, r#"
        do {
            print 1;
        } while a < b;
        "#).unwrap(),
        parse!(parser, r#"
        {
            :___0 {
                print 1;
            }
            goto :___0 a < b;
        }
        "#).unwrap(),
    );

    assert_eq!(
        parse!(parser, r#"
        gwhile a < b {
            print 1;
        }
        "#).unwrap(),
        parse!(parser, r#"
        {
            goto :___0 _;
            :___1 {
                print 1;
            }
            :___0
            goto :___1 a < b;
        }
        "#).unwrap(),
    );

    assert_eq!(
        parse!(parser, r#"
        skip _ {
            print 1;
        }
        "#).unwrap(),
        parse!(parser, r#"
        skip {
            print 1;
        }
        "#).unwrap(),
    );

    assert_eq!(
        parse!(parser, r#"
        do do {
            print 1;
        } while a < b; while c < d;
        "#).unwrap(),
        parse!(parser, r#"
        {
            :___1 {
                :___0 {
                    print 1;
                }
                goto :___0 a < b;
            }
            goto :___1 c < d;
        }
        "#).unwrap(),
    );

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
    let parser = LogicLineParser::new();

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
            parse!(parser, src).unwrap().as_goto().unwrap().1.clone().reverse(),
            parse!(parser, dst).unwrap().as_goto().unwrap().1,
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
        assert_eq!(
            parse!(parser, src).unwrap().as_goto().unwrap().1,
            parse!(parser, dst).unwrap().as_goto().unwrap().1,
        );
    }
}

#[test]
fn goto_compile_test() {
    let parser = TopLevelParser::new();

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    goto :x _;
    :x
    end;
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
               "jump 1 always 0 0",
               "end",
    ]);

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    const 0 = 1;
    goto :x _;
    :x
    end;
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
               "jump 1 always 0 0",
               "end",
    ]);

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    const 0 = 1;
    goto :x !_;
    :x
    end;
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
               "end",
    ]);

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    const 0 = 1;
    const false = true;
    goto :x a === b;
    :x
    end;
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
               "jump 1 strictEqual a b",
               "end",
    ]);

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    const 0 = 1;
    const false = true;
    goto :x !!a === b;
    :x
    end;
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
               "jump 1 strictEqual a b",
               "end",
    ]);

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    const 0 = 1;
    const false = true;
    goto :x !a === b;
    :x
    end;
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
               "op strictEqual __0 a b",
               "jump 2 equal __0 false",
               "end",
    ]);

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    const 0 = 1;
    const false = true;
    goto :x a !== b;
    :x
    end;
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
               "op strictEqual __0 a b",
               "jump 2 equal __0 false",
               "end",
    ]);

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    skip !_ && !_ {
        print true;
    }
    end;
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
               "jump 1 always 0 0",
               "print true",
               "end",
    ]);

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    skip _ && !_ {
        print true;
    }
    end;
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
               "print true",
               "end",
    ]);

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    skip !_ {
        print true;
    }
    end;
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
               "print true",
               "end",
    ]);
}

#[test]
fn line_test() {
    let parser = LogicLineParser::new();
    assert_eq!(parse!(parser, "noop;").unwrap(), LogicLine::NoOp);
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
    let parser = LogicLineParser::new();

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
        Select(
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
        ).into()
    );
    let tag_codes = CompileMeta::new()
        .compile(Expand(vec![ast]).into());
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
        Select(
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
        ).into()
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
        Select(
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
        ).into()
    );

    assert_eq!(
        parse!(parser, r#"
            switch 1 {
            print end;
            print end1;
            case 0: print 0;
            }
        "#).unwrap(),
        Select(
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
        ).into()
    );

}

#[test]
fn comments_test() {
    let parser = LogicLineParser::new();
    assert_eq!(
        parse!(parser, r#"
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
        "#
        ).unwrap(),
        LogicLine::NoOp
    );
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
    let src = r#"
    op x 1 + 2;
    op y (op $ x + 3;) * (op $ x * 2;);
    if (op tmp y & 1; op $ tmp + 1;) == 1 {
        print "a ";
    } else {
        print "b ";
    }
    print (op $ y + 3;);
    "#;
    let ast = parse!(parser, src).unwrap();
    let meta = CompileMeta::new();
    let tag_codes = meta.compile(ast);
    let logic_lines = tag_codes.compile().unwrap();
    assert_eq!(logic_lines, [
        r#"op add x 1 2"#,
        r#"op add __0 x 3"#,
        r#"op mul __1 x 2"#,
        r#"op mul y __0 __1"#,
        r#"op and tmp y 1"#,
        r#"op add __2 tmp 1"#,
        r#"jump 9 equal __2 1"#,
        r#"print "b ""#,
        r#"jump 10 always 0 0"#,
        r#"print "a ""#,
        r#"op add __3 y 3"#,
        r#"print __3"#,
    ])
}

#[test]
fn compile_take_test() {
    let parser = LogicLineParser::new();
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

    let src = r#"
    x = C;
    const C = (read $ cell1 0;);
    y = C;
    "#;
    let ast = parse!(parser, src).unwrap();
    let meta = CompileMeta::new();
    let tag_codes = meta.compile(ast);
    let logic_lines = tag_codes.compile().unwrap();
    assert_eq!(logic_lines, vec![
               "set x C",
               "read __0 cell1 0",
               "set y __0",
    ]);

    let src = r#"
    x = C;
    const C = (k: read k cell1 0;);
    y = C;
    "#;
    let ast = parse!(parser, src).unwrap();
    let meta = CompileMeta::new();
    let tag_codes = meta.compile(ast);
    let logic_lines = tag_codes.compile().unwrap();
    assert_eq!(logic_lines, vec![
               "set x C",
               "read k cell1 0",
               "set y k",
    ]);

    let src = r#"
    x = C;
    const C = (read $ cell1 0;);
    foo a b C d C;
    "#;
    let ast = parse!(parser, src).unwrap();
    let meta = CompileMeta::new();
    let tag_codes = meta.compile(ast);
    let logic_lines = tag_codes.compile().unwrap();
    assert_eq!(logic_lines, vec![
               "set x C",
               "read __0 cell1 0",
               "read __1 cell1 0",
               "foo a b __0 d __1",
    ]);

    let src = r#"
    const C = (m: read $ cell1 0;);
    x = C;
    "#;
    let ast = parse!(parser, src).unwrap();
    let meta = CompileMeta::new();
    let tag_codes = meta.compile(ast);
    let logic_lines = tag_codes.compile().unwrap();
    assert_eq!(logic_lines, vec![
               "read m cell1 0",
               "set x m",
    ]);

    let src = r#"
    const C = (read $ cell1 (i: read $ cell2 0;););
    print C;
    "#;
    let ast = parse!(parser, src).unwrap();
    let meta = CompileMeta::new();
    let tag_codes = meta.compile(ast);
    let logic_lines = tag_codes.compile().unwrap();
    assert_eq!(logic_lines, vec![
               "read i cell2 0",
               "read __0 cell1 i",
               "print __0",
    ]);
}

#[test]
fn const_value_block_range_test() {
    let parser = TopLevelParser::new();

    let src = r#"
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
    "#;
    let ast = parse!(parser, src).unwrap();
    let meta = CompileMeta::new();
    let tag_codes = meta.compile(ast);
    let logic_lines = tag_codes.compile().unwrap();
    assert_eq!(logic_lines, vec![
               "set x C",
               "read __0 cell3 0",
               "set m __0",
               "read __1 cell2 0",
               "set y __1",
               "read __2 cell2 0",
               "read __3 cell2 0",
               "foo __2 __3",
               "set z C",
    ]);
}

#[test]
fn take_test() {
    let parser = TopLevelParser::new();

    let src = r#"
    print start;
    const F = (read $ cell1 0;);
    take V = F; # 求值并映射
    print V;
    print V; # 再来一次
    foo V V;
    take V1 = F; # 再求值并映射
    print V1;
    "#;
    let ast = parse!(parser, src).unwrap();
    let meta = CompileMeta::new();
    let tag_codes = meta.compile(ast);
    let logic_lines = tag_codes.compile().unwrap();
    assert_eq!(logic_lines, vec![
               "print start",
               "read __0 cell1 0",
               "print __0",
               "print __0",
               "foo __0 __0",
               "read __1 cell1 0",
               "print __1",
    ]);

    let src = r#"
    const F = (m: read $ cell1 0;);
    take V = F; # 求值并映射
    print V;
    "#;
    let ast = parse!(parser, src).unwrap();
    let meta = CompileMeta::new();
    let tag_codes = meta.compile(ast);
    let logic_lines = tag_codes.compile().unwrap();
    assert_eq!(logic_lines, vec![
               "read m cell1 0",
               "print m",
    ]);

    let src = r#"
    take X = 2;
    take Y = `X`;
    const Z = `X`;
    print Y Z;
    "#;
    let ast = parse!(parser, src).unwrap();
    let meta = CompileMeta::new();
    let tag_codes = meta.compile(ast);
    let logic_lines = tag_codes.compile().unwrap();
    assert_eq!(logic_lines, vec![
               "print X",
               "print X",
    ]);

    let src = r#"
    take X = 2;
    take Y = X;
    const Z = X;
    print Y Z;
    "#;
    let ast = parse!(parser, src).unwrap();
    let meta = CompileMeta::new();
    let tag_codes = meta.compile(ast);
    let logic_lines = tag_codes.compile().unwrap();
    assert_eq!(logic_lines, vec![
               "print 2",
               "print 2",
    ]);

    let src = r#"
    const 2 = 3;
    take X = `2`;
    take Y = X;
    const Z = X;
    print Y Z;
    "#;
    let ast = parse!(parser, src).unwrap();
    let meta = CompileMeta::new();
    let tag_codes = meta.compile(ast);
    let logic_lines = tag_codes.compile().unwrap();
    assert_eq!(logic_lines, vec![
               "print 2",
               "print 2",
    ]);

    assert_eq!(
        parse!(parser, r#"
        take+A+B+C+D;
        "#).unwrap(),
        parse!(parser, r#"
        inline {
            take A = ();
            take B = ();
            take C = ();
            take D = ();
        }
        "#).unwrap(),
    );
}

#[test]
fn print_test() {
    let parser = TopLevelParser::new();

    let src = r#"
    print "abc" "def" "ghi" j 123 @counter;
    "#;
    let ast = parse!(parser, src).unwrap();
    let meta = CompileMeta::new();
    let tag_codes = meta.compile(ast);
    let logic_lines = tag_codes.compile().unwrap();
    assert_eq!(logic_lines, vec![
               r#"print "abc""#,
               r#"print "def""#,
               r#"print "ghi""#,
               r#"print j"#,
               r#"print 123"#,
               r#"print @counter"#,
    ]);

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

    let mut meta = Meta::new();
    let ast = parser.parse(&mut meta, r#"
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
    "#).unwrap();
    let compile_meta = CompileMeta::new();
    let tag_codes = compile_meta.compile(ast);
    let logic_lines = tag_codes.compile().unwrap();
    assert_eq!(
        logic_lines,
        vec![
            r#"jump 3 lessThan num 2"#,
            r#"print "num >= 2""#,
            r#"jump 0 always 0 0"#,
            r#"print "num < 2""#,
            r#"jump 0 always 0 0"#,
            r#"jump 8 lessThan num 2"#,
            r#"print "num >= 2""#,
            r#"jump 0 always 0 0"#,
            r#"print "num < 2""#,
            r#"jump 0 always 0 0"#,
        ]
    );

    let mut meta = Meta::new();
    let ast = parser.parse(&mut meta, r#"
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
    "#).unwrap();
    let compile_meta = CompileMeta::new();
    let tag_codes = compile_meta.compile(ast);
    let logic_lines = tag_codes.compile().unwrap();
    assert_eq!(
        logic_lines,
        vec![
            r#"op add __2 1 1"#,
            r#"set i __2"#,
            r#"jump 4 always 0 0"#,
            r#"print "skiped""#,
            r#"print "in a""#,
            r#"op add i i 1"#,
            r#"jump 4 lessThan i 5"#,
        ]
    );
}

#[test]
fn dexp_result_handle_use_const_test() {
    let parser = TopLevelParser::new();

    let ast = parse!(parser, r#"
    {
        print (R: $ = 2;);
        const R = x;
        print (R: $ = 2;);
    }
    print (R: $ = 2;);
    "#).unwrap();
    let meta = CompileMeta::new();
    let tag_codes = meta.compile(ast);
    let logic_lines = tag_codes.compile().unwrap();
    assert_eq!(logic_lines, vec![
               "set R 2",
               "print R",
               "set x 2",
               "print x",
               "set R 2",
               "print R",
    ]);
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
}

#[test]
fn take_default_result_test() {
    let parser = LogicLineParser::new();

    let ast = parse!(parser, "take 2;").unwrap();
    assert_eq!(ast, Take("__".into(), "2".into()).into());
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
    let parser = LogicLineParser::new();

    let ast = parse!(parser, "take X;").unwrap();
    assert_eq!(ast, Take("__".into(), "X".into()).into());

    let ast = parse!(parser, "take R = X;").unwrap();
    assert_eq!(ast, Take("R".into(), "X".into()).into());

    let ast = parse!(parser, "take[] X;").unwrap();
    assert_eq!(ast, Take("__".into(), "X".into()).into());

    let ast = parse!(parser, "take[] R = X;").unwrap();
    assert_eq!(ast, Take("R".into(), "X".into()).into());

    let ast = parse!(parser, "take[1 2] R = X;").unwrap();
    assert_eq!(ast, Expand(vec![
            LogicLine::SetArgs(vec!["1".into(), "2".into()].into()),
            Take("R".into(), "X".into()).into(),
            LogicLine::ConstLeak("R".into()),
    ]).into());

    let ast = parse!(parser, "take[1 2] X;").unwrap();
    assert_eq!(ast, Expand(vec![
            LogicLine::SetArgs(vec!["1".into(), "2".into()].into()),
            Take("__".into(), "X".into()).into(),
    ]).into());
}

#[test]
fn take_args_test() {
    let parser = TopLevelParser::new();

    let ast = parse!(parser, r#"
    const M = (
        print _0 _1 _2;
        set $ 3;
    );
    take[1 2 3] M;
    take[4 5 6] R = M;
    print R;
    "#).unwrap();
    let meta = CompileMeta::new();
    let tag_codes = meta.compile(ast);
    let logic_lines = tag_codes.compile().unwrap();
    assert_eq!(logic_lines, vec![
               "print 1",
               "print 2",
               "print 3",
               "set __3 3",
               "print 4",
               "print 5",
               "print 6",
               "set __7 3",
               "print __7",
    ]);

    let ast = parse!(parser, r#"
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
    "#).unwrap();
    let meta = CompileMeta::new();
    let tag_codes = meta.compile(ast);
    let logic_lines = tag_codes.compile().unwrap();
    assert_eq!(logic_lines, vec![
               r#"print "loop""#,
               r#"print "start""#,
               r#"set i 0"#,
               r#"jump 7 greaterThanEq i 10"#,
               r#"print i"#,
               r#"op add i i 1"#,
               r#"jump 4 lessThan i 10"#,
               r#"print "loop""#,
               r#"print "start*2""#,
               r#"set i 0"#,
               r#"jump 14 greaterThanEq i 10"#,
               r#"print i"#,
               r#"op add i i 1"#,
               r#"jump 11 lessThan i 10"#,
               r#"printflush message1"#,
    ]);
}

#[test]
fn sets_test() {
    let parser = TopLevelParser::new();

    let ast = parse!(parser, r#"
    a b c = 1 2 ({}op $ 2 + 1;);
    "#).unwrap();
    let meta = CompileMeta::new();
    let tag_codes = meta.compile(ast);
    let logic_lines = tag_codes.compile().unwrap();
    assert_eq!(logic_lines, vec![
               "set a 1",
               "set b 2",
               "op add __0 2 1",
               "set c __0",
    ]);

    assert!(parse!(parser, r#"
    a b c = 1 2;
    "#).is_err());

    assert!(parse!(parser, r#"
    a = 1 2;
    "#).is_err());

    assert!(parse!(parser, r#"
     = 1 2;
    "#).is_err());
}

#[test]
fn const_value_clone_test() {
    let parser = TopLevelParser::new();

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    const A = 1;
    const B = A;
    const A = 2;
    print A B;
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
               "print 2",
               "print 1",
    ]);

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    const A = 1;
    const B = A;
    const A = 2;
    const C = B;
    const B = 3;
    const B = B;
    print A B C;
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
               "print 2",
               "print 3",
               "print 1",
    ]);

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    const A = B;
    const B = 2;
    print A;
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
               "print B",
    ]);

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    const A = B;
    const B = 2;
    const A = A;
    print A;
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
               "print B",
    ]);

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    const A = B;
    const B = 2;
    {
        const A = A;
        print A;
    }
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
               "print B",
    ]);

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    const A = B;
    {
        const B = 2;
        const A = A;
        print A;
    }
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
               "print B",
    ]);

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    const A = B;
    {
        const B = 2;
        print A;
    }
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
               "print B",
    ]);

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    const A = B;
    const B = C;
    const C = A;
    print C;
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
               "print B",
    ]);

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    const A = C;
    const C = 2;
    const B = A;
    const A = 3;
    const C = B;
    print C;
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
               "print C",
    ]);
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

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    goto :end a && b;
    foo;
    :end
    end;
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
               "jump 2 equal a false",
               "jump 3 notEqual b false",
               "foo",
               "end",
    ]);

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    goto :end (a || b) && c;
    foo;
    :end
    end;
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
               "jump 2 notEqual a false",
               "jump 3 equal b false",
               "jump 4 notEqual c false",
               "foo",
               "end",
    ]);

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    goto :end (a || b) && (c || d);
    foo;
    :end
    end;
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
               "jump 2 notEqual a false",
               "jump 4 equal b false",
               "jump 5 notEqual c false",
               "jump 5 notEqual d false",
               "foo",
               "end",
    ]);

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    goto :end a || b || c || d || e;
    foo;
    :end
    end;
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
               "jump 6 notEqual a false",
               "jump 6 notEqual b false",
               "jump 6 notEqual c false",
               "jump 6 notEqual d false",
               "jump 6 notEqual e false",
               "foo",
               "end",
    ]);

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    goto :end a && b && c && d && e;
    foo;
    :end
    end;
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
               "jump 5 equal a false",
               "jump 5 equal b false",
               "jump 5 equal c false",
               "jump 5 equal d false",
               "jump 6 notEqual e false",
               "foo",
               "end",
    ]);

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    goto :end (a && b && c) && d && e;
    foo;
    :end
    end;
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
               "jump 5 equal a false",
               "jump 5 equal b false",
               "jump 5 equal c false",
               "jump 5 equal d false",
               "jump 6 notEqual e false",
               "foo",
               "end",
    ]);

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    goto :end a && b && (c && d && e);
    foo;
    :end
    end;
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
               "jump 5 equal a false",
               "jump 5 equal b false",
               "jump 5 equal c false",
               "jump 5 equal d false",
               "jump 6 notEqual e false",
               "foo",
               "end",
    ]);

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    goto :end a && (op $ b && c;);
    foo;
    :end
    end;
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
               "jump 3 equal a false",
               "op land __0 b c",
               "jump 4 notEqual __0 false",
               "foo",
               "end",
    ]);

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    goto :end a && b || c && d;
    foo;
    :end
    end;
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
               "jump 2 equal a false",
               "jump 5 notEqual b false",
               "jump 4 equal c false",
               "jump 5 notEqual d false",
               "foo",
               "end",
    ]);

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    goto :end !a && b || c && d;
    foo;
    :end
    end;
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
               "jump 2 notEqual a false",
               "jump 5 notEqual b false",
               "jump 4 equal c false",
               "jump 5 notEqual d false",
               "foo",
               "end",
    ]);

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    goto :end (a && b) || !(c && d);
    foo;
    :end
    end;
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
               "jump 2 equal a false",
               "jump 5 notEqual b false",
               "jump 5 equal c false",
               "jump 5 equal d false",
               "foo",
               "end",
    ]);

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    goto :end (a && b && c) || (d && e);
    foo;
    :end
    end;
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
               "jump 3 equal a false",
               "jump 3 equal b false",
               "jump 6 notEqual c false",
               "jump 5 equal d false",
               "jump 6 notEqual e false",
               "foo",
               "end",
    ]);

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    goto :end (a && b || c) || (d && e);
    foo;
    :end
    end;
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
               "jump 2 equal a false",
               "jump 6 notEqual b false",
               "jump 6 notEqual c false",
               "jump 5 equal d false",
               "jump 6 notEqual e false",
               "foo",
               "end",
    ]);

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    goto :end ((a && b) || c) || (d && e);
    foo;
    :end
    end;
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
               "jump 2 equal a false",
               "jump 6 notEqual b false",
               "jump 6 notEqual c false",
               "jump 5 equal d false",
               "jump 6 notEqual e false",
               "foo",
               "end",
    ]);

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    goto :end (a && (b || c)) || (d && e);
    foo;
    :end
    end;
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
               "jump 3 equal a false",
               "jump 6 notEqual b false",
               "jump 6 notEqual c false",
               "jump 5 equal d false",
               "jump 6 notEqual e false",
               "foo",
               "end",
    ]);

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    goto :end (op $ a + 2;) && (op $ b + 2;);
    foo;
    :end
    end;
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
               "op add __0 a 2",
               "jump 4 equal __0 false",
               "op add __1 b 2",
               "jump 5 notEqual __1 false",
               "foo",
               "end",
    ]);
}

#[test]
fn set_res_test() {
    let parser = TopLevelParser::new();

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    print (setres (x: op $ 1 + 2;););
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
               "op add x 1 2",
               "print x",
    ]);

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    print (setres m;);
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
               "print m",
    ]);
}

#[test]
fn repr_var_test() {
    let parser = TopLevelParser::new();

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    print a;
    print `a`;
    const a = b;
    print a;
    print `a`;
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
               "print a",
               "print a",
               "print b",
               "print a",
    ]);
}

#[test]
fn select_test() {
    let parser = TopLevelParser::new();

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    select 1 {
        print 0;
        print;
    }
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
        "op add @counter @counter 1",
        "print 0",
    ]);

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    select 1 {
        print 0;
        print;
        print;
    }
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
        "op add @counter @counter 1",
        "print 0",
        "jump 0 always 0 0",
    ]);

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    select 1 {
        print 0;
        print;
        print 2;
    }
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
        "op add @counter @counter 1",
        "print 0",
        "jump 3 always 0 0",
        "print 2",
    ]);

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    select 1 {
        print 0;
        print 1 " is one!";
        print 2;
    }
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
        "op mul __0 1 2",
        "op add @counter @counter __0",
        "print 0",
        "jump 4 always 0 0",
        "print 1",
        "print \" is one!\"",
        "print 2",
    ]);

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    select x {
        print 0;
        print 1;
        print 2;
    }
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
        "op add @counter @counter x",
        "print 0",
        "print 1",
        "print 2",
    ]);

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    select (y: op $ x + 2;) {}
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
        "op add y x 2",
    ]);

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    select x {}
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, Vec::<&str>::new());

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    select m {
        print 0;
        print 1 " is one!" ", one!!" "\n";
        print 2;
    }
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![ // 跳转表式, 因为这样更省行数
        "op add @counter @counter m",
        "jump 4 always 0 0",
        "jump 5 always 0 0",
        "jump 9 always 0 0",
        "print 0",
        "print 1",
        "print \" is one!\"",
        "print \", one!!\"",
        "print \"\\n\"",
        "print 2",
    ]);
}

#[test]
fn switch_catch_test() {
    let parser = TopLevelParser::new();

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
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
    "#).unwrap()).compile().unwrap();
    // 这里由于是比较生成后的代码而不是语法树, 所以可以不用那么严谨
    assert_eq!(logic_lines, CompileMeta::new().compile(parse!(parser, r#"
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
    "#).unwrap()).compile().unwrap());

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    switch (op $ x + 2;) {
        end;
    case <!>:
        stop;
    case 1:
        print 1;
    case 3:
        print 3 "!";
    }
    "#).unwrap()).compile().unwrap();
    // 这里由于是比较生成后的代码而不是语法树, 所以可以不用那么严谨
    assert_eq!(logic_lines, CompileMeta::new().compile(parse!(parser, r#"
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
    "#).unwrap()).compile().unwrap());

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
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
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, CompileMeta::new().compile(parse!(parser, r#"
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
    "#).unwrap()).compile().unwrap());

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    switch (op $ x + 2;) {
        end;
    case !!!: # 捕获多个未命中也可以哦, 当然只有最后一个生效
        stop;
    case 1:
        print 1;
    case 3:
        print 3 "!";
    }
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, CompileMeta::new().compile(parse!(parser, r#"
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
    "#).unwrap()).compile().unwrap());

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
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
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, CompileMeta::new().compile(parse!(parser, r#"
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
    "#).unwrap()).compile().unwrap());

    let ast = parse!(parser, r#"
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
    "#).unwrap();
    assert_eq!(ast, parse!(parser, r#"
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
    "#).unwrap());

    let ast = parse!(parser, r#"
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
    "#).unwrap();
    assert_eq!(ast, parse!(parser, r#"
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
    "#).unwrap());

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    switch (op $ x + 2;) {
    case !:
        stop;
    case 1:
    case 3:
    }
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, CompileMeta::new().compile(parse!(parser, r#"
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
    "#).unwrap()).compile().unwrap());
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
            DExp::new("__".into(), vec![
                LogicLine::SetArgs(vec!["1".into(), "2".into()].into()),
                LogicLine::SetResultHandle("Foo".into()),
            ].into()).into(),
        ].into())].into(),
    );


    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    const Add = (
        take A = _0;
        take B = _1;
        op $ A + B;
    );
    print Add[1 2];
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
               "op add __2 1 2",
               "print __2",
    ]);

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
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
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
               "print enter",
               "op add __3 1 2",
               "print __3",
    ]);

    assert_eq!(
        parse!(parser, r#"
        const V = F->[A B C @]->V;
        "#).unwrap(),
        parse!(parser, r#"
        const V = F[A B C @]->$->V;
        "#).unwrap(),
    );

    assert_eq!(
        parse!(parser, r#"
        Foo! a b c @ d;
        "#).unwrap(),
        parse!(parser, r#"
        take Foo[a b c @ d];
        "#).unwrap(),
    );

}

#[test]
fn value_bind_test() {
    let parser = TopLevelParser::new();

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    const Jack = jack;
    Jack Jack.age = "jack" 18;
    print Jack Jack.age;
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
               "set jack \"jack\"",
               "set __0 18",
               "print jack",
               "print __0",
    ]);

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    print a.b.c;
    print a.b;
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
               "print __1",
               "print __0",
    ]);

    assert_eq!(
        parse!(parser, r#"
        take Foo = (%x).y;
        "#),
        parse!(parser, r#"
        take Foo = x.y;
        "#),
    );

    assert_eq!(
        parse!(parser, r#"
        print (%(print 1 2;)).x;
        "#),
        parse!(parser, r#"
        print (%print 1 2;%).x;
        "#),
    );

    assert_eq!(
        parse!(parser, r#"
        print (%(x: print 1 2;)).x;
        "#),
        parse!(parser, r#"
        print (%x: print 1 2;%).x;
        "#),
    );

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    print (%()).x;
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
               "print __1",
    ]);

    assert_eq!(
        parse!(parser, r#"
        print (%(%(%(print 2;))));
        "#),
        parse!(parser, r#"
        print (print 2;);
        "#),
    );
}

#[test]
fn no_string_var_test() {
    let parser = NoStringVarParser::new();

    assert!(parse!(parser, r#"1"#).is_ok());
    assert!(parse!(parser, r#"1.5"#).is_ok());
    assert!(parse!(parser, r#"sbosb"#).is_ok());
    assert!(parse!(parser, r#"0x1b"#).is_ok());
    assert!(parse!(parser, r#"@abc"#).is_ok());
    assert!(parse!(parser, r#"'My_name"s'"#).is_ok());
    assert!(parse!(parser, r#"'"no_str"'"#).is_ok());

    assert!(parse!(parser, r#""abc""#).is_err());
    assert!(parse!(parser, r#""""#).is_err());
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

    assert_eq!(
        parse!(parser, r#"
        x = max(1, 2);
        y = max(max(1, 2), max(3, max(4, 5)));
        "#).unwrap(),
        parse!(parser, r#"
        op x max 1 2;
        op y max (op $ max 1 2;) (op $ max 3 (op $ max 4 5;););
        "#).unwrap(),
    );

    assert_eq!(
        parse!(parser, r#"
        x = 1+2*3;
        y = (1+2)*3;
        z = 1+2+3;
        "#).unwrap(),
        parse!(parser, r#"
        op x 1 + (op $ 2 * 3;);
        op y (op $ 1 + 2;) * 3;
        op z (op $ 1 + 2;) + 3;
        "#).unwrap(),
    );

    assert_eq!(
        parse!(parser, r#"
        x = 1*max(2, 3);
        y = a & b | c & d & e | f;
        "#).unwrap(),
        parse!(parser, r#"
        op x 1 * (op $ max 2 3;);
        op y (op $ (op $ a & b;) | (op $ (op $ c & d;) & e;);) | f;
        "#).unwrap(),
    );

    assert_eq!(
        parse!(parser, r#"
        x = a**b**c; # pow的右结合
        y = -x;
        z = ~y;
        e = a !== b;
        "#).unwrap(),
        parse!(parser, r#"
        op x a ** (op $ b ** c;);
        op y `0` - x;
        op z ~y;
        op e (op $ a === b;) == `false`;
        "#).unwrap(),
    );

    assert_eq!(
        parse!(parser, r#"
        a, b, c = x, -y, z+2*3;
        "#).unwrap(),
        parse!(parser, r#"
        {
            a = x;
            b = -y;
            c = z+2*3;
        }
        "#).unwrap(),
    );

    assert_eq!(
        parse!(parser, r#"
        a, b, c = 1;
        "#).unwrap(),
        parse!(parser, r#"
        {
            take ___0 = a;
            ___0 = 1;
            b = ___0;
            c = ___0;
        }
        "#).unwrap(),
    );

    assert_eq!(
        parse!(parser, r#"
        x = a < b == c > d;
        x = a < b != c > d;
        x = a < b === c > d;
        x = a < b !== c > d;
        "#).unwrap(),
        parse!(parser, r#"
        op x (op $ a < b;) == (op $ c > d;);
        op x (op $ a < b;) != (op $ c > d;);
        op x (op $ a < b;) === (op $ c > d;);
        op x (op $ a < b;) !== (op $ c > d;);
        "#).unwrap(),
    );

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

    assert_eq!(
        parse!(parser, r#"
        x = (a < b) < c;
        "#).unwrap(),
        parse!(parser, r#"
        op x (op $ a < b;) < c;
        "#).unwrap(),
    );

    assert_eq!(
        parse!(parser, r#"
        x += 2;
        "#).unwrap(),
        parse!(parser, r#"
        {
            take ___0 = x;
            op ___0 ___0 + 2;
        }
        "#).unwrap(),
    );

    assert_eq!(
        parse!(parser, r#"
        x += y*z;
        "#).unwrap(),
        parse!(parser, r#"
        {
            take ___0 = x;
            op ___0 ___0 + (op $ y * z;);
        }
        "#).unwrap(),
    );

    assert_eq!(
        parse!(parser, r#"
        take Foo = (?a+b);
        "#).unwrap(),
        parse!(parser, r#"
        take Foo = ($ = a+b;);
        "#).unwrap(),
    );

    assert_eq!(
        parse!(parser, r#"
        take Foo = (?m: a+b);
        "#).unwrap(),
        parse!(parser, r#"
        take Foo = (m: $ = a+b;);
        "#).unwrap(),
    );

    assert_eq!(
        parse!(parser, r#"
        a, b += 2;
        "#).unwrap(),
        parse!(parser, r#"
        {
            take ___0 = 2;
            {take ___1 = a; ___1 = ___1 + ___0;}
            {take ___2 = b; ___2 = ___2 + ___0;}
        }
        "#).unwrap(),
    );

    assert_eq!(
        parse!(parser, r#"
        a, b += 2, 3;
        "#).unwrap(),
        parse!(parser, r#"
        {
            a += 2;
            b += 3;
        }
        "#).unwrap(),
    );

    assert_eq!(
        parse!(parser, r#"
        a, b min= 2, 3;
        "#).unwrap(),
        parse!(parser, r#"
        {
            a min= 2;
            b min= 3;
        }
        "#).unwrap(),
    );

    assert_eq!(
        parse!(parser, r#"
        a, b max= 2, 3;
        "#).unwrap(),
        parse!(parser, r#"
        {
            a max= 2;
            b max= 3;
        }
        "#).unwrap(),
    );

    assert_eq!(
        parse!(parser, r#"
        x min= 2;
        "#).unwrap(),
        parse!(parser, r#"
        {
            take ___0 = x;
            op ___0 min ___0 2;
        }
        "#).unwrap(),
    );

    assert_eq!(
        parse!(parser, r#"
        a, b min= 2;
        "#).unwrap(),
        parse!(parser, r#"
        {
            take ___0 = 2;
            {take ___1 = a; op ___1 min ___1 ___0;}
            {take ___2 = b; op ___2 min ___2 ___0;}
        }
        "#).unwrap(),
    );

    assert_eq!(
        parse!(parser, r#"
        x = ++i;
        "#).unwrap(),
        parse!(parser, r#"
        x = (__:
            setres i;
            $ = $ + `1`;
        );
        "#).unwrap(),
    );

    assert_eq!(
        parse!(parser, r#"
        x = --i;
        "#).unwrap(),
        parse!(parser, r#"
        x = (__:
            setres i;
            $ = $ - `1`;
        );
        "#).unwrap(),
    );

    assert_eq!(
        parse!(parser, r#"
        x = i++;
        "#).unwrap(),
        parse!(parser, r#"
        {
            take ___0 = i;
            x = ___0;
            ___0 = ___0 + `1`;
        }
        "#).unwrap(),
    );

    assert_eq!(
        parse!(parser, r#"
        x = 2 + ++i;
        "#).unwrap(),
        parse!(parser, r#"
        x = 2 + (__:
            setres i;
            $ = $ + `1`;
        );
        "#).unwrap(),
    );

    assert_eq!(
        parse!(parser, r#"
        x = 2 + --i;
        "#).unwrap(),
        parse!(parser, r#"
        x = 2 + (__:
            setres i;
            $ = $ - `1`;
        );
        "#).unwrap(),
    );

    assert_eq!(
        parse!(parser, r#"
        x = 2 + i++;
        "#).unwrap(),
        parse!(parser, r#"
        x = 2 + (
            take ___0 = i;
            $ = ___0;
            ___0 = ___0 + `1`;
        );
        "#).unwrap(),
    );

    assert_eq!(
        parse!(parser, r#"
        x = i++(2+_);
        "#).unwrap(),
        parse!(parser, r#"
        {
            take ___0 = i;
            x = 2 + ___0;
            ___0 = ___0 + `1`;
        }
        "#).unwrap(),
    );

    assert_eq!(
        parse!(parser, r#"
        x = 8+i++(2+_);
        "#).unwrap(),
        parse!(parser, r#"
        x = 8 + (
            take ___0 = i;
            $ = 2 + ___0;
            ___0 = ___0 + `1`;
        );
        "#).unwrap(),
    );

    assert_eq!(
        parse!(parser, r#"
        x = 8+i++(j++(_) + _);
        "#).unwrap(),
        parse!(parser, r#"
        x = 8 + (
            take ___0 = i;
            $ = (
                take ___1 = j;
                $ = ___1;
                ___1 = ___1 + `1`;
            ) + ___0;
            ___0 = ___0 + `1`;
        );
        "#).unwrap(),
    );

    assert_eq!(
        parse!(parser, r#"
        print (?++i);
        "#).unwrap(),
        parse!(parser, r#"
        print ($ = (__: setres i; $ = $ + `1`;););
        "#).unwrap(),
    );

    assert_eq!(
        parse!(parser, r#"
        print (*++i);
        "#).unwrap(),
        parse!(parser, r#"
        print (__: setres i; $ = $ + `1`;);
        "#).unwrap(),
    );

    assert_eq!(
        parse!(parser, r#"
        take*A, B = x+y, i++;
        take*C = j--;
        "#).unwrap(),
        parse!(parser, r#"
        take A = (*x+y) B = (*i++);
        take C = (*j--);
        "#).unwrap(),
    );
}

#[test]
fn op_test() {
    let parser = TopLevelParser::new();

    assert_eq!(
        parse!(parser, r#"
        op x a !== b;
        "#).unwrap(),
        parse!(parser, r#"
        op x (op $ a === b;) == `false`;
        "#).unwrap(),
    );

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

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    print A;
    inline {
        const A = 2;
        print A;
    }
    print A;
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
               "print A",
               "print 2",
               "print 2",
    ]);
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
                DExp::new(
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
                        LogicLine::SetResultHandle("___0".into()),
                    ].into()
                ).into()
            ].into()),
        ]).into()
    );
    dbg!(1);

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
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
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
               "jump 3 lessThan a b",
               "print 2",
               "jump 4 always 0 0",
               "print 1",
               "jump 7 lessThan a b",
               "print 2",
               "jump 0 always 0 0",
               "print 1",
    ]);
    dbg!(2);

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
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
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
               "jump 3 lessThan a b",
               "print 2",
               "jump 4 always 0 0",
               "print 1",
               "jump 7 lessThan a b",
               "print 2",
               "jump 0 always 0 0",
               "print 1",
    ]);
    dbg!(3);

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
    dbg!(4);

    assert_eq!(
        parse!(parser, r#"
        foo const(:x bar;);
        "#).unwrap(),
        parse!(parser, r#"
        foo const!(:x bar;);
        "#).unwrap(),
    );

    let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
    const x.Y = 2;
    print const!(%setres x;%).Y;
    "#).unwrap()).compile().unwrap();
    assert_eq!(logic_lines, vec![
               "print 2",
    ]);
}

#[test]
fn inline_cmp_op_test() {
    let parser = TopLevelParser::new();

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        :0 goto :0 a < b;
        "#).unwrap()).compile().unwrap(),
        vec![
            "jump 0 lessThan a b"
        ]
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        :0 goto :0 a < b;
        "#).unwrap()).compile().unwrap(),
        CompileMeta::new().compile(parse!(parser, r#"
        :0 goto :0 a < b;
        "#).unwrap()).compile().unwrap(),
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        :0 goto :0 (op $ a < b;);
        "#).unwrap()).compile().unwrap(),
        CompileMeta::new().compile(parse!(parser, r#"
        :0 goto :0 a < b;
        "#).unwrap()).compile().unwrap(),
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        :0 goto :0 (op $ a === b;);
        "#).unwrap()).compile().unwrap(),
        CompileMeta::new().compile(parse!(parser, r#"
        :0 goto :0 a === b;
        "#).unwrap()).compile().unwrap(),
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        :0 goto :0 (x: op x a === b;);
        "#).unwrap()).compile().unwrap(),
        CompileMeta::new().compile(parse!(parser, r#"
        :0 goto :0 (x: op x a === b;);
        "#).unwrap()).compile().unwrap(),
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        :0 goto :0 (op x a === b;);
        "#).unwrap()).compile().unwrap(),
        CompileMeta::new().compile(parse!(parser, r#"
        :0 goto :0 (op x a === b;);
        "#).unwrap()).compile().unwrap(),
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        :0 goto :0 (x: op $ a === b;);
        "#).unwrap()).compile().unwrap(),
        CompileMeta::new().compile(parse!(parser, r#"
        :0 goto :0 (x: op $ a === b;);
        "#).unwrap()).compile().unwrap(),
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        :0 goto :0 !(op $ a < b;);
        "#).unwrap()).compile().unwrap(),
        CompileMeta::new().compile(parse!(parser, r#"
        :0 goto :0 !a < b;
        "#).unwrap()).compile().unwrap(),
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        :0 goto :0 !!!(op $ a < b;);
        "#).unwrap()).compile().unwrap(),
        CompileMeta::new().compile(parse!(parser, r#"
        :0 goto :0 !!!a < b;
        "#).unwrap()).compile().unwrap(),
    );

    // 暂未实现直接到StrictNotEqual, 目前这就算了吧, 反正最终编译产物一样
    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        :0 goto :0 !(op $ a === b;);
        "#).unwrap()).compile().unwrap(),
        CompileMeta::new().compile(parse!(parser, r#"
        :0 goto :0 a !== b;
        "#).unwrap()).compile().unwrap(),
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        :0 goto :0 (noop; op $ a < b;);
        "#).unwrap()).compile().unwrap(),
        CompileMeta::new().compile(parse!(parser, r#"
        :0 goto :0 (noop; op $ a < b;);
        "#).unwrap()).compile().unwrap(),
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        :0 goto :0 (op $ a < b; noop;);
        "#).unwrap()).compile().unwrap(),
        CompileMeta::new().compile(parse!(parser, r#"
        :0 goto :0 (op $ a < b; noop;);
        "#).unwrap()).compile().unwrap(),
    );

    assert_eq!( // 连续内联的作用
        CompileMeta::new().compile(parse!(parser, r#"
        :0 goto :0 (op $ !(op $ a < b;););
        "#).unwrap()).compile().unwrap(),
        CompileMeta::new().compile(parse!(parser, r#"
        :0 goto :0 !a < b;
        "#).unwrap()).compile().unwrap(),
    );

    assert_eq!( // 连续内联的作用
        CompileMeta::new().compile(parse!(parser, r#"
        :0 goto :0 (op $ !(op $ !(op $ a < b;);););
        "#).unwrap()).compile().unwrap(),
        CompileMeta::new().compile(parse!(parser, r#"
        :0 goto :0 a < b;
        "#).unwrap()).compile().unwrap(),
    );

    // 强化内联

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        const F = false;
        do {} while (op $ a < b;) != F;
        do {} while (op $ a < b;) == F;
        "#).unwrap()).compile().unwrap(),
        CompileMeta::new().compile(parse!(parser, r#"
        do {} while a < b;
        do {} while !a < b;
        "#).unwrap()).compile().unwrap(),
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        const F = 0;
        do {} while (op $ a < b;) != F;
        do {} while (op $ a < b;) == F;
        "#).unwrap()).compile().unwrap(),
        CompileMeta::new().compile(parse!(parser, r#"
        do {} while a < b;
        do {} while !a < b;
        "#).unwrap()).compile().unwrap(),
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        const F = 0;
        const Op = (op $ a < b;);
        do {} while Op != F;
        do {} while Op == F;
        "#).unwrap()).compile().unwrap(),
        CompileMeta::new().compile(parse!(parser, r#"
        do {} while a < b;
        do {} while !a < b;
        "#).unwrap()).compile().unwrap(),
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        const Op = (op $ a < b;);
        do {} while Op != (0:);
        do {} while Op == (0:);
        "#).unwrap()).compile().unwrap(),
        CompileMeta::new().compile(parse!(parser, r#"
        do {} while a < b;
        do {} while !a < b;
        "#).unwrap()).compile().unwrap(),
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        const F = (0:);
        const Op = (op $ a < b;);
        do {} while Op != F;
        do {} while Op == F;
        "#).unwrap()).compile().unwrap(),
        CompileMeta::new().compile(parse!(parser, r#"
        do {} while a < b;
        do {} while !a < b;
        "#).unwrap()).compile().unwrap(),
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        const F = (false:);
        const Op = (op $ a < b;);
        do {} while Op != F;
        do {} while Op == F;
        "#).unwrap()).compile().unwrap(),
        CompileMeta::new().compile(parse!(parser, r#"
        do {} while a < b;
        do {} while !a < b;
        "#).unwrap()).compile().unwrap(),
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        const F = 0;
        const Op = (op $ (op $ a < b;) != F;);
        do {} while Op != F;
        do {} while Op == F;
        "#).unwrap()).compile().unwrap(),
        CompileMeta::new().compile(parse!(parser, r#"
        do {} while a < b;
        do {} while !a < b;
        "#).unwrap()).compile().unwrap(),
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        const F = 0;
        const Op = (op $ (op $ a < b;) == F;);
        do {} while Op != F;
        do {} while Op == F;
        "#).unwrap()).compile().unwrap(),
        CompileMeta::new().compile(parse!(parser, r#"
        do {} while !a < b;
        do {} while a < b;
        "#).unwrap()).compile().unwrap(),
    );

}

#[test]
fn top_level_break_and_continue_test() {
    let parser = TopLevelParser::new();

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        foo;
        continue;
        bar;
        "#).unwrap()).compile().unwrap(),
        [
            "foo",
            "jump 0 always 0 0",
            "bar",
        ]
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        foo;
        continue _;
        bar;
        "#).unwrap()).compile().unwrap(),
        [
            "foo",
            "jump 0 always 0 0",
            "bar",
        ]
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        foo;
        continue a < b;
        bar;
        "#).unwrap()).compile().unwrap(),
        [
            "foo",
            "jump 0 lessThan a b",
            "bar",
        ]
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        foo;
        continue a < b || c < d;
        bar;
        "#).unwrap()).compile().unwrap(),
        [
            "foo",
            "jump 0 lessThan a b",
            "jump 0 lessThan c d",
            "bar",
        ]
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        foo;
        continue;
        bar;
        "#).unwrap()).compile().unwrap(),
        [
            "foo",
            "jump 0 always 0 0",
            "bar",
        ]
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        foo;
        continue _;
        bar;
        "#).unwrap()).compile().unwrap(),
        [
            "foo",
            "jump 0 always 0 0",
            "bar",
        ]
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        foo;
        continue a < b;
        bar;
        "#).unwrap()).compile().unwrap(),
        [
            "foo",
            "jump 0 lessThan a b",
            "bar",
        ]
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        foo;
        continue a < b || c < d;
        bar;
        "#).unwrap()).compile().unwrap(),
        [
            "foo",
            "jump 0 lessThan a b",
            "jump 0 lessThan c d",
            "bar",
        ]
    );

}

#[test]
fn control_stmt_break_and_continue_test() {
    let parser = TopLevelParser::new();

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
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
        "#).unwrap()).compile().unwrap(),
        [
            "foo",
            "jump 10 greaterThanEq a b",
            "foo1",
            "jump 7 greaterThanEq c d",
            "foo2",
            "jump 7 always 0 0",
            "jump 4 lessThan c d",
            "bar1",
            "jump 10 always 0 0",
            "jump 2 lessThan a b",
            "bar",
            "jump 0 always 0 0",
        ]
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
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
        "#).unwrap()).compile().unwrap(),
        [
            "foo",
            "jump 9 always 0 0",
            "foo1",
            "jump 6 always 0 0",
            "foo2",
            "jump 7 always 0 0",
            "jump 4 lessThan c d",
            "bar1",
            "jump 10 always 0 0",
            "jump 2 lessThan a b",
            "bar",
            "jump 0 always 0 0",
        ]
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
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
        "#).unwrap()).compile().unwrap(),
        [
            "foo",
            "xxx",
            "foo1",
            "xxx",
            "foo2",
            "jump 7 always 0 0",
            "jump 4 lessThan c d",
            "bar1",
            "jump 10 always 0 0",
            "jump 2 lessThan a b",
            "bar",
            "jump 0 always 0 0",
        ]
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        switch a {
        case 0: foo;
        case 1: break;
        case 2: bar;
        }
        end;
        break;
        "#).unwrap()).compile().unwrap(),
        [
            "op add @counter @counter a",
            "foo",
            "jump 4 always 0 0",
            "bar",
            "end",
            "jump 0 always 0 0",
        ]
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        select a {
            foo;
            break;
            bar;
        }
        end;
        break;
        "#).unwrap()).compile().unwrap(),
        [
            "op add @counter @counter a",
            "foo",
            "jump 4 always 0 0",
            "bar",
            "end",
            "jump 0 always 0 0",
        ]
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
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
        "#).unwrap()).compile().unwrap(),
        [
            "foo",
            "jump 10 greaterThanEq a b",
            "foo1",
            "jump 7 greaterThanEq c d",
            "foo2",
            "jump 6 always 0 0",
            "jump 4 lessThan c d",
            "bar1",
            "jump 9 always 0 0",
            "jump 2 lessThan a b",
            "bar",
            "jump 0 always 0 0",
        ]
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
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
        "#).unwrap()).compile().unwrap(),
        [
            "foo",
            "jump 10 greaterThanEq a b",
            "foo1",
            "jump 7 greaterThanEq c d",
            "jump 6 always 0 0",
            "foo2",
            "jump 6 lessThan c d", // 4 -> 6
            "bar1",
            "jump 9 always 0 0",
            "jump 2 lessThan a b",
            "bar",
            "jump 0 always 0 0",
        ]
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
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
        "#).unwrap()).compile().unwrap(),
        [
            "foo",
            "jump 9 always 0 0",
            "foo1",
            "jump 6 always 0 0",
            "foo2",
            "jump 6 always 0 0",
            "jump 4 lessThan c d",
            "bar1",
            "jump 9 always 0 0",
            "jump 2 lessThan a b",
            "bar",
            "jump 0 always 0 0",
        ]
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
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
        "#).unwrap()).compile().unwrap(),
        [
            "foo",
            "xxx",
            "foo1",
            "xxx",
            "foo2",
            "jump 6 always 0 0",
            "jump 4 lessThan c d",
            "bar1",
            "jump 9 always 0 0",
            "jump 2 lessThan a b",
            "bar",
            "jump 0 always 0 0",
        ]
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        switch a {
        case 0: foo;
        case 1: continue;
        case 2: bar;
        }
        end;
        continue;
        "#).unwrap()).compile().unwrap(),
        [
            "op add @counter @counter a",
            "foo",
            "jump 0 always 0 0",
            "bar",
            "end",
            "jump 0 always 0 0",
        ]
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        select a {
            foo;
            continue;
            bar;
        }
        end;
        continue;
        "#).unwrap()).compile().unwrap(),
        [
            "op add @counter @counter a",
            "foo",
            "jump 0 always 0 0",
            "bar",
            "end",
            "jump 0 always 0 0",
        ]
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        end;
        switch a {
        case 0: foo;
        case 1: continue;
        case 2: bar;
        }
        end;
        continue;
        "#).unwrap()).compile().unwrap(),
        [
            "end",
            "op add @counter @counter a",
            "foo",
            "jump 1 always 0 0",
            "bar",
            "end",
            "jump 0 always 0 0",
        ]
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        end;
        select a {
            foo;
            continue;
            bar;
        }
        end;
        continue;
        "#).unwrap()).compile().unwrap(),
        [
            "end",
            "op add @counter @counter a",
            "foo",
            "jump 1 always 0 0",
            "bar",
            "end",
            "jump 0 always 0 0",
        ]
    );

}

#[test]
fn op_expr_if_else_test() {
    let parser = TopLevelParser::new();

    assert_eq!(
        parse!(parser, r#"
        a = if b < c ? b + 2 : c;
        "#).unwrap(),
        parse!(parser, r#"
        {
            take ___0 = a;
            goto :___0 b < c;
            `set` ___0 c;
            goto :___1 _;
            :___0
            op ___0 b + 2;
            :___1
        }
        "#).unwrap()
    );

    assert_eq!(
        parse!(parser, r#"
        a = (if b < c ? b + 2 : c);
        "#).unwrap(),
        parse!(parser, r#"
        {
            take ___0 = a;
            goto :___0 b < c;
            `set` ___0 c;
            goto :___1 _;
            :___0
            op ___0 b + 2;
            :___1
        }
        "#).unwrap()
    );

    assert_eq!(
        parse!(parser, r#"
        a = if b < c ? b + 2 : if d < e ? 8 : c - 2;
        "#).unwrap(),
        parse!(parser, r#"
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
        "#).unwrap()
    );

    assert_eq!(
        parse!(parser, r#"
        a = 1 + (if b ? c : d);
        "#).unwrap(),
        parse!(parser, r#"
        op a 1 + (
            take ___0 = $;
            goto :___0 b;
            `set` ___0 d;
            goto :___1 _;
            :___0
            `set` ___0 c;
            :___1
        );
        "#).unwrap()
    );

}

#[test]
fn optional_jumpcmp_test() {
    let parser = TopLevelParser::new();

    assert_eq!(
        parse!(parser, r#"
        :x
        goto :x;
        "#).unwrap(),
        parse!(parser, r#"
        :x
        goto :x _;
        "#).unwrap()
    );

    assert_eq!(
        parse!(parser, r#"
        do {
            foo;
        } while;
        "#).unwrap(),
        parse!(parser, r#"
        do {
            foo;
        } while _;
        "#).unwrap()
    );

}

#[test]
fn control_block_test() {
    let parser = TopLevelParser::new();

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        a;
        break {
            b;
            break;
            c;
        }
        d;
        break;
        "#).unwrap()).compile().unwrap(),
        [
            "a",
            "b",
            "jump 4 always 0 0",
            "c",
            "d",
            "jump 0 always 0 0",
        ]
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        a;
        break! {
            b;
            break;
            c;
        }
        d;
        break;
        "#).unwrap()).compile().unwrap(),
        [
            "a",
            "b",
            "jump 1 always 0 0",
            "c",
            "d",
            "jump 0 always 0 0",
        ]
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        a;
        continue {
            b;
            continue;
            c;
        }
        d;
        continue;
        "#).unwrap()).compile().unwrap(),
        [
            "a",
            "b",
            "jump 1 always 0 0",
            "c",
            "d",
            "jump 0 always 0 0",
        ]
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        a;
        continue! {
            b;
            continue;
            c;
        }
        d;
        continue;
        "#).unwrap()).compile().unwrap(),
        [
            "a",
            "b",
            "jump 4 always 0 0",
            "c",
            "d",
            "jump 0 always 0 0",
        ]
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        a;
        continue {
            b;
            break;
            c;
        }
        d;
        continue;
        "#).unwrap()).compile().unwrap(),
        [
            "a",
            "b",
            "jump 0 always 0 0",
            "c",
            "d",
            "jump 0 always 0 0",
        ]
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        a;
        break {
            b;
            continue;
            c;
        }
        d;
        break;
        "#).unwrap()).compile().unwrap(),
        [
            "a",
            "b",
            "jump 0 always 0 0",
            "c",
            "d",
            "jump 0 always 0 0",
        ]
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        a;
        break continue {
            b;
            continue;
            break;
            c;
        }
        d;
        "#).unwrap()).compile().unwrap(),
        [
            "a",
            "b",
            "jump 1 always 0 0",
            "jump 5 always 0 0",
            "c",
            "d",
        ]
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        a;
        continue break {
            b;
            continue;
            break;
            c;
        }
        d;
        "#).unwrap()).compile().unwrap(),
        [
            "a",
            "b",
            "jump 1 always 0 0",
            "jump 5 always 0 0",
            "c",
            "d",
        ]
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        a;
        continue! break! {
            b;
            break;
            continue;
            c;
        }
        d;
        "#).unwrap()).compile().unwrap(),
        [
            "a",
            "b",
            "jump 1 always 0 0",
            "jump 5 always 0 0",
            "c",
            "d",
        ]
    );
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

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        const Cmp = goto(_0 < _1);
        do {} while ({const _0 = a; const _1 = b;} => Cmp);
        "#).unwrap()).compile().unwrap(),
        CompileMeta::new().compile(parse!(parser, r#"
        do {} while a < b;
        "#).unwrap()).compile().unwrap(),
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        const Cmp = goto(_0 < _1);
        do {} while !({const _0 = a; const _1 = b;} => Cmp);
        "#).unwrap()).compile().unwrap(),
        CompileMeta::new().compile(parse!(parser, r#"
        do {} while !a < b;
        "#).unwrap()).compile().unwrap(),
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        const Cmp = goto(_0 < _1);
        do {} while ({const _0 = a; const _1 = b;} => !Cmp);
        "#).unwrap()).compile().unwrap(),
        CompileMeta::new().compile(parse!(parser, r#"
        do {} while !a < b;
        "#).unwrap()).compile().unwrap(),
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        const Cmp = goto(!_0 < _1);
        do {} while ({const _0 = a; const _1 = b;} => Cmp);
        "#).unwrap()).compile().unwrap(),
        CompileMeta::new().compile(parse!(parser, r#"
        do {} while !a < b;
        "#).unwrap()).compile().unwrap(),
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        const Cmp = goto(!_0 < _1);
        do {} while ({const _0 = a; const _1 = b;} => !Cmp);
        "#).unwrap()).compile().unwrap(),
        CompileMeta::new().compile(parse!(parser, r#"
        do {} while a < b;
        "#).unwrap()).compile().unwrap(),
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        const Cmp = goto(_0 < _1);
        do {} while !(x && ({const _0 = a; const _1 = b;} => Cmp));
        "#).unwrap()).compile().unwrap(),
        CompileMeta::new().compile(parse!(parser, r#"
        do {} while !(x && a < b);
        "#).unwrap()).compile().unwrap(),
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        do {} while !(x && ({const _0 = a; const _1 = b;} => goto(_0 < _1)));
        "#).unwrap()).compile().unwrap(),
        CompileMeta::new().compile(parse!(parser, r#"
        do {} while !(x && a < b);
        "#).unwrap()).compile().unwrap(),
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        do {} while !(x && ({const _0 = a; const _1 = b;} => !goto(_0 < _1)));
        "#).unwrap()).compile().unwrap(),
        CompileMeta::new().compile(parse!(parser, r#"
        do {} while !(x && !a < b);
        "#).unwrap()).compile().unwrap(),
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        const x.Cmp = goto(.. < 2);
        const Cmp = x->Cmp;
        do {} while Cmp;
        "#).unwrap()).compile().unwrap(),
        CompileMeta::new().compile(parse!(parser, r#"
        do {} while x < 2;
        "#).unwrap()).compile().unwrap(),
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        const x.New = (
            setres ..;
            const $.Cmp = goto(.. < 2);
        );
        const Cmp = x->New->Cmp;
        do {} while Cmp;
        "#).unwrap()).compile().unwrap(),
        CompileMeta::new().compile(parse!(parser, r#"
        do {} while x < 2;
        "#).unwrap()).compile().unwrap(),
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        const x.New = (
            setres ..;
            const $.Cmp = goto({print ..;} => .. < 2);
        );
        const Cmp = x->New->Cmp;
        do {} while Cmp;
        "#).unwrap()).compile().unwrap(),
        CompileMeta::new().compile(parse!(parser, r#"
        do {print x;} while x < 2;
        "#).unwrap()).compile().unwrap(),
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        const x.Cmp = goto(.. < 2);
        const Cmp = x->Cmp;
        do {} while Cmp;
        do {} while Cmp;
        "#).unwrap()).compile().unwrap(),
        CompileMeta::new().compile(parse!(parser, r#"
        do {} while x < 2;
        do {} while x < 2;
        "#).unwrap()).compile().unwrap(),
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        const x.Cmp = goto({
            do {} while;
        } => .. < 2);
        const Cmp = x->Cmp;
        do {} while Cmp;
        do {} while Cmp;
        "#).unwrap()).compile().unwrap(),
        CompileMeta::new().compile(parse!(parser, r#"
        do {do {} while;} while x < 2;
        do {do {} while;} while x < 2;
        "#).unwrap()).compile().unwrap(),
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        const x.Cmp = goto({
            do {} while;
        } => .. < 2);
        const Cmp = x->Cmp;
        do {} while Cmp;
        do {} while Cmp;
        print ..;
        "#).unwrap()).compile().unwrap(),
        CompileMeta::new().compile(parse!(parser, r#"
        do {do {} while;} while x < 2;
        do {do {} while;} while x < 2;
        print __;
        "#).unwrap()).compile().unwrap(),
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        break ({match 1 2 3 => @ {}} => _);
        print _0 @;
        "#).unwrap()).compile().unwrap(),
        vec![
            "jump 0 always 0 0",
            "print _0",
        ]
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        break (=>[1 2 3] _);
        print _0 @;
        "#).unwrap()).compile().unwrap(),
        vec![
            "jump 0 always 0 0",
            "print _0",
        ]
    );
}

#[test]
fn mul_takes_test() {
    let parser = TopLevelParser::new();

    assert_eq!(
        parse!(parser, r#"
        take A=B C=D E=F G=I;
        "#).unwrap(),
        parse!(parser, r#"
        inline {
            take A = B;
            take C = D;
            take E = F;
            take G = I;
        }
        "#).unwrap(),
    );
}

#[test]
fn mul_consts_test() {
    let parser = TopLevelParser::new();

    assert_eq!(
        parse!(parser, r#"
        const A=B C=D E=F G=I;
        "#).unwrap(),
        parse!(parser, r#"
        inline {
            const A = B;
            const C = D;
            const E = F;
            const G = I;
        }
        "#).unwrap(),
    );

    assert_eq!( // label
        parse!(parser, r#"
        const A=(:m goto :m;) C=(if x {});
        "#).unwrap(),
        parse!(parser, r#"
        inline {
            const A = (:m goto :m;);
            const C = (if x {});
        }
        "#).unwrap(),
    );
}

#[test]
fn switch_ignored_id_test() {
    let parser = TopLevelParser::new();

    assert_eq!(
        parse!(parser, r#"
        switch x {
            case: foo;
            case: bar;
            case: baz;
        }
        "#).unwrap(),
        parse!(parser, r#"
        switch x {
            case 0: foo;
            case 1: bar;
            case 2: baz;
        }
        "#).unwrap(),
    );

    assert_eq!(
        parse!(parser, r#"
        switch x {
            case 1: foo;
            case: bar;
            case: baz;
        }
        "#).unwrap(),
        parse!(parser, r#"
        switch x {
            case 1: foo;
            case 2: bar;
            case 3: baz;
        }
        "#).unwrap(),
    );

    assert_eq!(
        parse!(parser, r#"
        switch x {
            case: foo;
            case 2: bar;
            case: baz;
        }
        "#).unwrap(),
        parse!(parser, r#"
        switch x {
            case 0: foo;
            case 2: bar;
            case 3: baz;
        }
        "#).unwrap(),
    );

    assert_eq!(
        parse!(parser, r#"
        switch x {
            case 0 2 4: foo;
            case: bar;
            case: baz;
        }
        "#).unwrap(),
        parse!(parser, r#"
        switch x {
            case 0 2 4: foo;
            case 5: bar;
            case 6: baz;
        }
        "#).unwrap(),
    );
}

#[test]
fn switch_append_tail_once_test() {
    let parser = TopLevelParser::new();

    // switch填充行仅最后一个进行填充

    assert_eq!(
        parse!(parser, r#"
        switch x {
            break;
            case 6:
                foo;
            case 3:
                bar;
        }
        "#).unwrap(),
        parse!(parser, r#"
        select x {
            print; # ignore
            print;
            { break; } # switch的封装以限制作用域
            { bar; break; }
            print;
            { break; }
            { foo; break; }
        }
        "#).unwrap(),
    );
}

#[test]
fn const_expr_eval_test() {
    let parser = TopLevelParser::new();

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        print 1.00;
        print `1.00`;
        print (1.00:);
        "#).unwrap()).compile().unwrap(),
        CompileMeta::new().compile(parse!(parser, r#"
        print 1.00;
        print 1.00;
        print 1.00;
        "#).unwrap()).compile().unwrap(),
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        print ($ = 1.00;);
        "#).unwrap()).compile().unwrap(),
        CompileMeta::new().compile(parse!(parser, r#"
        print 1;
        "#).unwrap()).compile().unwrap(),
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        take N = (op $ (op $ (op $ 1 + 2;) + 3;) + 4;);
        print N;
        "#).unwrap()).compile().unwrap(),
        CompileMeta::new().compile(parse!(parser, r#"
        print `10`;
        "#).unwrap()).compile().unwrap(),
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        take N = ($ = 1 + 2 + 3 + 4;);
        print N;
        "#).unwrap()).compile().unwrap(),
        CompileMeta::new().compile(parse!(parser, r#"
        print `10`;
        "#).unwrap()).compile().unwrap(),
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        take N = ($ = 1 << 10;);
        print N;
        "#).unwrap()).compile().unwrap(),
        CompileMeta::new().compile(parse!(parser, r#"
        take N = 1024;
        print N;
        "#).unwrap()).compile().unwrap(),
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        take N = ($ = 1.0 == 1;);
        print N;
        "#).unwrap()).compile().unwrap(),
        CompileMeta::new().compile(parse!(parser, r#"
        take N = 1;
        print N;
        "#).unwrap()).compile().unwrap(),
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        take N = (m: $ = 1.0 == 1;);
        print N;
        "#).unwrap()).compile().unwrap(),
        CompileMeta::new().compile(parse!(parser, r#"
        m = 1.0 == 1;
        print m;
        "#).unwrap()).compile().unwrap(),
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        print ($ = (`set` $ ($ = 1 + 1;);););
        "#).unwrap()).compile().unwrap(),
        CompileMeta::new().compile(parse!(parser, r#"
        print 2;
        "#).unwrap()).compile().unwrap(),
    );

    assert_eq!( // 非匿名句柄不优化
        CompileMeta::new().compile(parse!(parser, r#"
        print (x: $ = 1 + 1;);
        "#).unwrap()).compile().unwrap(),
        CompileMeta::new().compile(parse!(parser, r#"
        op x 1 + 1;
        print x;
        "#).unwrap()).compile().unwrap(),
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        print ($ = log(0););
        "#).unwrap()).compile().unwrap(),
        CompileMeta::new().compile(parse!(parser, r#"
        print null;
        "#).unwrap()).compile().unwrap(),
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        print ($ = acos(0.5););
        print ($ = cos(60););
        "#).unwrap()).compile().unwrap(),
        CompileMeta::new().compile(parse!(parser, r#"
        print 60.00000000000001;
        print 0.5000000000000001;
        "#).unwrap()).compile().unwrap(),
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        print ($ = -3 // 2;);
        "#).unwrap()).compile().unwrap(),
        CompileMeta::new().compile(parse!(parser, r#"
        print -2;
        "#).unwrap()).compile().unwrap(),
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        const N=($ = -3 // 2;);
        const N1=($ = -N;);
        print N1;
        "#).unwrap()).compile().unwrap(),
        CompileMeta::new().compile(parse!(parser, r#"
        print 2;
        "#).unwrap()).compile().unwrap(),
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        const N=($ = -3 // 2;);
        take N1=($ = -N;);
        print N1;
        "#).unwrap()).compile().unwrap(),
        CompileMeta::new().compile(parse!(parser, r#"
        print 2;
        "#).unwrap()).compile().unwrap(),
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        const N=($ = -3 // 2;);
        take N=($ = -N;);
        print N;
        "#).unwrap()).compile().unwrap(),
        CompileMeta::new().compile(parse!(parser, r#"
        print 2;
        "#).unwrap()).compile().unwrap(),
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        print ($ = -2 - 3;);
        print ($ = max(3, 5););
        print ($ = min(0, -2););
        print ($ = min(null, -2););
        print ($ = abs(null););
        print ($ = null;);
        "#).unwrap()).compile().unwrap(),
        CompileMeta::new().compile(parse!(parser, r#"
        print -5;
        print 5;
        print -2;
        print -2;
        print 0;
        print null;
        "#).unwrap()).compile().unwrap(),
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        print ($ = 999999+0;);
        print ($ = 999999+1;);
        print ($ = -999999 - 0;);
        print ($ = -999999 - 1;);
        print ($ = 1 - 1;);
        "#).unwrap()).compile().unwrap(),
        CompileMeta::new().compile(parse!(parser, r#"
        print 999999;
        print 0xF4240;
        print -999999;
        print 0x-F4240;
        print 0;
        "#).unwrap()).compile().unwrap(),
    );
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

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        const Foo = (
            inline@{
                print @;
            }
        );
        take Foo[a b c];
        "#).unwrap()).compile().unwrap(),
        vec![
            "print a",
            "print b",
            "print c",
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        const Foo = (
            print @;
        );
        take Foo[a b c];
        "#).unwrap()).compile().unwrap(),
        vec![
            "print a",
            "print b",
            "print c",
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        take Foo[a b c];
        print @;
        "#).unwrap()).compile().unwrap(),
        Vec::<&str>::new(),
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        match a b c {}
        print @;
        "#).unwrap()).compile().unwrap(),
        Vec::<&str>::new(),
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        match a b c { @{} }
        print @;
        "#).unwrap()).compile().unwrap(),
        vec![
            "print a",
            "print b",
            "print c",
        ],
    );

    assert_eq!( // 作用域测试
        CompileMeta::new().compile(parse!(parser, r#"
        {
            match a b c { @{} }
        }
        print @;
        "#).unwrap()).compile().unwrap(),
        Vec::<&str>::new(),
    );

    assert_eq!( // 作用域测试
        CompileMeta::new().compile(parse!(parser, r#"
        match a b c { @{} }
        inline@{
            print @;
        }
        print end;
        print @;
        "#).unwrap()).compile().unwrap(),
        vec![
            "print a",
            "print b",
            "print c",
            "print end",
            "print a",
            "print b",
            "print c",
        ],
    );

    assert_eq!( // 作用域测试
        CompileMeta::new().compile(parse!(parser, r#"
        match a b c { @{} }
        inline 2@{
            foo @;
        }
        print end;
        print @;
        "#).unwrap()).compile().unwrap(),
        vec![
            "foo a b",
            "foo c",
            "print end",
            "print a",
            "print b",
            "print c",
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        match a b c { __ @{} }
        print @;
        "#).unwrap()).compile().unwrap(),
        vec![
            "print b",
            "print c",
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        match a b c {
            X Y {}
            @{}
        }
        print @;
        "#).unwrap()).compile().unwrap(),
        vec![
            "print a",
            "print b",
            "print c",
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        match a b {
            X Y {}
            @{}
        }
        print @;
        "#).unwrap()).compile().unwrap(),
        Vec::<&str>::new(),
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        match a b {
            X Y {}
            @{}
        }
        print X Y;
        "#).unwrap()).compile().unwrap(),
        vec![
            "print a",
            "print b",
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
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
        "#).unwrap()).compile().unwrap(),
        vec![
            "print a",
            "print b",
            "print c",
            "print end",
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
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
        "#).unwrap()).compile().unwrap(),
        vec![
            "print c",
            "print b",
            "print a",
            "print end",
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
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
        "#).unwrap()).compile().unwrap(),
        vec![
            "print a",
            "print e",
            "print b",
            "print d",
            "print c",
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
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
        "#).unwrap()).compile().unwrap(),
        vec![
            "print a",
            "print f",
            "print b",
            "print e",
            "print c",
            "print d",
            "print end",
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
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
        "#).unwrap()).compile().unwrap(),
        vec![
            "print one",
            "print two",
            "print three_or_four",
            "print 3",
            "print three_or_four",
            "print 4",
            "print other",
            "print 5",
            "print other",
            "print 6",
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
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
        "#).unwrap()).compile().unwrap(),
        vec![
            "print one",
            "print two",
            "print three_or_four",
            "print 3",
            "print three_or_four",
            "print 4",
            "print other",
            "print 5",
            "print other",
            "print 6",
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
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
        "#).unwrap()).compile().unwrap(),
        vec![
            "print one",
            "print two",
            "print three_or_four",
            "print 3",
            "print three_or_four",
            "print 4",
            "print other",
            "print 5",
            "print other",
            "print 6",
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
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
        "#).unwrap()).compile().unwrap(),
        vec![
            "print equal",
            "print a",
            "print not_equal",
            "print a",
            "print b",
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
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
        "#).unwrap()).compile().unwrap(),
        vec![
            "print 5",
            "print 5",
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        # 验证其并不会连锁追溯
        const Foo = (
            take A = _0;
            print _0 A;
        );
        const X = 2;
        const Y = `X`;
        print Y;
        take Foo[Y];
        "#).unwrap()).compile().unwrap(),
        vec![
            "print X",
            "print X",
            "print X",
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
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
        "#).unwrap()).compile().unwrap(),
        vec![
            "print X",
            "print X",
            "print X",
            "print X",
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
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
        "#).unwrap()).compile().unwrap(),
        vec![
            "print X",
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        const 3 = 0;
        const X = 2;
        const Y = `3`;
        match X Y 4 {
            A:[`2`] B:[`3`] C:[4] {
                print A B C;
            }
        }
        "#).unwrap()).compile().unwrap(),
        vec![
            "print 2",
            "print 3",
            "print 4",
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
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
        "#).unwrap()).compile().unwrap(),
        vec![
            "print X",
            "print X",
            "print Y",
            "print Y",
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
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
        "#).unwrap()).compile().unwrap(),
        vec![
            "print X",
            "print X",
            "print Y",
            "print Y",
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
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
        "#).unwrap()).compile().unwrap(),
        vec![
            "print a",
            "print nouse",
            "print a",
            "print nouse",
            "print a",
            "print nouse",
        ],
    );

    assert_eq!(
        parse!(parser, r#"
        inline@ A B *C {
            print A B C;
        }
        "#).unwrap(),
        parse!(parser, r#"
        inline 3@{
            const match @ {
                A B *C {
                    print A B C;
                }
            }
        }
        "#).unwrap(),
    );

    assert_eq!(
        parse!(parser, r#"
        match x @ y => a @ b {
            body;
        }
        "#).unwrap(),
        parse!(parser, r#"
        match x @ y {
            a @ b {
                body;
            }
        }
        "#).unwrap(),
    );

    assert_eq!(
        parse!(parser, r#"
        const match x @ y => a @ b {
            body;
        }
        "#).unwrap(),
        parse!(parser, r#"
        const match x @ y {
            a @ b {
                body;
            }
        }
        "#).unwrap(),
    );

    assert_eq!(
        parse!(parser, r#"
        const match x @ y => *a @ [b] {
            body;
        }
        "#).unwrap(),
        parse!(parser, r#"
        const match x @ y {
            *a @ [b] {
                body;
            }
        }
        "#).unwrap(),
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        match 1 2 => @ {
            print _0 _1 @;
        }
        print _0 _1 @;
        "#).unwrap()).compile().unwrap(),
        vec![
            "print 1",
            "print 2",
            "print 1",
            "print 2",
            "print 1",
            "print 2",
            "print 1",
            "print 2",
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        const match 1 2 => @ {
            print _0 _1 @;
        }
        print _0 _1 @;
        "#).unwrap()).compile().unwrap(),
        vec![
            "print 1",
            "print 2",
            "print 1",
            "print 2",
            "print 1",
            "print 2",
            "print 1",
            "print 2",
        ],
    );
}

#[test]
fn const_match_test() {
    let parser = TopLevelParser::new();

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        const match {
            {
                print empty;
            }
        }
        "#).unwrap()).compile().unwrap(),
        vec![
            "print empty",
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        match 1 2 { @ {} }
        const match {
            @ {
                print @;
            }
        }
        "#).unwrap()).compile().unwrap(),
        Vec::<&str>::new(),
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        const Val = (res:
            print taked;
        );
        const match Val {
            V {
                print 1 V;
            }
        }
        "#).unwrap()).compile().unwrap(),
        vec![
            "print 1",
            "print taked",
            "print res",
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        const Val = (res:
            print taked;
        );
        const match Val {
            *V {
                print 1 V;
            }
        }
        "#).unwrap()).compile().unwrap(),
        vec![
            "print taked",
            "print 1",
            "print res",
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        const Val = 2;
        const match `Val` {
            *V {
                print 1 V;
            }
        }
        "#).unwrap()).compile().unwrap(),
        vec![
            "print 1",
            "print Val",
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        const Val = 2;
        const match Val {
            *V:[1] {
                print 1 V;
            }
            *V:[(1:print take1;) 2 (3: print err;)] { # lazy
                print x2 V;
            }
        }
        "#).unwrap()).compile().unwrap(),
        vec![
            "print take1",
            "print x2",
            "print 2",
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        const Val = 2;
        const match Val {
            V:[1] {
                print 1 V;
            }
            V:[(1:print take1;) 2 (3: print err;)] { # lazy
                print x2 V;
            }
        }
        "#).unwrap()).compile().unwrap(),
        vec![
            "print take1",
            "print x2",
            "print 2",
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        const Val = 2;
        const match Val {
            [?(0: print _0;)] {
                print unreachable;
            }
        }
        "#).unwrap()).compile().unwrap(),
        vec![
            "print 2",
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        const Val = 2;
        const match Val {
            [?(__: print _0;)] {
                print x;
            }
        }
        "#).unwrap()).compile().unwrap(),
        vec![
            "print 2",
            "print x",
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        const Val = 2;
        const match Val {
            [?(1: print _0;)] {
                print x;
            }
        }
        "#).unwrap()).compile().unwrap(),
        vec![
            "print 2",
            "print x",
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        const Val = 2;
        const match Val {
            [?(false: print _0;)] {
                print x;
            }
        }
        "#).unwrap()).compile().unwrap(),
        vec![
            "print 2",
            "print x",
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        const match 2 {
            [2] {
                print only;
            }
            [2] {
                print unreachable;
            }
        }
        "#).unwrap()).compile().unwrap(),
        vec![
            "print only",
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
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
        "#).unwrap()).compile().unwrap(),
        vec![
            "print default",
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        const match (%2: print x;%)->$ {
            [2] {
                print only;
            }
            [2] {
                print unreachable;
            }
        }
        "#).unwrap()).compile().unwrap(),
        vec![
            "print x",
            "print only",
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        const match (2: print x;) {
            _ {
                print only;
            }
            [2] {
                print unreachable;
            }
        }
        "#).unwrap()).compile().unwrap(),
        vec![
            "print only",
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        const match (2: print x;) {
            *_ {
                print only;
            }
            [2] {
                print unreachable;
            }
        }
        "#).unwrap()).compile().unwrap(),
        vec![
            "print x",
            "print only",
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        const match (2: print x;) {
            [*] {
                print only;
            }
            [2] {
                print unreachable;
            }
        }
        "#).unwrap()).compile().unwrap(),
        vec![
            "print x",
            "print only",
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
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
        "#).unwrap()).compile().unwrap(),
        vec![
            "print x",
            "print x",
            "print only",
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        const match (2: print x;) 3 {
            [*2] [2] {
                print unreachable;
            }
            [*2] [3] {
                print only;
            }
        }
        "#).unwrap()).compile().unwrap(),
        vec![
            "print x",
            "print x",
            "print only",
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        const match (2: print x;) 3 {
            *_ [2] {
                print unreachable;
            }
            *_ [3] {
                print only;
            }
        }
        "#).unwrap()).compile().unwrap(),
        vec![
            "print x",
            "print only",
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
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
        "#).unwrap()).compile().unwrap(),
        vec![
            "print yes",
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        foo (const match (h: print taked;) {
            $_ {
                print body;
            }
        });
        "#).unwrap()).compile().unwrap(),
        vec![
            "print taked",
            "print body",
            "foo h",
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        foo (const match (h: print taked;) {
            $*M {
                print body M;
            }
        });
        "#).unwrap()).compile().unwrap(),
        vec![
            "print taked",
            "print body",
            "print h",
            "foo h",
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        foo (const match (h: print taked;) {
            $*M {
                setres M;
                print body M;
            }
        });
        "#).unwrap()).compile().unwrap(),
        vec![
            "print taked",
            "print body",
            "print h",
            "foo h",
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        foo (const match (h: print taked;) {
            $*M:[h] {
                print body M;
            }
        });
        "#).unwrap()).compile().unwrap(),
        vec![
            "foo __0",
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        foo (const match (h: print taked;) {
            $*M:[*h] {
                setres M;
                print body M;
            }
        });
        "#).unwrap()).compile().unwrap(),
        vec![
            "print taked",
            "print taked",
            "print body",
            "print h",
            "foo h",
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        foo (const match (h: print taked;) {
            $M:[*h] {
                setres M;
                print body M;
            }
        });
        "#).unwrap()).compile().unwrap(),
        vec![
            "print taked",
            "print taked",
            "print taked",
            "print body",
            "print taked",
            "print h",
            "foo h",
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        foo (const match (h: print taked;) {
            $M {
                setres M;
                print body M;
            }
        });
        "#).unwrap()).compile().unwrap(),
        vec![
            "print taked",
            "print taked",
            "print body",
            "print taked",
            "print h",
            "foo h",
        ],
    );
}

#[test]
fn value_bind_of_constkey_test() {
    let parser = TopLevelParser::new();

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        take X = ();
        take X.Y = 2;
        print X.Y;
        "#).unwrap()).compile().unwrap(),
        vec![
            "print 2",
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        take X = ();
        {
            take X.Y = 2; # to global
        }
        print X.Y;
        "#).unwrap()).compile().unwrap(),
        vec![
            "print 2",
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        take X = ();
        {
            const X.Y = (
                :x
                goto :x;
            );
        }
        take X.Y;
        take X.Y;
        "#).unwrap()).compile().unwrap(),
        vec![
            "jump 0 always 0 0",
            "jump 1 always 0 0",
        ],
    );
}

#[test]
fn const_value_expand_binder_test() {
    let parser = TopLevelParser::new();

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        print ..;
        "#).unwrap()).compile().unwrap(),
        vec![
            "print __",
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        const X.Y = (
            print ..;
        );
        take X.Y;
        "#).unwrap()).compile().unwrap(),
        vec![
            "print X",
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        const X.Y = (
            const Foo = (
                print ..;
            );
            take Foo;
        );
        take X.Y;
        "#).unwrap()).compile().unwrap(),
        vec![
            "print X",
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        const X.Y = (
            const Foo.Bar = (
                print ..;
            );
            take Foo.Bar;
        );
        take X.Y;
        "#).unwrap()).compile().unwrap(),
        vec![
            "print Foo",
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        const X.Y = (
            const Foo.Bar = (
                print ..;
            );
            const F = Foo.Bar;
            take F;
        );
        take X.Y;
        "#).unwrap()).compile().unwrap(),
        vec![
            "print Foo",
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
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
        "#).unwrap()).compile().unwrap(),
        vec![
            "set __2 2",
            "set __6 3",
            "op add __2 __2 __6",
            "print __",
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
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
        "#).unwrap()).compile().unwrap(),
        vec![
            "set __2 2",
            "set __6 3",
            "op add __2 __2 __6",
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
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
        "#).unwrap()).compile().unwrap(),
        vec![
            "set __2 2",
            "set __6 3",
            "op add __2 __2 __6",
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        take a.B = 1;
        take a.B = 2;
        print a.B;
        "#).unwrap()).compile().unwrap(),
        vec![
            "print 2",
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        take a.N = 1;
        take b.N = 2;
        # 故意不实现的常量求值
        take X = ($ = a.N + b.N;);
        print X;
        "#).unwrap()).compile().unwrap(),
        vec![
            "op add __2 1 2",
            "print __2",
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        take a.N = 1;
        take b.N = 2;
        take A=a.N B=b.N;
        take X = ($ = A + B;);
        print X;
        "#).unwrap()).compile().unwrap(),
        vec![
            "print 3",
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        const a.X = (
            print ..;
        );
        const b.X = a.X;
        take a.X;
        take b.X;
        "#).unwrap()).compile().unwrap(),
        vec![
            "print a",
            "print a",
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        const A = a;
        const A.X = (
            print ..;
        );
        take A.X;
        "#).unwrap()).compile().unwrap(),
        vec![
            "print a",
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        const a.X = (
            print ..;
        );
        take Handle = Builtin.BindHandle2[`a` `X`];
        take Builtin.Const[`Handle` Handle];
        const b.X = Handle;
        take a.X;
        const a.X = 2;
        take b.X;
        "#).unwrap()).compile().unwrap(),
        vec![
            "print a",
            "print a",
        ],
    );
}

#[test]
fn builtin_func_test() {
    let parser = TopLevelParser::new();

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        print Builtin.Type[x];
        "#).unwrap()).compile().unwrap(),
        vec![
            "print var",
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        print Builtin.Stringify[2];
        print Builtin.Stringify[x];
        print Builtin.Stringify["x"];
        "#).unwrap()).compile().unwrap(),
        vec![
            r#"print "2""#,
            r#"print "x""#,
            r#"print "x""#,
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        print Builtin.Concat["abc" "def"];
        print Builtin.Status;
        print Builtin.Concat["abc" def];
        print Builtin.Status;
        "#).unwrap()).compile().unwrap(),
        vec![
            r#"print "abcdef""#,
            r#"print 0"#,
            r#"print __"#,
            r#"print 2"#,
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        print Builtin.Type[()];
        print Builtin.Type[`m`];
        "#).unwrap()).compile().unwrap(),
        vec![
            "print dexp",
            "print var",
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
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
        "#).unwrap()).compile().unwrap(),
        vec![
            "print dexp",
            "print var",
            "print valuebind",
            "print resulthandle",
            "print binder",
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        const A.B = 2;
        print Builtin.Type[A.B];
        "#).unwrap()).compile().unwrap(),
        vec![
            "print valuebind",
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        const A = x;
        print Builtin.Info[A];
        print Builtin.Info[y];
        print Builtin.Err[A];
        print Builtin.Err[y];
        "#).unwrap()).compile().unwrap(),
        vec![
            "print x",
            "print y",
            "print x",
            "print y",
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        const A = x.y;
        print Builtin.Unbind[A];
        print Builtin.Unbind[x.y];
        "#).unwrap()).compile().unwrap(),
        vec![
            "print y",
            "print y",
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
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
        "#).unwrap()).compile().unwrap(),
        vec![
            "print h",
            "print i",
            "print i",
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        const Value = (x:);
        const Binded = Value.x;
        print Builtin.Type[Binded];
        take Builtin.Binder[Res Binded];
        print Builtin.Type[Res];
        print Res;
        "#).unwrap()).compile().unwrap(),
        vec![
            "print valuebind",
            "print dexp",
            "print x",
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        const Foo = (
            take Builtin.SliceArgs[1 4];
            print @;
        );
        take Foo[0 1 2 3 4 5];
        "#).unwrap()).compile().unwrap(),
        vec![
            "print 1",
            "print 2",
            "print 3",
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        const Foo = (
            print Builtin.ArgsLen[];
        );
        print Builtin.ArgsLen[];
        match a b { @ {} }
        take Foo[0 1 2 3 4 5];
        print Builtin.ArgsLen[];
        "#).unwrap()).compile().unwrap(),
        vec![
            "print 0",
            "print 6",
            "print 2",
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        const Foo = (
            take Handle = Builtin.ArgsHandle[1];
            take Builtin.Const[Value Handle];
            print Handle Value;
        );
        take Foo[0 1 2 3 4 5];
        "#).unwrap()).compile().unwrap(),
        vec![
            "print __1",
            "print 1",
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        const F = ($ = 1+2;);
        print Builtin.EvalNum[F];
        "#).unwrap()).compile().unwrap(),
        vec![
            "print 3",
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        const N = 2 S = "s" D = ();
        print Builtin.IsString[N];
        print Builtin.IsString[2];
        print Builtin.IsString[D];
        print Builtin.IsString[()];
        print Builtin.IsString[S];
        print Builtin.IsString["s"];
        "#).unwrap()).compile().unwrap(),
        vec![
            "print 0",
            "print 0",
            "print 0",
            "print 0",
            "print 1",
            "print 1",
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        const F = (
            {
                take Ref = Builtin.RefArg[0];
                match Ref { [113] { take Builtin.Exit[2]; } }
                take Builtin.Const[`N` Ref];
                print N;
            }
        );
        take F[113];
        "#).unwrap()).compile().unwrap(),
        vec![
            "print 113",
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        take Builtin.SetNoOp["set noop \\'noop\n\\'"];
        noop;
        "#).unwrap()).compile().unwrap(),
        vec![
            r#"set noop "noop\n""#,
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        take Builtin.SetNoOp["set noop \\'noop\n\\'"];
        select x {
            print 1 2 3 4 5;
            print 1 2 3;
            print 1 2 3 4 5;
            print 1 2 3 4 5;
            print 1 2 3 4 5;
        }
        "#).unwrap()).compile().unwrap(),
        vec![
            r#"op mul __1 x 5"#,
            r#"op add @counter @counter __1"#,
            r#"print 1"#,
            r#"print 2"#,
            r#"print 3"#,
            r#"print 4"#,
            r#"print 5"#,
            r#"print 1"#,
            r#"print 2"#,
            r#"print 3"#,
            r#"jump 12 always 0 0"#,
            r#"set noop "noop\n""#,
            r#"print 1"#,
            r#"print 2"#,
            r#"print 3"#,
            r#"print 4"#,
            r#"print 5"#,
            r#"print 1"#,
            r#"print 2"#,
            r#"print 3"#,
            r#"print 4"#,
            r#"print 5"#,
            r#"print 1"#,
            r#"print 2"#,
            r#"print 3"#,
            r#"print 4"#,
            r#"print 5"#,
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        take Builtin.SetNoOp['\"str\"'];
        select x {
            print 1 2 3 4 5;
            print 1 2 3;
            print 1 2 3 4 5;
            print 1 2 3 4 5;
            print 1 2 3 4 5;
        }
        "#).unwrap()).compile().unwrap(),
        vec![
            r#"op mul __1 x 5"#,
            r#"op add @counter @counter __1"#,
            r#"print 1"#,
            r#"print 2"#,
            r#"print 3"#,
            r#"print 4"#,
            r#"print 5"#,
            r#"print 1"#,
            r#"print 2"#,
            r#"print 3"#,
            r#"jump 12 always 0 0"#,
            r#""str""#,
            r#"print 1"#,
            r#"print 2"#,
            r#"print 3"#,
            r#"print 4"#,
            r#"print 5"#,
            r#"print 1"#,
            r#"print 2"#,
            r#"print 3"#,
            r#"print 4"#,
            r#"print 5"#,
            r#"print 1"#,
            r#"print 2"#,
            r#"print 3"#,
            r#"print 4"#,
            r#"print 5"#,
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
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
        "#).unwrap()).compile().unwrap(),
        vec![
            r#"print axb"#,
            r#"print 2"#,
            r#"print 2"#,
            r#"print 2"#,
            r#"print __2"#,
            r#"print 3"#,
            r#"print cxd"#,
            r#"print 3"#,
        ],
    );
}

#[test]
fn closure_value_test() {
    let parser = TopLevelParser::new();

    assert_eq!(
        parse!(parser, r#"
        const X = ([A &B]2);
        "#).unwrap(),
        parse!(parser, r#"
        const X = ([A:A &B:B]2);
        "#).unwrap(),
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        const A = (a: print "makeA";);
        const B = (b: print "makeB";);
        const F = ([A &B](
            print "Do"A B"End";
        ));
        const A="eA" B="eB";
        take F;
        "#).unwrap()).compile().unwrap(),
        vec![
            r#"print "makeA""#,
            r#"print "Do""#,
            r#"print a"#,
            r#"print "makeB""#,
            r#"print b"#,
            r#"print "End""#,
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        const A = 2;
        const B = `A`;
        const F = ([B](
            print B;
        ));
        take F;
        "#).unwrap()).compile().unwrap(),
        vec![
            r#"print A"#,
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        const A = 2;
        const B = `A`;
        const V = ([]`B`);
        const B = "e";
        print V;
        "#).unwrap()).compile().unwrap(),
        vec![
            r#"print B"#,
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
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
        "#).unwrap()).compile().unwrap(),
        vec![
            r#"print 2"#,
            r#"jump 0 lessThan a b"#,
            r#"print 2"#,
            r#"jump 2 lessThan a b"#,
            r#"jump 4 always 0 0"#,
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
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
        "#).unwrap()).compile().unwrap(),
        vec![
            r#"print 2"#,
            r#"jump 0 lessThan a b"#,
            r#"print 2"#,
            r#"jump 2 lessThan a b"#,
            r#"jump 4 always 0 0"#,
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        print ([X:1](
            print "foo";
            setres X;
        ));
        "#).unwrap()).compile().unwrap(),
        vec![
            r#"print "foo""#,
            r#"print 1"#,
        ],
    );
}

#[test]
fn value_bind_ref_test() {
    let parser = TopLevelParser::new();

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
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
        "#).unwrap()).compile().unwrap(),
        vec![
            r#"print 1"#,
            r#"print 2"#,
            r#"print "take""#,
            r#"print "finish""#,
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
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
        "#).unwrap()).compile().unwrap(),
        vec![
            r#"print 1"#,
            r#"print 2"#,
            r#"print "take""#,
            r#"print "finish""#,
            r#"print "take""#,
            r#"print "finish""#,
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
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
        "#).unwrap()).compile().unwrap(),
        vec![
            r#"print 1"#,
            r#"print 2"#,
            r#"print bind"#,
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
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
        "#).unwrap()).compile().unwrap(),
        vec![
            r#"print 1"#,
            r#"print 2"#,
            r#"print bind"#,
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        const Res = (%
            print "maked";
            const $.M = 2;
        %)->$;
        print 1 Res.M;
        "#).unwrap()).compile().unwrap(),
        vec![
            r#"print "maked""#,
            r#"print 1"#,
            r#"print 2"#,
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        const F = (print 2;);
        print 1;
        const Attr = F->X;
        print 3 Attr;
        "#).unwrap()).compile().unwrap(),
        vec![
            r#"print 1"#,
            r#"print 2"#,
            r#"print 3"#,
            r#"print __1"#,
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        const bind.next.X = 2;
        const F = bind->next->X;
        print bind.next F->.. F->..->..;
        "#).unwrap()).compile().unwrap(),
        vec![
            r#"print __0"#,
            r#"print __0"#,
            r#"print __"#,
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        const bind.X = (x: print "makeX";);
        const bind.Y = (y: print "makeY";);
        print 1;
        const X = bind->X;
        print 2 X;
        const Y = X->..->Y;
        print 3 Y;
        print X->.. Y->..;
        "#).unwrap()).compile().unwrap(),
        vec![
            r#"print 1"#,
            r#"print 2"#,
            r#"print "makeX""#,
            r#"print x"#,
            r#"print 3"#,
            r#"print "makeY""#,
            r#"print y"#,
            r#"print bind"#,
            r#"print bind"#,
        ],
    );
}

#[test]
fn gswitch_test() {
    let parser = TopLevelParser::new();

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        gswitch x {
        case: print 1;
        case: print 2;
        }
        "#).unwrap()).compile().unwrap(),
        vec![
            r#"op add @counter @counter x"#,
            r#"jump 3 always 0 0"#,
            r#"jump 4 always 0 0"#,
            r#"print 1"#,
            r#"print 2"#,
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        gswitch x {
            break;
        case: print 1;
        case: print 2;
        }
        "#).unwrap()).compile().unwrap(),
        vec![
            r#"op add @counter @counter x"#,
            r#"jump 3 always 0 0"#,
            r#"jump 5 always 0 0"#,
            r#"print 1"#,
            r#"jump 0 always 0 0"#,
            r#"print 2"#,
            r#"jump 0 always 0 0"#,
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        gswitch x {
            break;
        case 1: print 1;
        case: print 2;
        }
        "#).unwrap()).compile().unwrap(),
        vec![
            r#"op add @counter @counter x"#,
            r#"jump 0 always 0 0"#,
            r#"jump 4 always 0 0"#,
            r#"jump 6 always 0 0"#,
            r#"print 1"#,
            r#"jump 0 always 0 0"#,
            r#"print 2"#,
            r#"jump 0 always 0 0"#,
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        gswitch x {
            break;
        case 1: print 1;
        case !: print mis;
        case: print 2;
        }
        "#).unwrap()).compile().unwrap(),
        vec![
            r#"op add @counter @counter x"#,
            r#"jump 6 always 0 0"#,
            r#"jump 4 always 0 0"#,
            r#"jump 8 always 0 0"#,
            r#"print 1"#,
            r#"jump 0 always 0 0"#,
            r#"print mis"#,
            r#"jump 0 always 0 0"#,
            r#"print 2"#,
            r#"jump 0 always 0 0"#,
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        gswitch x {
            break;
        case 1: print 1;
        case ! MAX: print mis MAX;
        case: print 2;
        }
        "#).unwrap()).compile().unwrap(),
        vec![
            r#"op add @counter @counter x"#,
            r#"jump 6 always 0 0"#,
            r#"jump 4 always 0 0"#,
            r#"jump 9 always 0 0"#,
            r#"print 1"#,
            r#"jump 0 always 0 0"#,
            r#"print mis"#,
            r#"print x"#,
            r#"jump 0 always 0 0"#,
            r#"print 2"#,
            r#"jump 0 always 0 0"#,
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        gswitch x {
            break;
        case 1: print 1;
        case ! MAX: print mis MAX;
        case: print 2;
        }
        end;
        "#).unwrap()).compile().unwrap(),
        vec![
            r#"op add @counter @counter x"#,
            r#"jump 6 always 0 0"#,
            r#"jump 4 always 0 0"#,
            r#"jump 9 always 0 0"#,
            r#"print 1"#,
            r#"jump 11 always 0 0"#,
            r#"print mis"#,
            r#"print x"#,
            r#"jump 11 always 0 0"#,
            r#"print 2"#,
            r#"jump 11 always 0 0"#,
            r#"end"#,
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        gswitch x {
            break;
        case 1: print 1;
        case <: print less;
        case: print 2;
        }
        end;
        "#).unwrap()).compile().unwrap(),
        vec![
            r#"jump 7 lessThan x 0"#,
            r#"op add @counter @counter x"#,
            r#"jump 11 always 0 0"#,
            r#"jump 5 always 0 0"#,
            r#"jump 9 always 0 0"#,
            r#"print 1"#,
            r#"jump 11 always 0 0"#,
            r#"print less"#,
            r#"jump 11 always 0 0"#,
            r#"print 2"#,
            r#"jump 11 always 0 0"#,
            r#"end"#,
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        gswitch x {
            break;
        case 1: print 1;
        case >: print 'greaterThan';
        case: print 2;
        }
        end;
        "#).unwrap()).compile().unwrap(),
        vec![
            r#"jump 7 greaterThan x 2"#,
            r#"op add @counter @counter x"#,
            r#"jump 11 always 0 0"#,
            r#"jump 5 always 0 0"#,
            r#"jump 9 always 0 0"#,
            r#"print 1"#,
            r#"jump 11 always 0 0"#,
            r#"print greaterThan"#,
            r#"jump 11 always 0 0"#,
            r#"print 2"#,
            r#"jump 11 always 0 0"#,
            r#"end"#,
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        gswitch x {
            break;
        case 1: print 1;
        case !>: print hit;
        case: print 2;
        }
        end;
        "#).unwrap()).compile().unwrap(),
        vec![
            r#"jump 7 greaterThan x 2"#,
            r#"op add @counter @counter x"#,
            r#"jump 7 always 0 0"#,
            r#"jump 5 always 0 0"#,
            r#"jump 9 always 0 0"#,
            r#"print 1"#,
            r#"jump 11 always 0 0"#,
            r#"print hit"#,
            r#"jump 11 always 0 0"#,
            r#"print 2"#,
            r#"jump 11 always 0 0"#,
            r#"end"#,
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        gswitch x {
            break;
        case 1: print 1;
        case <!>: print hit;
        case: print 2;
        }
        end;
        "#).unwrap()).compile().unwrap(),
        vec![
            r#"jump 8 lessThan x 0"#,
            r#"jump 8 greaterThan x 2"#,
            r#"op add @counter @counter x"#,
            r#"jump 8 always 0 0"#,
            r#"jump 6 always 0 0"#,
            r#"jump 10 always 0 0"#,
            r#"print 1"#,
            r#"jump 12 always 0 0"#,
            r#"print hit"#,
            r#"jump 12 always 0 0"#,
            r#"print 2"#,
            r#"jump 12 always 0 0"#,
            r#"end"#,
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        const I = 1;
        gswitch x {
            break;
        case I: print `I`;
        }
        end;
        "#).unwrap()).compile().unwrap(),
        vec![
            r#"op add @counter @counter x"#,
            r#"jump 5 always 0 0"#,
            r#"jump 3 always 0 0"#,
            r#"print I"#,
            r#"jump 5 always 0 0"#,
            r#"end"#,
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        const I = 1;
        gswitch x {
            break;
        case I if y: print `I`;
        }
        end;
        "#).unwrap()).compile().unwrap(),
        vec![
            r#"op add @counter @counter x"#,
            r#"jump 8 always 0 0"#,
            r#"jump 4 always 0 0"#,
            r#"jump 8 always 0 0"#,
            r#"jump 6 notEqual y false"#,
            r#"jump 8 always 0 0"#,
            r#"print I"#,
            r#"jump 8 always 0 0"#,
            r#"end"#,
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        match 1 2 { @ {} }
        gswitch x {
            break;
        case @: print x;
        }
        end;
        "#).unwrap()).compile().unwrap(),
        vec![
            r#"op add @counter @counter x"#,
            r#"jump 6 always 0 0"#,
            r#"jump 4 always 0 0"#,
            r#"jump 4 always 0 0"#,
            r#"print x"#,
            r#"jump 6 always 0 0"#,
            r#"end"#,
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        const match (1:) 2 { @ {} }
        gswitch x {
            break;
        case @: print x;
        }
        end;
        "#).unwrap()).compile().unwrap(),
        vec![
            r#"op add @counter @counter x"#,
            r#"jump 6 always 0 0"#,
            r#"jump 4 always 0 0"#,
            r#"jump 4 always 0 0"#,
            r#"print x"#,
            r#"jump 6 always 0 0"#,
            r#"end"#,
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        const match (1:) (2: print start;) { @ {} }
        gswitch x {
            break;
        case @: print x;
        }
        end;
        "#).unwrap()).compile().unwrap(),
        vec![
            r#"print start"#,
            r#"op add @counter @counter x"#,
            r#"jump 7 always 0 0"#,
            r#"jump 5 always 0 0"#,
            r#"jump 5 always 0 0"#,
            r#"print x"#,
            r#"jump 7 always 0 0"#,
            r#"end"#,
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        gswitch x {
        case: print 1;
        case: print 2 3 4;
        }
        "#).unwrap()).compile().unwrap(),
        vec![
            r#"op add @counter @counter x"#,
            r#"jump 3 always 0 0"#,
            r#"jump 4 always 0 0"#,
            r#"print 1"#,
            r#"print 2"#,
            r#"print 3"#,
            r#"print 4"#,
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        gswitch x {
        case 0: print 0;
        case 1 2 3: print 1 2 3;
        }
        "#).unwrap()).compile().unwrap(),
        vec![
            r#"op add @counter @counter x"#,
            r#"jump 5 always 0 0"#,
            r#"jump 6 always 0 0"#,
            r#"jump 6 always 0 0"#,
            r#"jump 6 always 0 0"#,
            r#"print 0"#,
            r#"print 1"#,
            r#"print 2"#,
            r#"print 3"#,
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        gswitch x {
            break;
        case 1 2: print x;
        case*3: print end;
        }
        end;
        "#).unwrap()).compile().unwrap(),
        vec![
            r#"op add @counter @counter x"#,
            r#"jump 8 always 0 0"#,
            r#"jump 5 always 0 0"#,
            r#"jump 5 always 0 0"#,
            r#"jump 7 always 0 0"#,
            r#"print x"#,
            r#"jump 8 always 0 0"#,
            r#"print end"#,
            r#"end"#,
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        gswitch x {
            break;
        case*1 2: print x;
        case 3: print end;
        }
        end;
        "#).unwrap()).compile().unwrap(),
        vec![
            r#"op add @counter @counter x"#,
            r#"jump 8 always 0 0"#,
            r#"jump 5 always 0 0"#,
            r#"jump 5 always 0 0"#,
            r#"jump 6 always 0 0"#,
            r#"print x"#,
            r#"print end"#,
            r#"jump 8 always 0 0"#,
            r#"end"#,
        ],
    );
}

#[test]
fn closure_catch_label_test() {
    let parser = TopLevelParser::new();

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
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
        "#).unwrap()).compile().unwrap(),
        vec![
            r#"print expected"#,
            r#"print unexpected"#,
            r#"jump 1 always 0 0"#,
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
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
        "#).unwrap()).compile().unwrap(),
        vec![
            r#"print expected"#,
            r#"print unexpected"#,
            r#"jump 0 always 0 0"#,
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
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
        "#).unwrap()).compile().unwrap(),
        vec![
            r#"print expected"#,
            r#"print unexpected"#,
            r#"jump 0 always 0 0"#,
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
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
        "#).unwrap()).compile().unwrap(),
        vec![
            r#"print expected"#,
            r#"print unexpected"#,
            r#"jump 0 always 0 0"#,
            r#"print expected"#,
            r#"print unexpected"#,
            r#"jump 3 always 0 0"#,
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
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
        "#).unwrap()).compile().unwrap(),
        vec![
            r#"print expected"#,
            r#"print unexpected"#,
            r#"jump 0 always 0 0"#,
            r#"jump 0 always 0 0"#,
            r#"print expected"#,
            r#"print unexpected"#,
            r#"jump 4 always 0 0"#,
            r#"jump 4 always 0 0"#,
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
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
        "#).unwrap()).compile().unwrap(),
        vec![
            r#"print unexpected"#,
            r#"jump 3 always 0 0"#,
            r#"jump 3 always 0 0"#,
            r#"print expected"#,
            r#"print unexpected"#,
            r#"jump 7 always 0 0"#,
            r#"jump 7 always 0 0"#,
            r#"print expected"#,
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
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
        "#).unwrap()).compile().unwrap(),
        vec![
            r#"print unexpected"#,
            r#"jump 3 always 0 0"#,
            r#"jump 3 always 0 0"#,
            r#"print expected"#,
        ],
    );
}

#[test]
fn non_take_result_handle_dexp_test() {
    let parser = TopLevelParser::new();

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        const H = 2;
        print (H: print pre;);
        "#).unwrap()).compile().unwrap(),
        vec![
            r#"print pre"#,
            r#"print 2"#,
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        const H = 2;
        print (`H`: print pre;);
        "#).unwrap()).compile().unwrap(),
        vec![
            r#"print pre"#,
            r#"print H"#,
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        const H = (2:);
        print (`H`: print pre;);
        "#).unwrap()).compile().unwrap(),
        vec![
            r#"print pre"#,
            r#"print H"#,
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        print (?`n`: 2);
        "#).unwrap()).compile().unwrap(),
        vec![
            r#"set n 2"#,
            r#"print n"#,
        ],
    );
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

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        match a b c => @ {}
        const Clos = ([@](
            print @;
        ));
        take Clos[];
        "#).unwrap()).compile().unwrap(),
        vec![
            r#"print a"#,
            r#"print b"#,
            r#"print c"#,
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        match a b c => @ {}
        const Clos = ([@](
            print @;
        ));
        match d e f => @ {}
        take Clos[];
        "#).unwrap()).compile().unwrap(),
        vec![
            r#"print a"#,
            r#"print b"#,
            r#"print c"#,
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        match a b c => @ {}
        const Clos = ([@](
            print @;
        ));
        match d e f => @ {}
        take Clos[];
        print @;
        "#).unwrap()).compile().unwrap(),
        vec![
            r#"print a"#,
            r#"print b"#,
            r#"print c"#,
            r#"print d"#,
            r#"print e"#,
            r#"print f"#,
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        match a b c => @ {}
        const Clos = ([@](
            print @;
        ));
        const a = 2;
        take Clos[];
        "#).unwrap()).compile().unwrap(),
        vec![
            r#"print a"#,
            r#"print b"#,
            r#"print c"#,
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        match a b c => @ {}
        const Clos = ([@](
            print @;
        ));
        take Clos[1 2];
        "#).unwrap()).compile().unwrap(),
        vec![
            r#"print a"#,
            r#"print b"#,
            r#"print c"#,
        ],
    );

    assert_eq!( // moved value owned test
        CompileMeta::new().compile(parse!(parser, r#"
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
        "#).unwrap()).compile().unwrap(),
        vec![
            r#"print a"#,
            r#"print b"#,
            r#"print c"#,
            r#"print a"#,
            r#"print b"#,
            r#"print c"#,
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
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
        "#).unwrap()).compile().unwrap(),
        vec![
            r#"print a"#,
            r#"print b"#,
            r#"print c"#,
            r#"print a"#,
            r#"print b"#,
            r#"print c"#,
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
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
        "#).unwrap()).compile().unwrap(),
        vec![
            r#"print split"#,
            r#"print run"#,
            r#"print x"#,
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
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
        "#).unwrap()).compile().unwrap(),
        vec![
            r#"print pre"#,
            r#"print split"#,
            r#"print run"#,
            r#"print x"#,
        ],
    );
}

#[test]
fn param_deref_test() {
    let parser = TopLevelParser::new();

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        const F = (
            const N = 2;
            print @;
        );
        const N = 1;
        take F[(setres N;)];
        take F[*(setres N;)];
        "#).unwrap()).compile().unwrap(),
        vec![
            r#"print 2"#,
            r#"print 1"#,
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        const F = (
            const N = 2;
            print @;
        );
        const match (setres N;) => @ {}
        const N = 1;
        take F[(setres N;) @];
        take F[*(setres N;) @];
        "#).unwrap()).compile().unwrap(),
        vec![
            r#"print 2"#,
            r#"print 2"#,
            r#"print 1"#,
            r#"print 2"#,
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        const F = (
            const N = 2;
            print @;
        );
        const match (setres N;) => @ {}
        const N = 1;
        take F[(setres N;) @];
        match @ => @ {}
        take F[*(setres N;) @];
        "#).unwrap()).compile().unwrap(),
        vec![
            r#"print 2"#,
            r#"print 2"#,
            r#"print 1"#,
            r#"print 1"#,
        ],
    );
}

#[test]
fn param_inf_len_test() {
    let parser = TopLevelParser::new();

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        const F = (
            print @;
        );
        F! 0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15
            16 17 18 19 20 21 22 23 24 25 26 27 28 29 30 31;
        "#).unwrap()).compile().unwrap(),
        vec![
            r#"print 0"#,
            r#"print 1"#,
            r#"print 2"#,
            r#"print 3"#,
            r#"print 4"#,
            r#"print 5"#,
            r#"print 6"#,
            r#"print 7"#,
            r#"print 8"#,
            r#"print 9"#,
            r#"print 10"#,
            r#"print 11"#,
            r#"print 12"#,
            r#"print 13"#,
            r#"print 14"#,
            r#"print 15"#,
            r#"print 16"#,
            r#"print 17"#,
            r#"print 18"#,
            r#"print 19"#,
            r#"print 20"#,
            r#"print 21"#,
            r#"print 22"#,
            r#"print 23"#,
            r#"print 24"#,
            r#"print 25"#,
            r#"print 26"#,
            r#"print 27"#,
            r#"print 28"#,
            r#"print 29"#,
            r#"print 30"#,
            r#"print 31"#,
        ],
    );

    assert_eq!(
        CompileMeta::new().compile(parse!(parser, r#"
        (%print @;%)! 0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15
            16 17 18 19 20 21 22 23 24 25 26 27 28 29 30 31;
        "#).unwrap()).compile().unwrap(),
        vec![
            r#"print 0"#,
            r#"print 1"#,
            r#"print 2"#,
            r#"print 3"#,
            r#"print 4"#,
            r#"print 5"#,
            r#"print 6"#,
            r#"print 7"#,
            r#"print 8"#,
            r#"print 9"#,
            r#"print 10"#,
            r#"print 11"#,
            r#"print 12"#,
            r#"print 13"#,
            r#"print 14"#,
            r#"print 15"#,
            r#"print 16"#,
            r#"print 17"#,
            r#"print 18"#,
            r#"print 19"#,
            r#"print 20"#,
            r#"print 21"#,
            r#"print 22"#,
            r#"print 23"#,
            r#"print 24"#,
            r#"print 25"#,
            r#"print 26"#,
            r#"print 27"#,
            r#"print 28"#,
            r#"print 29"#,
            r#"print 30"#,
            r#"print 31"#,
        ],
    );
}
