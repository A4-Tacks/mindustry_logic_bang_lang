use std::ops::ControlFlow;

use either::Either;

use crate::{
    Args, ClosuredValue, CmpTree, Cmper, Const, ConstKey, ConstMatchPat, ConstMatchPatAtom, Goto,
    LogicLine, MatchPat, MatchPatAtom, Take, Value, ValueBind,
};

pub fn line(line: &LogicLine, mut f: impl FnMut(&LogicLine) -> ControlFlow<()>) -> ControlFlow<()> {
    walk_internal(line, &mut |elem| {
        match elem {
            Node::Line(logic_line) => f(logic_line),
            _ => ControlFlow::Continue(()),
        }
    })
}

pub fn lines<'a>(lines: impl IntoIterator<Item = &'a LogicLine>, mut f: impl FnMut(&LogicLine) -> ControlFlow<()>) -> ControlFlow<()> {
    for line in lines {
        self::line(line, &mut f)?;
    }
    ControlFlow::Continue(())
}

pub fn nodes<'a>(nodes: impl IntoIterator<Item = impl Into<Node<'a>> + 'a>, mut f: impl FnMut(Node<'_>) -> ControlFlow<()>) -> ControlFlow<()> {
    for node in nodes {
        walk_internal(node.into(), &mut f)?;
    }
    ControlFlow::Continue(())
}

#[derive(Debug, Clone, Copy)]
pub enum Node<'a> {
    Value(&'a Value),
    Line(&'a LogicLine),
    Cmp(&'a CmpTree),
    Key(&'a ConstKey),
    MatchPatAtom(&'a MatchPatAtom),
    ConstMatchPatAtom(&'a ConstMatchPatAtom),
}
impl<'a> From<&'a ConstMatchPatAtom> for Node<'a> {
    fn from(v: &'a ConstMatchPatAtom) -> Self {
        Self::ConstMatchPatAtom(v)
    }
}
impl<'a> From<&'a MatchPatAtom> for Node<'a> {
    fn from(v: &'a MatchPatAtom) -> Self {
        Self::MatchPatAtom(v)
    }
}
impl<'a> From<&'a Box<ConstKey>> for Node<'a> {
    fn from(v: &'a Box<ConstKey>) -> Self {
        Self::Key(v)
    }
}
impl<'a> From<&'a ConstKey> for Node<'a> {
    fn from(v: &'a ConstKey) -> Self {
        Self::Key(v)
    }
}
impl<'a> From<&'a Box<CmpTree>> for Node<'a> {
    fn from(v: &'a Box<CmpTree>) -> Self {
        Self::Cmp(v)
    }
}
impl<'a> From<&'a CmpTree> for Node<'a> {
    fn from(v: &'a CmpTree) -> Self {
        Self::Cmp(v)
    }
}
impl<'a> From<&'a Box<Value>> for Node<'a> {
    fn from(v: &'a Box<Value>) -> Self {
        Self::Value(v)
    }
}
impl<'a> From<&'a Value> for Node<'a> {
    fn from(v: &'a Value) -> Self {
        Self::Value(v)
    }
}

impl<'a> From<&'a LogicLine> for Node<'a> {
    fn from(v: &'a LogicLine) -> Self {
        Self::Line(v)
    }
}

