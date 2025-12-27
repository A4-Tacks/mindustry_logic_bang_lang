use syntax::*;
use crate::{DisplaySource, DisplaySourceMeta};
use either::{self, Left, Right};

fn inline_labs_and_binder<T>(
    labs: &[T],
    binder: Option<&Var>,
    meta: &mut DisplaySourceMeta,
)
where T: DisplaySource
{
    if !labs.is_empty() || binder.is_some() {
        meta.push("#*");
        if !labs.is_empty() {
            meta.push("labels: [");
            meta.display_source_iter_by_splitter(
                |meta| meta.push(", "),
                labs,
            );
            meta.push("]");
        }
        if let Some(binder) = binder {
            if !labs.is_empty() { meta.add_space() }
            meta.push("binder: ");
            binder.display_source(meta);
        }
        meta.push("*#");
    }
}
fn inline_labs<T>(
    labs: &[T],
    meta: &mut DisplaySourceMeta,
)
where T: DisplaySource
{
    inline_labs_and_binder(labs, None, meta)
}

fn is_oneline(line: &LogicLine) -> bool {
    matches!(line,
        | LogicLine::SetArgs(..)
        | LogicLine::ConstLeak(..)
        | LogicLine::Ignore
        )
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
        match self {
            ClosuredValue::Uninit {
                catch_values,
                catch_labels,
                binder_to,
                value,
                labels,
                catch_args,
                lazy,
            } => {
                meta.push("([");
                meta.display_source_iter_by_space(catch_values);
                if *catch_args {
                    if !catch_values.is_empty() {
                        meta.add_space();
                    }
                    meta.push("@");
                }
                if let Some(to) = binder_to {
                    if !catch_values.is_empty() || *catch_args {
                        meta.add_space();
                    }
                    meta.push("..");
                    to.display_source(meta);
                }
                if !catch_labels.is_empty() {
                    if !catch_values.is_empty() || *catch_args {
                        meta.add_space();
                    }
                    meta.push("|");
                    for label in catch_labels {
                        meta.add_space();
                        meta.push(":");
                        label.display_source(meta);
                    }
                }
                meta.push("]");
                if *lazy { meta.push("#*lazy*#"); }
                value.display_source(meta);
                inline_labs(labels, meta);
                meta.push(")");
            },
            ClosuredValue::Inited {
                bind_handle,
                binder_to,
                rename_labels,
                vars,
                reset_argc: reset_args,
                lazy,
            } => {
                struct CatchedVar<'a>(&'a Var);
                impl DisplaySource for CatchedVar<'_> {
                    fn display_source(
                        &self,
                        meta: &mut DisplaySourceMeta,
                    ) {
                        self.0.display_source(meta);
                        meta.push(":__catched");
                    }
                }
                let catcheds = vars.iter()
                    .map(CatchedVar);
                meta.push("([");
                meta.display_source_iter_by_space(catcheds);
                if let Some(args) = reset_args {
                    if !vars.is_empty() {
                        meta.add_space();
                    }
                    meta.push("@(");
                    meta.push_fmt(args);
                    meta.push(")");
                }
                if let Some(to) = binder_to {
                    if !reset_args.is_some() || !vars.is_empty() {
                        meta.add_space();
                    }
                    meta.push("..");
                    to.display_source(meta);
                }
                if !rename_labels.is_empty() {
                    if !vars.is_empty() || reset_args.is_some() {
                        meta.add_space();
                    }
                    meta.push("|");
                    for (src, dst) in rename_labels {
                        meta.add_space();
                        meta.push(":");
                        src.display_source(meta);
                        meta.push("->:");
                        dst.display_source(meta);
                    }
                }
                meta.push("]");
                if *lazy { meta.push("#*lazy*#"); }
                bind_handle.display_source(meta);
                meta.push(")");
            },
            ClosuredValue::Empty => unreachable!(),
        }
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
            Self::ResultHandle(_) => meta.push("$"),
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
            if !self.take_result() { meta.push("`"); }
            self.result().display_source(meta);
            if !self.take_result() { meta.push("`"); }
            meta.push(":");
        }
        match self.lines().len() {
            0 => (),
            1 if !is_oneline(&self.lines()[0]) => {
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
            ValueBindRefTarget::Binder(_) => meta.push(".."),
            ValueBindRefTarget::ResultHandle => meta.push("$"),
            ValueBindRefTarget::Op => meta.push("op"),
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
        if let Self::Always | Self::Never = self {
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
                match &deps[..] {
                    [line] if !line.is_set_args() => {
                        line.display_source(meta)
                    },
                    _ => {
                        meta.do_block(|meta| {
                            meta.add_lf();
                            deps.display_source(meta)
                        })
                    },
                }
                meta.push("}");
                meta.add_space();
                meta.push("=>");
                meta.add_space();
                cmp.display_source(meta);
                meta.push(")");
            },
            Self::Expand(..) => {
                unreachable!("应只在build处理过程中使用的变体, 所以不可能达到")
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
                Not, Abs, Sign, Log, Log10, Floor, Ceil, Round, Sqrt,
                Rand, Sin, Cos, Tan, Asin, Acos, Atan,
            ]
            op2: [
                Add, Sub, Mul, Div, Idiv, LogN,
                Mod, EMod, Pow, Equal, NotEqual, Land,
                LessThan, LessThanEq, GreaterThan, GreaterThanEq, StrictEqual,
                Shl, Shr, UShr, Or, And, Xor,
            ]
            op2l: [
                Max, Min, Angle, AngleDiff, Len, Noise,
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
            Self::Unused(var) => var.display_source(meta),
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
            Self::SetResultHandle(val, _) => {
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
                meta.display_source_iter_by_space(args);
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
        match &**self.count() {
            Left(n) => {
                meta.add_space();
                meta.push_fmt(n);
            },
            Right(value) => {
                meta.push("*");
                value.display_source(meta);
            },
        }
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
        if self.set_res() {
            meta.push("$");
        }
        if show_name {
            meta.push(self.name());
            if show_list { meta.push(":") }
        }
        if show_list {
            meta.push("[");
            meta.display_source_iter_by_space(self.pattern());
            meta.push("]");
        }
        if !show_name && !show_list {
            meta.push("_");
        }
    }
}
impl DisplaySource for MatchPat {
    fn display_source(&self, meta: &mut DisplaySourceMeta) {
        match self {
            MatchPat::Normal(args) => {
                meta.display_source_iter_by_space(args)
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
        if self.set_res() {
            meta.push("$")
        }
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
            meta.display_source_iter_by_space(
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
                meta.display_source_iter_by_space(args)
            },
            ConstMatchPat::Expanded(prefix, do_take, suffix) => {
                for s in prefix {
                    s.display_source(meta);
                    meta.add_space();
                }
                if *do_take {
                    meta.push("*")
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
impl DisplaySource for BindsDisplayer<'_> {
    fn display_source(&self, meta: &mut DisplaySourceMeta) {
        meta.push("(__:");
        meta.add_lf();
        meta.do_block(|meta| {
            meta.push("setres");
            meta.add_space();
            Value::ReprVar(self.handle.clone())
                .display_source(meta);
            meta.push(";");
            self.bind_names.take().unwrap()
                .enumerate()
                .for_each(|(i, (name, data))| {
                    if i == 0 { meta.add_lf(); }
                    meta.add_lf();
                    meta.push("const");
                    meta.add_space();
                    meta.push("$.");
                    name.display_source(meta);
                    meta.add_space();
                    meta.push("=");
                    meta.add_space();
                    data.value().display_source(meta);
                    inline_labs_and_binder(
                        data.labels(),
                        data.binder(),
                        meta,
                    );
                    meta.push(";");
                })
        });
        meta.add_lf();
        meta.push(")");
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
    let mut meta = Default::default();

    macro_rules! check {
        ( $src:expr, $expected:expr $(,)? ) => {
            assert_eq!(
                parse!(top_parser, $src).unwrap()
                    .display_source_and_get(&mut meta)
                    .trim_end(),
                $expected
            );
        };
    }

    check!(
        r#"'abc' 'abc"def' "str" "str'str" 'no_str' '2';"#,
        r#"abc 'abc"def' "str" "str'str" no_str 2;"#,
    );
    assert_eq!(
        JumpCmp::GreaterThan("a".into(), "1".into())
            .display_source_and_get(&mut meta)
            .trim_end(),
        "a > 1"
    );
    check!(
        "goto(a < b && c < d && e < f);",
        "goto(((a < b && c < d) && e < f));",
    );
    check!(
        "{foo;}",
        "{\n    foo;\n}",
    );
    check!(
        "print ($ = x;);",
        "`'print'` (`set` $ x;);",
    );
    check!(
        "print (res: $ = x;);",
        "`'print'` (res: `set` $ x;);",
    );
    check!(
        "print (noop;$ = x;);",
        "`'print'` (\n    noop;\n    `set` $ x;\n);",
    );
    check!(
        "print (res: noop;$ = x;);",
        "`'print'` (res:\n    noop;\n    `set` $ x;\n);",
    );
    check!(
        "print a.b.c;",
        "`'print'` a.b.c;",
    );
    check!(
        "op add a b c;",
        "op a b + c;",
    );
    check!(
        "op x noise a b;",
        "op x noise a b;",
    );
    check!(
        "foo 1 0b1111_0000 0x8f_ee abc 你我他 _x @a-b;",
        "foo 1 0b11110000 0x8fee abc 你我他 _x @a-b;",
    );
    check!(
        "'take' '1._2' '0b_11_00' '-0b1111_0000' '-0x8f' 'a-bc';",
        "'take' '1._2' '0b_11_00' '-0b1111_0000' '-0x8f' 'a-bc';",
    );
    check!(
        "'take' 'set' 'print' 'const' 'take' 'op';",
        "'take' set 'print' 'const' 'take' 'op';",
    );
    check!(
        "goto(({take X = N;} => X > 10 && X < 50));",
        "goto(({take X = N;} => (X > 10 && X < 50)));",
    );
    check!(
        "goto(({take X = A; take Y = B;} => X > 10 && Y > 20 && X < Y));",
        "goto(({\n    take X = A;\n    take Y = B;\n} => ((X > 10 && Y > 20) && X < Y)));",
    );
    check!(
        "goto((=>[A B] X == 2));",
        "goto(({\n    # setArgs A B;\n} => X == 2));",
    );
    check!(
        "goto((=>[] X == 2));",
        "goto(({\n    # setArgs;\n} => X == 2));",
    );
    check!(
        "goto((=>[@] X == 2));",
        "goto(({\n    # setArgs @;\n} => X == 2));",
    );
    check!(
        r#"set a "\n\\\[hi]\\n";"#,
        r#"set a "\n\\[[hi]\\n";"#,
    );
    check!(
        r#"foo bar baz;"#,
        r#"foo bar baz;"#,
    );
    check!(
        r#"foo @ bar baz;"#,
        r#"foo @ bar baz;"#,
    );
    check!(
        r#"@ bar baz;"#,
        r#"@ bar baz;"#,
    );
    check!(
        r#"foo @;"#,
        r#"foo @;"#,
    );
    check!(
        r#"@;"#,
        r#"@;"#,
    );
    check!(
        r#"inline @{}"#,
        r#"inline 1@{}"#,
    );
    check!(
        r#"inline 23@{}"#,
        r#"inline 23@{}"#,
    );
    check!(
        r#"print @;"#,
        "inline 1@{\n    `'print'` @;\n}",
    );
    check!(
        r#"print a b @ c d;"#,
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
    check!(
        r#"
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
        "#,
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
    check!(
        r#"
        match a b c @ d e f {}
        "#,
        "match a b c @ d e f {}"
    );
    check!(
        r#"
        match {}
        "#,
        "match {}"
    );
    check!(
        r#"
        match { $X {} }
        "#,
        "match {\n    $X {}\n}"
    );
    check!(
        r#"
        match { $X:[1] {} }
        "#,
        "match {\n    $X:[1] {}\n}"
    );
    check!(
        r#"
        match { $[1 2] {} }
        "#,
        "match {\n    $[1 2] {}\n}"
    );
    check!(
        r#"
        match { _ {} }
        "#,
        "match {\n    _ {}\n}"
    );
    check!(
        r#"
        match { $_ {} }
        "#,
        "match {\n    $_ {}\n}"
    );
    check!(
        r#"
        foo 'match';
        "#,
        "foo 'match';"
    );

    check!(
        r#"
        take Foo[a b].x;
        "#,
        "take __ = (%(__:\n    # setArgs a b;\n    setres Foo;\n)).x;"
    );

    check!(
        r#"
        take a.b;
        "#,
        "take __ = a.b;"
    );

    check!(
        r#"
        take $.b;
        "#,
        "take __ = $.b;"
    );

    check!(
        r#"
        const X = 2;
        const Y = ();
        const Z = (:x);
        "#,
        "const X = 2;\nconst Y = ();\nconst Z = (:x);#*labels: [x]*#"
    );

    check!(
        r#"
        const X = ([A &B C:2 &D:(:m)](:z));
        "#,
        "const X = ([A:A &B:B C:2 &D:(:m)#*labels: [m]*#](:z)#*labels: [z]*#);"
    );

    check!(
        r#"
        const match @ {
            A *B *_ C:[1] *D:[2 3] E:[?x] [1 2] {}
            X @ Y {}
            @ Z {}
            @ {}
            $_ {}
            $M {}
            $*M {}
            $M:[2] {}
            $[2] {}
        }
        "#,
        "\
        const match @ {\n\
     \x20   A *B *_ C:[1] *D:[2 3] E:[?x] [1 2] {}\n\
     \x20   X @ Y {}\n\
     \x20   @ Z {}\n\
     \x20   @ {}\n\
     \x20   $_ {}\n\
     \x20   $M {}\n\
     \x20   $*M {}\n\
     \x20   $M:[2] {}\n\
     \x20   $[2] {}\n\
        }\
        "
    );

    check!(
        r#"
        const X = ([| :c :d]2);
        "#,
        "const X = ([| :c :d]2);"
    );

    check!(
        r#"
        const X = ([A B | :c :d]2);
        "#,
        "const X = ([A:A B:B | :c :d]2);"
    );

    check!(
        r#"
        const X = ([@]2);
        "#,
        "const X = ([@]2);"
    );

    check!(
        r#"
        const X = ([A @]2);
        "#,
        "const X = ([A:A @]2);"
    );

    check!(
        r#"
        const X = ([A @ | :c]2);
        "#,
        "const X = ([A:A @ | :c]2);"
    );

    check!(
        r#"
        const X = ([@ | :c]2);
        "#,
        "const X = ([@ | :c]2);"
    );

    check!(
        r#"
        (a:) (`b`:);
        "#,
        "(a:) (`b`:);"
    );
}
