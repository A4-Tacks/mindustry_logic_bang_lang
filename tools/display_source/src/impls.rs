use syntax::*;
use crate::{DisplaySource, DisplaySourceMeta};

fn inline_labs<T>(labs: &[T], meta: &mut DisplaySourceMeta)
where T: DisplaySource
{
    if !labs.is_empty() {
        meta.push("#*");
        meta.push("labels: [");
        meta.display_source_iter_by_splitter(
            |meta| meta.push(", "),
            labs,
        );
        meta.push("]");
        meta.push("*#");
    }
}

impl DisplaySource for str {
    fn display_source(&self, meta: &mut DisplaySourceMeta) {
        meta.push(&Value::replace_ident(self))
    }
}
impl DisplaySource for Var {
    fn display_source(&self, meta: &mut DisplaySourceMeta) {
        self.as_str().display_source(meta)
    }
}
impl DisplaySource for ClosuredValueMethod {
    fn display_source(&self, meta: &mut DisplaySourceMeta) {
        match self {
            Self::Take(Take(var, val)) => {
                var.display_source(meta);
                meta.push(":");
                val.display_source(meta);
            },
            Self::Const(Const(var, val, labs)) => {
                meta.push("&");
                var.display_source(meta);
                meta.push(":");
                val.display_source(meta);

                inline_labs(labs, meta)
            },
        }
    }
}
impl DisplaySource for ClosuredValue {
    fn display_source(&self, meta: &mut DisplaySourceMeta) {
        let Self::Uninit {
            catch_values,
            value,
            labels,
        } = self else {
            panic!("failed builded value: {:?}", self)
        };
        meta.push("([");
        meta.display_source_iter_by_splitter(|meta| {
            meta.add_space();
        }, catch_values);
        meta.push("]");
        value.display_source(meta);
        inline_labs(labels, meta);
        meta.push(")");
    }
}
impl DisplaySource for Value {
    fn display_source(&self, meta: &mut DisplaySourceMeta) {
        match self {
            Self::Var(s) => s.display_source(meta),
            Self::ReprVar(s) => {
                meta.push("`");
                s.display_source(meta);
                meta.push("`");
            },
            Self::ResultHandle => meta.push("$"),
            Self::Binder => meta.push(".."),
            Self::DExp(dexp) => dexp.display_source(meta),
            Self::ValueBind(value_attr) => value_attr.display_source(meta),
            Self::ValueBindRef(bindref) => bindref.display_source(meta),
            Self::Cmper(cmp) => {
                meta.push("goto");
                meta.push("(");
                cmp.display_source(meta);
                meta.push(")");
            },
            Self::BuiltinFunc(builtin_func) => {
                meta.push("(#*");
                meta.push("BuiltinFunc: ");
                meta.push(builtin_func.name());
                meta.push("*#)");
            },
            Self::ClosuredValue(clos) => clos.display_source(meta),
        }
    }
}
impl DisplaySource for DExp {
    fn display_source(&self, meta: &mut DisplaySourceMeta) {
        meta.push("(");
        let has_named_res = !self.result().is_empty();
        if has_named_res {
            self.result().display_source(meta);
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
impl DisplaySource for ValueBindRef {
    fn display_source(&self, meta: &mut DisplaySourceMeta) {
        if let val @ Value::DExp(_) = self.value() {
            meta.push("(%");
            val.display_source(meta);
            meta.push(")");
        } else {
            self.value().display_source(meta);
        }
        meta.push("->");
        match self.bind_target() {
            ValueBindRefTarget::NameBind(bind) => bind.display_source(meta),
            ValueBindRefTarget::Binder => meta.push(".."),
            ValueBindRefTarget::ResultHandle => meta.push("$"),
        }
    }
}
impl DisplaySource for ValueBind {
    fn display_source(&self, meta: &mut DisplaySourceMeta) {
        if let Value::DExp(_) = &*self.0 {
            meta.push("(%");
            self.0.display_source(meta);
            meta.push(")");
        } else {
            self.0.display_source(meta);
        }
        meta.push(".");
        self.1.display_source(meta);
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
        lab.display_source(meta);
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

        self.0.display_source(meta);
        meta.add_space();

        meta.push("=");
        meta.add_space();

        self.1.display_source(meta);

        meta.push(";");

        inline_labs(&self.2, meta);
    }
}
impl DisplaySource for Take {
    fn display_source(&self, meta: &mut DisplaySourceMeta) {
        meta.push("take");
        meta.add_space();

        self.0.display_source(meta);
        meta.add_space();

        meta.push("=");
        meta.add_space();

        self.1.display_source(meta);
        meta.push(";");
    }
}
impl DisplaySource for ConstKey {
    fn display_source(&self, meta: &mut DisplaySourceMeta) {
        match self {
            Self::Var(var) => var.display_source(meta),
            Self::ValueBind(vbind) => vbind.display_source(meta),
        }
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
            Self::SetArgs(args) => {
                meta.do_insert_first("# ".into(), |meta| {
                    meta.push("setArgs");
                    if !args.as_normal()
                        .map(Vec::is_empty)
                        .unwrap_or_default()
                    {
                        meta.add_space();
                    }

                    args.display_source(meta);
                    meta.push(";");
                });
            },
            Self::NoOp => meta.push("noop;"),
            Self::Label(lab) => {
                meta.push(":");
                meta.push(&Value::replace_ident(lab))
            },
            Self::Goto(goto) => goto.display_source(meta),
            Self::Op(op) => op.display_source(meta),
            Self::Select(select) => select.display_source(meta),
            Self::GSwitch(gswitch) => gswitch.display_source(meta),
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
            Self::ArgsRepeat(args_repeat) => args_repeat.display_source(meta),
            Self::Match(r#match) => r#match.display_source(meta),
            Self::ConstMatch(r#match) => r#match.display_source(meta),
            Self::Other(args) => {
                if let Some(args) = args.as_normal() {
                    assert_ne!(args.len(), 0);
                }
                args.display_source(meta);
                meta.push(";");
            },
        }
    }
}
impl DisplaySource for Args {
    fn display_source(&self, meta: &mut DisplaySourceMeta) {
        match self {
            Args::Normal(args) => {
                meta.display_source_iter_by_splitter(
                    DisplaySourceMeta::add_space, args);
            },
            Args::Expanded(prefix, suffix) => {
                prefix.iter().for_each(|arg| {
                    arg.display_source(meta);
                    meta.add_space();
                });

                meta.push("@");

                suffix.iter().for_each(|arg| {
                    meta.add_space();
                    arg.display_source(meta);
                });
            },
        }
    }
}
impl DisplaySource for ArgsRepeat {
    fn display_source(&self, meta: &mut DisplaySourceMeta) {
        meta.push("inline");
        meta.add_space();
        meta.push(&self.count().to_string());
        meta.push("@");
        meta.push("{");
        if !self.block().is_empty() {
            meta.add_lf();
            meta.do_block(|meta| {
                self.block().display_source(meta);
            });
        }
        meta.push("}");
    }
}
impl DisplaySource for MatchPatAtom {
    fn display_source(&self, meta: &mut DisplaySourceMeta) {
        let show_name = !self.name().is_empty();
        let show_list = !self.pattern().is_empty();
        if show_name {
            meta.push(self.name());
            if show_list { meta.push(":") }
        }
        if show_list {
            meta.push("[");
            meta.display_source_iter_by_splitter(
                DisplaySourceMeta::add_space,
                self.pattern()
            );
            meta.push("]");
        }
    }
}
impl DisplaySource for MatchPat {
    fn display_source(&self, meta: &mut DisplaySourceMeta) {
        match self {
            MatchPat::Normal(args) => {
                meta.display_source_iter_by_splitter(
                    DisplaySourceMeta::add_space,
                    args,
                )
            },
            MatchPat::Expanded(prefix, suffix) => {
                for s in prefix {
                    s.display_source(meta);
                    meta.add_space();
                }
                meta.push("@");
                for s in suffix {
                    meta.add_space();
                    s.display_source(meta);
                }
            },
        }
    }
}
impl DisplaySource for Match {
    fn display_source(&self, meta: &mut DisplaySourceMeta) {
        meta.push("match");
        meta.add_space();
        let slen = meta.len();
        self.args().display_source(meta);
        if meta.len() != slen { meta.add_space(); }
        meta.push("{");
        if !self.cases().is_empty() {
            meta.add_lf();
            meta.do_block(|meta| {
                self.cases().iter().for_each(|(pat, block)| {
                    let slen = meta.len();
                    pat.display_source(meta);
                    if meta.len() != slen { meta.add_space(); }
                    meta.push("{");
                    if !block.is_empty() {
                        meta.add_lf();
                        meta.do_block(|meta| block.display_source(meta));
                    }
                    meta.push("}");
                    meta.add_lf();
                });
            });
        }
        meta.push("}");
    }
}
impl DisplaySource for ConstMatchPatAtom {
    fn display_source(&self, meta: &mut DisplaySourceMeta) {
        let show_list = self.pattern()
            .as_ref()
            .left()
            .map(|pats| !pats.is_empty())
            .unwrap_or_default()
            || self.pattern().is_right();
        if self.do_take() {
            meta.push("*")
        }
        if !self.name().is_empty() {
            meta.push(self.name());
            if show_list { meta.push(":") }
        } else if !show_list {
            meta.push("_")
        }
        if show_list {
            meta.push("[");
            if self.pattern().is_right() {
                meta.push("?");
            }
            meta.display_source_iter_by_splitter(
                DisplaySourceMeta::add_space,
                self.pattern()
                    .as_ref()
                    .map_right(Some)
                    .into_iter()
            );
            meta.push("]");
        }
    }
}
impl DisplaySource for ConstMatchPat {
    fn display_source(&self, meta: &mut DisplaySourceMeta) {
        match self {
            ConstMatchPat::Normal(args) => {
                meta.display_source_iter_by_splitter(
                    DisplaySourceMeta::add_space,
                    args,
                )
            },
            ConstMatchPat::Expanded(prefix, suffix) => {
                for s in prefix {
                    s.display_source(meta);
                    meta.add_space();
                }
                meta.push("@");
                for s in suffix {
                    meta.add_space();
                    s.display_source(meta);
                }
            },
        }
    }
}
impl DisplaySource for ConstMatch {
    fn display_source(&self, meta: &mut DisplaySourceMeta) {
        meta.push("const");
        meta.add_space();
        meta.push("match");
        meta.add_space();
        let slen = meta.len();
        self.args().display_source(meta);
        if meta.len() != slen { meta.add_space(); }
        meta.push("{");
        if !self.cases().is_empty() {
            meta.add_lf();
            meta.do_block(|meta| {
                for (pat, block) in self.cases() {
                    let slen = meta.len();
                    pat.display_source(meta);
                    if meta.len() != slen { meta.add_space(); }
                    meta.push("{");
                    if !block.is_empty() {
                        meta.add_lf();
                        meta.do_block(|meta| block.display_source(meta));
                    }
                    meta.push("}");
                    meta.add_lf();
                }
            });
        }
        meta.push("}");
    }
}
impl DisplaySource for GSwitchCase {
    fn display_source(&self, meta: &mut DisplaySourceMeta) {
        match *self {
            Self::Catch {
                skip_extra,
                underflow,
                missed,
                overflow,
                ref to,
            } => {
                if skip_extra {
                    meta.push("*");
                }
                meta.add_space();
                if underflow {
                    meta.push("<");
                }
                if missed {
                    meta.push("!")
                }
                if overflow {
                    meta.push(">")
                }
                if let Some(key) = to {
                    meta.add_space();
                    key.display_source(meta);
                }
            },
            Self::Normal {
                skip_extra,
                ref ids,
                ref guard
            } => {
                if skip_extra {
                    meta.push("*");
                }
                if !ids.as_normal().map(Vec::is_empty).unwrap_or_default() {
                    meta.add_space();
                    ids.display_source(meta);
                }
                if let Some(guard) = guard {
                    meta.add_space();
                    meta.push("if");
                    meta.add_space();
                    guard.display_source(meta);
                }
            },
        }
    }
}
impl DisplaySource for GSwitch {
    fn display_source(&self, meta: &mut DisplaySourceMeta) {
        meta.push("gswitch");
        meta.add_space();
        self.value.display_source(meta);
        meta.add_space();
        meta.push("{");
        meta.add_lf();
        meta.do_block(|meta| {
            self.extra.display_source(meta);
        });
        for (case, expand) in &self.cases {
            meta.push("case");
            case.display_source(meta);
            meta.push(":");
            meta.add_lf();
            meta.do_block(|meta| {
                expand.display_source(meta);
            })
        }
        meta.push("}");
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

    let top_parser = TopLevelParser::new();
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
        parse!(jumpcmp_parser, "(=>[A B] X == 2)")
            .unwrap()
            .display_source_and_get(&mut meta),
        "({\n    inline {}\n    # setArgs A B;\n} => X == 2)"
    );
    assert_eq!(
        parse!(jumpcmp_parser, "(=>[] X == 2)")
            .unwrap()
            .display_source_and_get(&mut meta),
        "({\n    inline {}\n    # setArgs;\n} => X == 2)"
    );
    assert_eq!(
        parse!(jumpcmp_parser, "(=>[@] X == 2)")
            .unwrap()
            .display_source_and_get(&mut meta),
        "({\n    inline {}\n    # setArgs @;\n} => X == 2)"
    );
    assert_eq!(
        parse!(line_parser, r#"set a "\n\\\[hi]\\n";"#)
            .unwrap()
            .display_source_and_get(&mut meta),
        r#"`'set'` a "\n\\[[hi]\\n";"#
    );
    assert_eq!(
        parse!(line_parser, r#"foo bar baz;"#)
            .unwrap()
            .display_source_and_get(&mut meta),
        r#"foo bar baz;"#
    );
    assert_eq!(
        parse!(line_parser, r#"foo @ bar baz;"#)
            .unwrap()
            .display_source_and_get(&mut meta),
        r#"foo @ bar baz;"#
    );
    assert_eq!(
        parse!(line_parser, r#"@ bar baz;"#)
            .unwrap()
            .display_source_and_get(&mut meta),
        r#"@ bar baz;"#
    );
    assert_eq!(
        parse!(line_parser, r#"foo @;"#)
            .unwrap()
            .display_source_and_get(&mut meta),
        r#"foo @;"#
    );
    assert_eq!(
        parse!(line_parser, r#"@;"#)
            .unwrap()
            .display_source_and_get(&mut meta),
        r#"@;"#
    );
    assert_eq!(
        parse!(line_parser, r#"inline @{}"#)
            .unwrap()
            .display_source_and_get(&mut meta),
        r#"inline 1@{}"#
    );
    assert_eq!(
        parse!(line_parser, r#"inline 23@{}"#)
            .unwrap()
            .display_source_and_get(&mut meta),
        r#"inline 23@{}"#
    );
    assert_eq!(
        parse!(line_parser, r#"print @;"#)
            .unwrap()
            .display_source_and_get(&mut meta),
        "inline 1@{\n    `'print'` @;\n}"
    );
    assert_eq!(
        parse!(line_parser, r#"print a b @ c d;"#)
            .unwrap()
            .display_source_and_get(&mut meta),
        "\
        inline {\n\
     \x20   `'print'` a;\n\
     \x20   `'print'` b;\n\
     \x20   inline 1@{\n\
     \x20       `'print'` @;\n\
     \x20   }\n\
     \x20   `'print'` c;\n\
     \x20   `'print'` d;\n\
        }"
    );
    assert_eq!(
        parse!(line_parser, r#"
        match a b c @ d e f {
            x y:[m n] [a b] {
                foo;
            }
            x @ {
                bar;
            }
            {}
            @ {}
        }
        "#)
            .unwrap()
            .display_source_and_get(&mut meta),
        "\
        match a b c @ d e f {\n\
     \x20   x y:[m n] [a b] {\n\
     \x20       foo;\n\
     \x20   }\n\
     \x20   x @ {\n\
     \x20       bar;\n\
     \x20   }\n\
     \x20   {}\n\
     \x20   @ {}\n\
        }"
    );
    assert_eq!(
        parse!(line_parser, r#"
        match a b c @ d e f {}
        "#)
            .unwrap()
            .display_source_and_get(&mut meta),
        "match a b c @ d e f {}"
    );
    assert_eq!(
        parse!(line_parser, r#"
        match {}
        "#)
            .unwrap()
            .display_source_and_get(&mut meta),
        "match {}"
    );
    assert_eq!(
        parse!(line_parser, r#"
        foo 'match';
        "#)
            .unwrap()
            .display_source_and_get(&mut meta),
        "foo 'match';"
    );

    assert_eq!(
        parse!(line_parser, r#"
        take Foo[a b].x;
        "#)
            .unwrap()
            .display_source_and_get(&mut meta),
        "take __ = (%(__:\n    # setArgs a b;\n    setres Foo;\n)).x;"
    );

    assert_eq!(
        parse!(line_parser, r#"
        take a.b;
        "#)
            .unwrap()
            .display_source_and_get(&mut meta),
        "take __ = a.b;"
    );

    assert_eq!(
        parse!(line_parser, r#"
        take $.b;
        "#)
            .unwrap()
            .display_source_and_get(&mut meta),
        "take __ = $.b;"
    );

    assert_eq!(
        parse!(top_parser, r#"
        const X = 2;
        const Y = ();
        const Z = (:x);
        "#)
            .unwrap()
            .display_source_and_get(&mut meta),
        "const X = 2;\nconst Y = ();\nconst Z = (:x);#*labels: [x]*#\n"
    );

    assert_eq!(
        parse!(line_parser, r#"
        const X = ([A &B C:2 &D:(:m)](:z));
        "#)
            .unwrap()
            .display_source_and_get(&mut meta),
        "const X = ([A:A &B:B C:2 &D:(:m)#*labels: [m]*#](:z)#*labels: [z]*#);"
    );

    assert_eq!(
        parse!(line_parser, r#"
        const match @ {
            A *B *_ C:[1] *D:[2 3] E:[?x] [1 2] {}
            X @ Y {}
            @ Z {}
            @ {}
        }
        "#)
            .unwrap()
            .display_source_and_get(&mut meta),
        "\
        const match @ {\n\
     \x20   A *B *_ C:[1] *D:[2 3] E:[?x] [1 2] {}\n\
     \x20   X @ Y {}\n\
     \x20   @ Z {}\n\
     \x20   @ {}\n\
        }\
        "
    );
}
