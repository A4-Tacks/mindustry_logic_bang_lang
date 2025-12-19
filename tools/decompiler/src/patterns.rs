use std::iter::{once, successors};

use tag_code::logic_parser::Args;

use crate::{Finder, Jump, Reduce, supp::{self, Cmp, Cond, CondOp}};

pub(crate) type HandleRet<'a, 's> = Option<(
    Option<Reduce<'a>>,
    Reduce<'a>,
    &'s [Reduce<'a>],
)>;

macro_rules! hit {
    ($pat:pat = $e:expr) => {
        let $pat = $e else { return Default::default() };
    };
    ($($pat:pat),+ = $e:expr) => {
        let ($($pat),+) = $e else { return Default::default() };
    };
}

macro_rules! check {
    ($e:expr) => {
        if !$e {
            return Default::default()
        }
    };
}

impl<'a> Finder<'a> {
    pub fn patterns<'free>() -> &'free [for<'s> fn(
        &mut Finder<'a>,
        &'s [Reduce<'a>],
    ) -> HandleRet<'a, 's>] {
        &[
            try_skip as _,
            try_do_while as _,
            try_strict_not_equal as _,
            try_single_cond_while_1 as _,
            try_single_cond_while_2 as _,
            try_double_trivia_cond_while_1 as _,
            try_basic_if_else as _,
            try_basic_gswitch as _,
            try_basic_switch as _,
            try_merge_jump_or as _,
            try_merge_jump_and as _,
        ]
    }
}

