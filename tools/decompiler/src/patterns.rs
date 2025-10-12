use crate::{Finder, Jump, Reduce, supp::{self, Cond, CondOp}};

impl<'a> Finder<'a> {
    pub fn patterns<'free>() -> &'free [for<'s> fn(
        &mut Finder<'a>,
        &'s [Reduce<'a>],
    ) -> Option<(Reduce<'a>, &'s [Reduce<'a>])>] {
        &[
            try_skip as _,
            try_do_while as _,
            try_strict_not_equal as _,
            try_single_cond_while_1 as _,
            try_single_cond_while_2 as _,
            try_double_trivia_cond_while_1 as _,
            try_basic_if_else as _,
        ]
    }
}

type HandleRet<'a, 's> = Option<(Reduce<'a>, &'s [Reduce<'a>])>;

fn try_skip<'a, 's>(_: &mut Finder<'a>, reduce: &'s [Reduce<'a>]) -> HandleRet<'a, 's> {
    let (Reduce::Jump(Jump(label, cond)), rest) = reduce.split_first()? else {
        return None;
    };
    let (body, _, rest) = supp::sfind(rest, |x| {
        matches!(x, Reduce::Label(l) if l == label)
    })?;
    let skip = Reduce::Skip(cond.clone(), body.into());
    Some((skip, rest))
}

fn try_do_while<'a, 's>(_: &mut Finder<'a>, reduce: &'s [Reduce<'a>]) -> HandleRet<'a, 's> {
    let (Reduce::Label(label), rest) = reduce.split_first()? else {
        return None;
    };
    let (body, Reduce::Jump(Jump(_, cond)), rest) = supp::sfind(rest, |x| {
        matches!(x, Reduce::Jump(Jump(l, _)) if l == label)
    })? else { return None };
    let skip = Reduce::DoWhile(cond.clone(), body.into());
    Some((skip, rest))
}

fn try_strict_not_equal<'a, 's>(_: &mut Finder<'a>, reduce: &'s [Reduce<'a>]) -> HandleRet<'a, 's> {
    let [
        Reduce::Pure(pur),
        Reduce::Jump(Jump(label, Cond(CondOp::Equal, jarg))),
        rest @ ..
    ] = reduce else { return None };
    let [ja, jb] = arr(jarg)?;
    let [args] = arr(pur)?;
    let [op, oper, res, a, b] = arr(args)?;

    if op == "op" && oper == "strictEqual"
        && (ja == res && is_zero(jb) || jb == res && is_zero(ja))
    {
        let cond = Cond(
            CondOp::StrictNotEqual,
            vec![a.clone(), b.clone()].try_into().unwrap(),
        );
        let jump = Reduce::Jump(Jump(label.clone(), cond));
        Some((jump, rest))
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
    Some((r#while, rest))
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
    Some((r#while, rest))
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
    Some((r#while, rest))
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
    Some((r#while, rest))
}

fn is_zero(value: &str) -> bool {
    value == "false" || value == "0" || value == "0.0"
}

fn arr<T, const N: usize>(value: &[T]) -> Option<&[T; N]> {
    value.try_into().ok()
}
