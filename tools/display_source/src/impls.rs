use syntax::*;
use crate::{DisplaySource, DisplaySourceMeta};

impl DisplaySource for Value {
    fn display_source(&self, meta: &mut DisplaySourceMeta) {
        let replace_ident = Self::replace_ident;
        match self {
            Self::Var(s) => meta.push(&replace_ident(s)),
            Self::ReprVar(s) => meta.push(&format!("`{}`", replace_ident(s))),
            Self::ResultHandle => meta.push("$"),
            Self::DExp(dexp) => dexp.display_source(meta),
            Self::ValueBind(value_attr) => value_attr.display_source(meta),
            Self::Cmper(cmp) => {
                meta.push("goto");
                meta.push("(");
                cmp.display_source(meta);
                meta.push(")");
            }
        }
    }
}
impl DisplaySource for DExp {
    fn display_source(&self, meta: &mut DisplaySourceMeta) {
        meta.push("(");
        let has_named_res = !self.result().is_empty();
        if has_named_res {
            meta.push(&Value::replace_ident(self.result()));
            meta.push(":");
        }
        match self.lines().len() {
            0 => (),
            1 => {
                if has_named_res {
                    meta.add_space();
                }
                self.lines()[0].display_source(meta);
            },
            _ => {
                meta.add_lf();
                meta.do_block(|meta| {
                    self.lines().display_source(meta);
                });
            }
        }
        meta.push(")");
    }
}
impl DisplaySource for ValueBind {
    fn display_source(&self, meta: &mut DisplaySourceMeta) {
        self.0.display_source(meta);
        meta.push(".");
        meta.push(&Value::replace_ident(&self.1));
    }
}
impl DisplaySource for JumpCmp {
    fn display_source(&self, meta: &mut DisplaySourceMeta) {
        if let Self::Always | Self::NotAlways = self {
            meta.push(self.get_symbol_cmp_str())
        } else {
            let sym = self.get_symbol_cmp_str();
            let (a, b) = self.get_values_ref().unwrap();
            a.display_source(meta);
            meta.add_space();
            meta.push(sym);
            meta.add_space();
            b.display_source(meta);
        }
    }
}
impl DisplaySource for CmpTree {
    fn display_source(&self, meta: &mut DisplaySourceMeta) {
        match self {
            Self::Atom(cmp) => cmp.display_source(meta),
            Self::Deps(deps, cmp) => {
                meta.push("(");
                meta.push("{");
                if let [line] = &deps[..] {
                    line.display_source(meta)
                } else {
                    meta.do_block(|meta| {
                        meta.add_lf();
                        deps.display_source(meta)
                    })
                }
                meta.push("}");
                meta.add_space();
                meta.push("=>");
                meta.add_space();
                cmp.display_source(meta);
                meta.push(")");
            },
            Self::Or(a, b) => {
                meta.push("(");
                a.display_source(meta);
                meta.add_space();
                meta.push("||");
                meta.add_space();
                b.display_source(meta);
                meta.push(")");
            },
            Self::And(a, b) => {
                meta.push("(");
                a.display_source(meta);
                meta.add_space();
                meta.push("&&");
                meta.add_space();
                b.display_source(meta);
                meta.push(")");
            }
        }
    }
}
impl DisplaySource for Op {
    fn display_source(&self, meta: &mut DisplaySourceMeta) {
        macro_rules! build_match {
            {
                op1: [ $( $oper1:ident ),* $(,)?  ]
                op2: [ $( $oper2:ident ),* $(,)?  ]
                op2l: [ $( $oper2l:ident ),* $(,)?  ]
            } => {
                match self {
                    $(
                        Self::$oper1(_, a) => {
                            meta.push(self.oper_symbol_str());
                            meta.add_space();

                            a.display_source(meta);
                        },
                    )*
                    $(
                        Self::$oper2(_, a, b) => {
                            a.display_source(meta);
                            meta.add_space();

                            meta.push(self.oper_symbol_str());
                            meta.add_space();

                            b.display_source(meta);
                        },
                    )*
                    $(
                        Self::$oper2l(_, a, b) => {
                            meta.push(self.oper_symbol_str());
                            meta.add_space();

                            a.display_source(meta);
                            meta.add_space();

                            b.display_source(meta);
                        },
                    )*
                }
            };
        }
        meta.push("op");
        meta.add_space();
        self.get_info().result.display_source(meta);
        meta.add_space();

        build_match! {
            op1: [
                Not, Abs, Log, Log10, Floor, Ceil, Sqrt,
                Rand, Sin, Cos, Tan, Asin, Acos, Atan,
            ]
            op2: [
                Add, Sub, Mul, Div, Idiv,
                Mod, Pow, Equal, NotEqual, Land,
                LessThan, LessThanEq, GreaterThan, GreaterThanEq, StrictEqual,
                Shl, Shr, Or, And, Xor,
            ]
            op2l: [
                Max, Min, Angle, Len, Noise,
            ]
        };
        meta.push(";");
    }
}
impl DisplaySource for Goto {
    fn display_source(&self, meta: &mut DisplaySourceMeta) {
        let Self(lab, cmp) = self;

        meta.push("goto");
        meta.add_space();
        meta.push(":");
        meta.push(&Value::replace_ident(lab));
        meta.add_space();
        cmp.display_source(meta);
        meta.push(";");
    }
}
impl DisplaySource for Expand {
    fn display_source(&self, meta: &mut DisplaySourceMeta) {
        self.0
            .iter()
            .for_each(|line| {
                line.display_source(meta);
                meta.add_lf();
            })
    }
}
impl DisplaySource for InlineBlock {
    fn display_source(&self, meta: &mut DisplaySourceMeta) {
        self.0
            .iter()
            .for_each(|line| {
                line.display_source(meta);
                meta.add_lf();
            })
    }
}
impl DisplaySource for Select {
    fn display_source(&self, meta: &mut DisplaySourceMeta) {
        meta.push("select");
        meta.add_space();

        self.0.display_source(meta);
        meta.add_space();

        meta.push("{");
        meta.add_lf();
        meta.do_block(|meta| {
            self.1.display_source(meta);
        });
        meta.push("}");
    }
}
impl DisplaySource for Const {
    fn display_source(&self, meta: &mut DisplaySourceMeta) {
        meta.push("const");
        meta.add_space();

        meta.push(&Value::replace_ident(&self.0));
        meta.add_space();

        meta.push("=");
        meta.add_space();

        self.1.display_source(meta);

        meta.push(";");
        meta.add_space();

        let labs = self.2
            .iter()
            .map(|s| Value::replace_ident(&**s))
            .fold(
                Vec::with_capacity(self.2.len()),
                |mut labs, s| {
                    labs.push(s);
                    labs
                }
            );
        meta.push(&format!("# labels: [{}]", labs.join(", ")));
    }
}
impl DisplaySource for Take {
    fn display_source(&self, meta: &mut DisplaySourceMeta) {
        meta.push("take");
        meta.add_space();

        meta.push(&Value::replace_ident(&self.0));
        meta.add_space();

        meta.push("=");
        meta.add_space();

        self.1.display_source(meta);
        meta.push(";");
    }
}
impl DisplaySource for LogicLine {
    fn display_source(&self, meta: &mut DisplaySourceMeta) {
        match self {
            Self::Expand(expand) => {
                meta.push("{");
                if !expand.is_empty() {
                    meta.add_lf();
                    meta.do_block(|meta| {
                        expand.display_source(meta);
                    });
                }
                meta.push("}");
            },
            Self::InlineBlock(block) => {
                meta.push("inline");
                meta.add_space();
                meta.push("{");
                if !block.is_empty() {
                    meta.add_lf();
                    meta.do_block(|meta| {
                        block.display_source(meta);
                    });
                }
                meta.push("}");
            },
            Self::Ignore => meta.push("{} # ignore line"),
            Self::NoOp => meta.push("noop;"),
            Self::Label(lab) => {
                meta.push(":");
                meta.push(&Value::replace_ident(lab))
            },
            Self::Goto(goto) => goto.display_source(meta),
            Self::Op(op) => op.display_source(meta),
            Self::Select(select) => select.display_source(meta),
            Self::Take(take) => take.display_source(meta),
            Self::Const(r#const) => r#const.display_source(meta),
            Self::ConstLeak(var) => {
                meta.push("# constleak");
                meta.add_space();
                meta.push(&Value::replace_ident(var));
                meta.push(";");
            },
            Self::SetResultHandle(val) => {
                meta.push("setres");
                meta.add_space();
                val.display_source(meta);
                meta.push(";");
            },
            Self::Other(args) => {
                assert_ne!(args.len(), 0);
                let mut iter = args.iter();
                iter.next().unwrap().display_source(meta);
                iter.for_each(|arg| {
                    meta.add_space();
                    arg.display_source(meta);
                });
                meta.push(";");
            },
        }
    }
}

#[cfg(test)]
#[test]
fn display_source_test() {
    use parser::*;

    macro_rules! parse {
        ( $parser:expr, $src:expr ) => {
            ($parser).parse(&mut Meta::new(), $src)
        };
    }

    let line_parser = LogicLineParser::new();
    let jumpcmp_parser = JumpCmpParser::new();

    let mut meta = Default::default();
    assert_eq!(
        parse!(
            line_parser,
            r#"'abc' 'abc"def' "str" "str'str" 'no_str' '2';"#
        )
            .unwrap()
            .display_source_and_get(&mut meta),
        r#"abc 'abc"def' "str" "str'str" no_str 2;"#
    );
    assert_eq!(
        JumpCmp::GreaterThan("a".into(), "1".into())
            .display_source_and_get(&mut meta),
        "a > 1"
    );
    assert_eq!(
        parse!(jumpcmp_parser, "a < b && c < d && e < f")
            .unwrap()
            .display_source_and_get(&mut meta),
        "((a < b && c < d) && e < f)"
    );
    assert_eq!(
        parse!(line_parser, "{foo;}")
            .unwrap()
            .display_source_and_get(&mut meta),
        "{\n    foo;\n}"
    );
    assert_eq!(
        parse!(line_parser, "print ($ = x;);")
            .unwrap()
            .display_source_and_get(&mut meta),
        "`'print'` (`'set'` $ x;);"
    );
    assert_eq!(
        parse!(line_parser, "print (res: $ = x;);")
            .unwrap()
            .display_source_and_get(&mut meta),
        "`'print'` (res: `'set'` $ x;);"
    );
    assert_eq!(
        parse!(line_parser, "print (noop;$ = x;);")
            .unwrap()
            .display_source_and_get(&mut meta),
        "`'print'` (\n    noop;\n    `'set'` $ x;\n);"
    );
    assert_eq!(
        parse!(line_parser, "print (res: noop;$ = x;);")
            .unwrap()
            .display_source_and_get(&mut meta),
        "`'print'` (res:\n    noop;\n    `'set'` $ x;\n);"
    );
    assert_eq!(
        parse!(line_parser, "print a.b.c;")
            .unwrap()
            .display_source_and_get(&mut meta),
        "`'print'` a.b.c;"
    );
    assert_eq!(
        parse!(line_parser, "op add a b c;")
            .unwrap()
            .display_source_and_get(&mut meta),
        "op a b + c;"
    );
    assert_eq!(
        parse!(line_parser, "op x noise a b;")
            .unwrap()
            .display_source_and_get(&mut meta),
        "op x noise a b;"
    );
    assert_eq!(
        parse!(line_parser, "foo 1 0b1111_0000 0x8f_ee abc 你我他 _x @a-b;")
            .unwrap()
            .display_source_and_get(&mut meta),
        "foo 1 0b11110000 0x8fee abc 你我他 _x @a-b;"
    );
    assert_eq!(
        parse!(line_parser, "'take' '1._2' '0b_11_00' '-0b1111_0000' '-0x8f' 'a-bc';")
            .unwrap()
            .display_source_and_get(&mut meta),
        "'take' '1._2' '0b_11_00' '-0b1111_0000' '-0x8f' 'a-bc';"
    );
    assert_eq!(
        parse!(line_parser, "'take' 'set' 'print' 'const' 'take' 'op';")
            .unwrap()
            .display_source_and_get(&mut meta),
        "'take' 'set' 'print' 'const' 'take' 'op';"
    );
    assert_eq!(
        parse!(jumpcmp_parser, "({take X = N;} => X > 10 && X < 50)")
            .unwrap()
            .display_source_and_get(&mut meta),
        "({take X = N;} => (X > 10 && X < 50))"
    );
    assert_eq!(
        parse!(jumpcmp_parser, "({take X = A; take Y = B;} => X > 10 && Y > 20 && X < Y)")
            .unwrap()
            .display_source_and_get(&mut meta),
        "({\n    take X = A;\n    take Y = B;\n} => ((X > 10 && Y > 20) && X < Y))"
    );
    assert_eq!(
        parse!(line_parser, r#"set a "\n\\\[hi]\\n";"#)
            .unwrap()
            .display_source_and_get(&mut meta),
        r#"`'set'` a "\n\\[[hi]\\n";"#
    );
}