fn try_skip<'a, 's>(_: &mut Finder<'a>, reduce: &'s [Reduce<'a>]) -> HandleRet<'a, 's> {
    hit!(Reduce::Jump(Jump(label, cond)), rest = reduce.split_first()?);
    let (body, _, rest) = supp::sfind(rest, |x| {
        matches!(x, Reduce::Label(l) if l == label)
    })?;
    let skip = Reduce::Skip(cond.clone(), body.into());
    Some((None, skip, rest))
}

fn try_do_while<'a, 's>(_: &mut Finder<'a>, reduce: &'s [Reduce<'a>]) -> HandleRet<'a, 's> {
    hit!(Reduce::Label(label), rest = reduce.split_first()?);
    hit!(body, Reduce::Jump(Jump(_, cond)), rest = supp::sfind(rest, |x| {
        matches!(x, Reduce::Jump(Jump(l, _)) if l == label)
    })?);
    let skip = Reduce::DoWhile(cond.clone(), body.into());
    Some((None, skip, rest))
}

fn try_strict_not_equal<'a, 's>(_: &mut Finder<'a>, reduce: &'s [Reduce<'a>]) -> HandleRet<'a, 's> {
    hit!([
        Reduce::Pure(pur),
        Reduce::Jump(Jump(label, Cond(CondOp::Equal, jarg))),
        rest @ ..
    ] = reduce);
    let [ja, jb] = arr(jarg)?;
    let (args, prefix) = pur.split_last()?;
    let [op, oper, res, a, b] = arr(args)?;

    check!(op == "op" && oper == "strictEqual");
    check!(ja == res && is_zero(jb) || jb == res && is_zero(ja));

    let cond = Cond(
        CondOp::StrictNotEqual,
        vec![a.clone(), b.clone()].try_into().unwrap(),
    );
    let jump = Reduce::Jump(Jump(label.clone(), cond));
    Some((pack_pures(prefix), jump, rest))
}

fn try_single_cond_while_1<'a, 's>(_: &mut Finder<'a>, reduce: &'s [Reduce<'a>]) -> HandleRet<'a, 's> {
    hit!(Reduce::Jump(Jump(forw_label, Cond(CondOp::Always, _))), rest = reduce.split_first()?);
    hit!(Reduce::Label(back_label), rest = rest.split_first()?);
    let (body, _, rest) = supp::sfind(rest, |x| {
        matches!(x, Reduce::Label(l) if l == forw_label)
    })?;
    hit!(deps, Reduce::Jump(Jump(_, cond)), rest = supp::sfind(rest, |x| {
        matches!(x, Reduce::Jump(Jump(l, _)) if l == back_label)
    })?);
    let r#while = Reduce::While(cond.clone(), deps.into(), body.into());
    Some((None, r#while, rest))
}

fn try_single_cond_while_2<'a, 's>(_: &mut Finder<'a>, reduce: &'s [Reduce<'a>]) -> HandleRet<'a, 's> {
    hit!(Reduce::Label(back_label), rest = reduce.split_first()?);
    hit!(deps, Reduce::Jump(Jump(brk_label, cond)), rest = supp::sfind(rest, |x| {
        matches!(x, Reduce::Jump(_))
    })?);
    let (body, _, rest) = supp::sfind(rest, |x| {
        matches!(x, Reduce::Jump(Jump(l, Cond(CondOp::Always, _))) if l == back_label)
    })?;
    hit!(Reduce::Label(l), rest = rest.split_first()?);
    check!(l == brk_label);
    let r#while = Reduce::While(cond.clone(), deps.into(), body.into());
    Some((None, r#while, rest))
}

fn try_double_trivia_cond_while_1<'a, 's>(_: &mut Finder<'a>, reduce: &'s [Reduce<'a>]) -> HandleRet<'a, 's> {
    hit!(Reduce::Jump(Jump(brk_label, skip_cond)), rest = reduce.split_first()?);
    hit!(Reduce::DoWhile(back_cond, body), rest = rest.split_first()?);
    check!(skip_cond.apply_not() == *back_cond);
    hit!(Reduce::Label(l), rest = rest.split_first()?);
    check!(l == brk_label);
    let cond = back_cond.clone();
    let r#while = Reduce::While(cond, vec![].into(), body.clone());
    Some((None, r#while, rest))
}

fn try_basic_if_else<'a, 's>(_: &mut Finder<'a>, reduce: &'s [Reduce<'a>]) -> HandleRet<'a, 's> {
    hit!(Reduce::Jump(Jump(then_label, cond)), rest = reduce.split_first()?);
    check!(!cond.is_always());
    let (body, _, rest) = supp::sfind(rest, |x| {
        matches!(x, Reduce::Label(l) if l == then_label)
    })?;
    hit!(Reduce::Jump(Jump(else_skip_l, Cond(CondOp::Always, _))), body = body.split_last()?);
    let (else_body, _, rest) = supp::sfind(rest, |x| {
        matches!(x, Reduce::Label(l) if l == else_skip_l)
    })?;
    check!(!else_body.is_empty() && !body.is_empty());
    let r#while = Reduce::IfElse(cond.clone(), body.into(), else_body.into());
    Some((None, r#while, rest))
}

fn try_basic_gswitch<'a, 's>(_: &mut Finder<'a>, reduce: &'s [Reduce<'a>]) -> HandleRet<'a, 's> {
    hit!([Reduce::Pure(pur), rest @ ..] = reduce);
    let (args, prefix) = pur.split_last()?;
    let [op, add, dst, a, rhs] = arr(args)?;
    check!(op == "op" && add == "add" && dst == "@counter" && a == dst);
    let (jumps, rest) = supp::split_at(rest, |x| {
        !matches!(x, Reduce::Jump(_))
    })?;
    check!(jumps.len() >= 3);
    let jump_labels = jumps.iter().map(|it| match it {
        Reduce::Jump(Jump(l, Cond(CondOp::Always, _))) => l.clone().into(),
        _ => None,
    }).collect::<Option<Vec<_>>>()?;
    let mut offseted_cases = Vec::new();
    let mut rest = successors(Some(rest), |rest| {
        hit!(case, Reduce::Label(l), rest = supp::sfind(rest, |x| {
            matches!(x, Reduce::Label(l) if jump_labels.contains(l))
        })?);
        offseted_cases.push((l, case));
        Some(rest)
    }).last().unwrap();
    check!(offseted_cases.len() == jump_labels.len());
    offseted_cases.iter_mut().reduce(|a, b| {
        a.1 = std::mem::take(&mut b.1);
        b
    });
    let mut cases = offseted_cases;
    let leaves = cases.iter()
        .filter_map(|(_, case)| case.split_last()?.0.as_jump())
        .filter_map(|Jump(l, cond)| cond.is_always().then_some(l))
        .collect::<Vec<_>>();
    if let Some((tail_body, rest1)) = trim_split_at(
        rest,
        |x| { matches!(x, Reduce::Label(l) if leaves.contains(&l)) })
    {
        if let Some((_, tail_case)) = cases.last_mut() {
            assert!(tail_case.is_empty());
            *tail_case = tail_body;
        }
        rest = rest1;
    }

    let cases = cases.iter()
        .flat_map(|&(case_label, body)|
    {
        let mut ths = jump_labels.iter()
            .enumerate()
            .filter_map(move |(i, l)| (l == case_label).then_some(i));
        let last_th = ths.next_back().unwrap();
        ths.map(|th| (th, Reduce::Product(vec![])))
            .chain(once((last_th, body.iter().cloned().collect())))
    });

    let gswitch = Reduce::GSwitch(rhs.clone(), cases.collect());
    let prefix = pack_pures(prefix);

    Some((prefix, gswitch, rest))
}

fn try_basic_switch<'a, 's>(_: &mut Finder<'a>, reduce: &'s [Reduce<'a>]) -> HandleRet<'a, 's> {
    hit!([Reduce::Pure(pur), reduce_rest @ ..] = reduce);
    let index = pur.iter().rposition(|p| {
        hit!([op, add, dst, lhs, _rhs] = &p[..]);
        op == "op" && add == "add" && dst == "@counter" && lhs == dst
    })?;
    let (prefix, rest) = pur.split_at(index);
    let (steper, prefix) = prefix.split_last()?;
    let [op, mul, step, addr, size] = arr(steper)?;
    check!(op == "op" && mul == "mul");
    let size = size.parse::<usize>().ok().filter(|n| *n > 1)?;

    let (head, spec) = rest.split_first()?;
    let [_op, _add, _dst, _lhs, offset] = arr(head)?;
    check!(offset == step && spec.len()+1 == size);
    let (jmp, mut rest) = reduce_rest.split_first()?;
    let cases = successors(Some((spec, jmp.as_jump()?)), |(spec, _)| {
        hit!(Reduce::Pure(pur), new_rest = rest.split_first()?);
        hit!(Reduce::Jump(jmp), new_rest = new_rest.split_first()?);
        check!(pur.len()+1 == size);
        check!(pur.iter().zip(spec.iter()).all(|(a, b)| a.first() == b.first()));
        rest = new_rest;
        Some((pur, jmp))
    }).enumerate().map(|(i, (body, jmp))| {
        let reduce = pack_pures(body)
            .into_iter()
            .chain(once(jmp.clone().into()))
            .collect();
        (i, reduce)
    }).collect();
    let switch = Reduce::GSwitch(addr.clone(), cases);

    Some((pack_pures(prefix), switch, rest))
}

fn try_merge_jump_or<'a, 's>(_: &mut Finder<'a>, reduce: &'s [Reduce<'a>]) -> HandleRet<'a, 's> {
    hit!([Reduce::Jump(Jump(la, a)), Reduce::Jump(Jump(lb, b)), rest @ ..] = reduce);
    check!(la == lb);
    let cmp = Cmp::Or(a.clone().into(), b.clone().into());
    Some((None, Jump(la.clone(), cmp).into(), rest))
}

fn try_merge_jump_and<'a, 's>(_: &mut Finder<'a>, reduce: &'s [Reduce<'a>]) -> HandleRet<'a, 's> {
    hit!([
        Reduce::Jump(Jump(lskip, a)),
        Reduce::Jump(Jump(lb, b)),
        Reduce::Label(end),
        rest @ ..
    ] = reduce);
    check!(lskip == end);
    let cmp = Cmp::And(a.apply_not().into(), b.clone().into());
    Some((None, Jump(lb.clone(), cmp).into(), rest))
}

pub(crate) fn trim_split_at<'a, 's, F>(slice: &'s [Reduce<'a>], predicate: F) -> Option<(
    &'s [Reduce<'a>],
    &'s [Reduce<'a>],
)>
where F: FnMut(&Reduce<'a>) -> bool,
{
    let basic_slice = supp::split_at(slice, predicate)?.0;
    let basic_slice = successors(basic_slice.into(), |x| {
        hit!([back @ .., Reduce::Label(_)] = x);
        Some(back)
    }).last()?;
    let i = basic_slice.len();
    Some((&slice[..i], &slice[i..]))
}

fn is_zero(value: &str) -> bool {
    value == "false" || value == "0" || value == "0.0"
}

fn arr<T, const N: usize>(value: &[T]) -> Option<&[T; N]> {
    value.try_into().ok()
}

fn pack_pures<'a>(prefix: &[Args<'a>]) -> Option<Reduce<'a>> {
    if prefix.is_empty() {
        return None;
    }
    Reduce::Pure(prefix.into()).into()
}