fn walk_internal<'a>(elem: impl Into<Node<'a>>, f: &mut impl FnMut(Node<'_>) -> ControlFlow<()>) -> ControlFlow<()> {
    let elem = elem.into();
    f(elem)?;
    match elem {
        Node::Value(value) => match value {
            Value::Var(_) | Value::ReprVar(_) | Value::ResultHandle(_) | Value::Binder |
            Value::BuiltinFunc(_) => (),
            Value::ValueBind(ValueBind(value, _)) => walk_internal(value, f)?,
            Value::ValueBindRef(it) => walk_internal(&it.value, f)?,
            Value::ClosuredValue(it) => match it {
                ClosuredValue::Uninit { value, .. } => walk_internal(value, f)?,
                ClosuredValue::Inited { .. } | ClosuredValue::Empty => (),
            },
            Value::DExp(dexp) => walk_lines_internal(dexp.lines.iter(), f)?,
            Value::Cmper(Cmper(cmp)) => walk_internal(cmp.as_ref().value, f)?,
        },
        Node::Line(logic_line) => match logic_line {
            LogicLine::Op(op) => {
                let crate::OpInfo { result, arg1, arg2, .. } = op.get_info();
                walk_internal(result, f)?;
                walk_internal(arg1, f)?;
                if let Some(arg2) = arg2 {
                    walk_internal(arg2, f)?;
                }
            },
            LogicLine::Label(_) | LogicLine::NoOp | LogicLine::Ignore | LogicLine::ConstLeak(_) => (),
            LogicLine::Goto(Goto(_var, cmp)) => walk_internal(cmp, f)?,
            LogicLine::Other(args) => walk_args_internal(args, f)?,
            LogicLine::Expand(expand) => walk_lines_internal(expand.iter(), f)?,
            LogicLine::InlineBlock(inline_block) => walk_lines_internal(inline_block.iter(), f)?,
            LogicLine::Select(select) => {
                walk_internal(&select.0, f)?;
                walk_lines_internal(select.1.iter(), f)?;
            },
            LogicLine::GSwitch(gswitch) => {
                walk_internal(&gswitch.value, f)?;
                walk_lines_internal(gswitch.extra.iter(), f)?;
                for (case, body) in gswitch.cases.iter() {
                    match case {
                        crate::GSwitchCase::Catch { to, .. } => {
                            if let Some(to) = to {
                                walk_internal(to, f)?;
                            }
                        },
                        crate::GSwitchCase::Normal { ids, guard, .. } => {
                            walk_args_internal(&ids.value, f)?;
                            if let Some(guard) = guard {
                                walk_internal(guard, f)?;
                            }
                        },
                    }
                    walk_lines_internal(body.iter(), f)?;
                }
            },
            LogicLine::Take(Take(k, v)) |
            LogicLine::Const(Const(k, v, _)) => {
                walk_internal(k, f)?;
                walk_internal(v, f)?;
            },
            LogicLine::SetResultHandle(value, _) => walk_internal(value, f)?,
            LogicLine::SetArgs(args) => walk_args_internal(args, f)?,
            LogicLine::ArgsRepeat(it) => {
                if let Either::Right(count) = it.count.as_ref().value {
                    walk_internal(count, f)?;
                }
                walk_lines_internal(it.block.iter(), f)?;
            },
            LogicLine::Match(it) => {
                walk_args_internal(&it.args, f)?;
                for (pats, body) in it.cases() {
                    walk_matchpat_internal(pats, f)?;
                    walk_lines_internal(body.iter(), f)?;
                }
            },
            LogicLine::ConstMatch(it) => {
                walk_args_internal(it.args(), f)?;
                for (pats, body) in it.cases() {
                    walk_cmatchpat_internal(pats, f)?;
                    walk_lines_internal(body.iter(), f)?;
                }
            },
        },
        Node::Cmp(cmp) => match cmp {
            CmpTree::Deps(inline_block, cmp_tree) => {
                walk_lines_internal(inline_block.iter(), f)?;
                walk_internal(cmp_tree, f)?;
            },
            CmpTree::Expand(_var, cmp_tree) => {
                walk_internal(cmp_tree, f)?;
            },
            CmpTree::Or(cmp_tree, cmp_tree1) |
            CmpTree::And(cmp_tree, cmp_tree1) => {
                walk_internal(cmp_tree, f)?;
                walk_internal(cmp_tree1, f)?;
            },
            CmpTree::Atom(it) => {
                if let Some((lhs, rhs)) = it.get_values_ref() {
                    walk_internal(lhs, f)?;
                    walk_internal(rhs, f)?;
                }
            },
        },
        Node::Key(key) => match key {
            ConstKey::Var(_) | ConstKey::Unused(_) => (),
            ConstKey::ValueBind(ValueBind(value, _)) => walk_internal(value, f)?,
        }
        Node::MatchPatAtom(match_pat_atom) => {
            for value in &match_pat_atom.pattern {
                walk_internal(value, f)?;
            }
        },
        Node::ConstMatchPatAtom(const_match_pat_atom) => {
            for value in const_match_pat_atom.pattern.as_ref().map_right(std::slice::from_ref).into_iter() {
                walk_internal(value, f)?;
            }
        },
    }
    ControlFlow::Continue(())
}

fn walk_lines_internal<'a>(lines: impl IntoIterator<Item = &'a LogicLine>, f: &mut impl FnMut(Node<'_>) -> ControlFlow<()>) -> ControlFlow<()> {
    lines.into_iter().try_for_each(|line| walk_internal(line, f))
}

fn walk_args_internal(args: &Args, f: &mut impl FnMut(Node<'_>) -> ControlFlow<()>) -> ControlFlow<()> {
    match args {
        Args::Normal(values) => values.iter().chain(const { &Vec::new() }),
        Args::Expanded(values, values1) => values.iter().chain(values1),
    }.try_for_each(|value| walk_internal(value, f))
}

fn walk_matchpat_internal(pats: &MatchPat, f: &mut impl FnMut(Node<'_>) -> ControlFlow<()>) -> ControlFlow<()> {
    match pats {
        MatchPat::Normal(match_pat_atoms) => match_pat_atoms.iter().chain(const { &Vec::new() }),
        MatchPat::Expanded(match_pat_atoms, match_pat_atoms1) => match_pat_atoms.iter().chain(match_pat_atoms1),
    }.try_for_each(|value| walk_internal(value, f))
}

fn walk_cmatchpat_internal(pats: &ConstMatchPat, f: &mut impl FnMut(Node<'_>) -> ControlFlow<()>) -> ControlFlow<()> {
    match pats {
        ConstMatchPat::Normal(match_pat_atoms) => match_pat_atoms.iter().chain(const { &Vec::new() }),
        ConstMatchPat::Expanded(match_pat_atoms, _, match_pat_atoms1) => match_pat_atoms.iter().chain(match_pat_atoms1),
    }.try_for_each(|value| walk_internal(value, f))
}
