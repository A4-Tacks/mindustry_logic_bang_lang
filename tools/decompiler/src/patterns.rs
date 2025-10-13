use std::iter::{once, successors};

use tag_code::logic_parser::Args;

use crate::{Finder, Jump, Reduce, supp::{self, Cond, CondOp}};

pub(crate) type HandleRet<'a, 's> = Option<(
    Option<Reduce<'a>>,
    Reduce<'a>,
    &'s [Reduce<'a>],
)>;

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
        ]
    }
}

fn try_skip<'a, 's>(_: &mut Finder<'a>, reduce: &'s [Reduce<'a>]) -> HandleRet<'a, 's> {
    let (Reduce::Jump(Jump(label, cond)), rest) = reduce.split_first()? else {
        return None;
    };
    let (body, _, rest) = supp::sfind(rest, |x| {
        matches!(x, Reduce::Label(l) if l == label)
    })?;
    let skip = Reduce::Skip(cond.clone(), body.into());
    Some((None, skip, rest))
}

fn try_do_while<'a, 's>(_: &mut Finder<'a>, reduce: &'s [Reduce<'a>]) -> HandleRet<'a, 's> {
    let (Reduce::Label(label), rest) = reduce.split_first()? else {
        return None;
    };
    let (body, Reduce::Jump(Jump(_, cond)), rest) = supp::sfind(rest, |x| {
        matches!(x, Reduce::Jump(Jump(l, _)) if l == label)
    })? else { return None };
    let skip = Reduce::DoWhile(cond.clone(), body.into());
    Some((None, skip, rest))
}

fn try_strict_not_equal<'a, 's>(_: &mut Finder<'a>, reduce: &'s [Reduce<'a>]) -> HandleRet<'a, 's> {
    let [
        Reduce::Pure(pur),
        Reduce::Jump(Jump(label, Cond(CondOp::Equal, jarg))),
        rest @ ..
    ] = reduce else { return None };
    let [ja, jb] = arr(jarg)?;
    let (args, prefix) = pur.split_last()?;
    let [op, oper, res, a, b] = arr(args)?;

    if op == "op" && oper == "strictEqual"
        && (ja == res && is_zero(jb) || jb == res && is_zero(ja))
    {
        let cond = Cond(
            CondOp::StrictNotEqual,
            vec![a.clone(), b.clone()].try_into().unwrap(),
        );
        let jump = Reduce::Jump(Jump(label.clone(), cond));
        Some((pack_pures(prefix), jump, rest))
    } else {
        None
    }
}

fn try_single_cond_while_1<'a, 's>(_: &mut Finder<'a>, reduce: &'s [Reduce<'a>]) -> HandleRet<'a, 's> {
    let (Reduce::Jump(Jump(forw_label, Cond(CondOp::Always, _))), rest) = reduce.split_first()? else {
        return None;
    };
    let (Reduce::Label(back_label), rest) = rest.split_first()? else {
        return None;
    };
    let (body, _, rest) = supp::sfind(rest, |x| {
        matches!(x, Reduce::Label(l) if l == forw_label)
    })?;
    let (deps, Reduce::Jump(Jump(_, cond)), rest) = supp::sfind(rest, |x| {
        matches!(x, Reduce::Jump(Jump(l, _)) if l == back_label)
    })? else { return None };
    let r#while = Reduce::While(cond.clone(), deps.into(), body.into());
    Some((None, r#while, rest))
}

fn try_single_cond_while_2<'a, 's>(_: &mut Finder<'a>, reduce: &'s [Reduce<'a>]) -> HandleRet<'a, 's> {
    let (Reduce::Label(back_label), rest) = reduce.split_first()? else {
        return None;
    };
    let (deps, Reduce::Jump(Jump(brk_label, cond)), rest) = supp::sfind(rest, |x| {
        matches!(x, Reduce::Jump(_))
    })? else { return None };
    let (body, _, rest) = supp::sfind(rest, |x| {
        matches!(x, Reduce::Jump(Jump(l, Cond(CondOp::Always, _))) if l == back_label)
    })?;
    let (Reduce::Label(l), rest) = rest.split_first()? else {
        return None;
    };
    if l != brk_label {
        return None;
    }
    let r#while = Reduce::While(cond.clone(), deps.into(), body.into());
    Some((None, r#while, rest))
}

fn try_double_trivia_cond_while_1<'a, 's>(_: &mut Finder<'a>, reduce: &'s [Reduce<'a>]) -> HandleRet<'a, 's> {
    let (Reduce::Jump(Jump(brk_label, Cond(skip_cond, skip_args))), rest) = reduce.split_first()? else {
        return None;
    };
    let (Reduce::DoWhile(Cond(back_cond, back_args), body), rest) = rest.split_first()? else {
        return None;
    };
    if skip_cond.clone().apply_not() != *back_cond || skip_args != back_args {
        return None;
    }
    let (Reduce::Label(l), rest) = rest.split_first()? else {
        return None;
    };
    if l != brk_label {
        return None;
    }
    let cond = Cond(back_cond.clone(), back_args.clone());
    let r#while = Reduce::While(cond, vec![].into(), body.clone());
    Some((None, r#while, rest))
}

fn try_basic_if_else<'a, 's>(_: &mut Finder<'a>, reduce: &'s [Reduce<'a>]) -> HandleRet<'a, 's> {
    let (Reduce::Jump(Jump(then_label, cond)), rest) = reduce.split_first()? else {
        return None;
    };
    let (body, _, rest) = supp::sfind(rest, |x| {
        matches!(x, Reduce::Label(l) if l == then_label)
    })?;
    let (Reduce::Jump(Jump(else_skip_l, Cond(CondOp::Always, _))), body) = body.split_last()? else {
        return None;
    };
    let (else_body, _, rest) = supp::sfind(rest, |x| {
        matches!(x, Reduce::Label(l) if l == else_skip_l)
    })?;
    let r#while = Reduce::IfElse(cond.clone(), body.into(), else_body.into());
    Some((None, r#while, rest))
}

fn try_basic_gswitch<'a, 's>(_: &mut Finder<'a>, reduce: &'s [Reduce<'a>]) -> HandleRet<'a, 's> {
    let [
        Reduce::Pure(pur),
        rest @ ..
    ] = reduce else { return None };
    let (args, prefix) = pur.split_last()?;
    let [op, add, dst, a, rhs] = arr(args)?;
    if op != "op" || add != "add" || dst != "@counter" || a != "@counter" {
        return None;
    }
    let (jumps, rest) = supp::split_at(rest, |x| {
        !matches!(x, Reduce::Jump(_))
    })?;
    if jumps.len() < 3 {
        return None;
    }
    let jump_labels = jumps.iter().map(|it| match it {
        Reduce::Jump(Jump(l, Cond(CondOp::Always, _))) => l.clone().into(),
        _ => None,
    }).collect::<Option<Vec<_>>>()?;
    let mut offseted_cases = Vec::new();
    let mut rest = successors(Some(rest), |rest| {
        let (case, Reduce::Label(l), rest) = supp::sfind(rest, |x| {
            matches!(x, Reduce::Label(l) if jump_labels.contains(l))
        })? else { return None };
        offseted_cases.push((l, case));
        Some(rest)
    }).last().unwrap();
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

pub(crate) fn trim_split_at<'a, 's, F>(slice: &'s [Reduce<'a>], predicate: F) -> Option<(
    &'s [Reduce<'a>],
    &'s [Reduce<'a>],
)>
where F: FnMut(&Reduce<'a>) -> bool,
{
    let basic_slice = supp::split_at(slice, predicate)?.0;
    let basic_slice = successors(basic_slice.into(), |x| {
        match x {
            [back @ .., Reduce::Label(_)] => Some(back),
            _ => None,
        }
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
