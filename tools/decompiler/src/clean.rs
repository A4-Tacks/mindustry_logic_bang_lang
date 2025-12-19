use std::{collections::{HashMap, HashSet}, iter};

use crate::{Jump, Label, Reduce, make};

pub fn dedup_labels(reduce: Reduce<'_>) -> Reduce<'_> {
    let mut label_map = HashMap::new();
    let reduce = quiet_unique_label_defs(reduce);

    reduce.walk_reduce_slices(&mut |reduces| {
        reduces.iter().reduce(|a, b| {
            match (a, b) {
                (lhs @ Reduce::Label(a), Reduce::Label(b)) => {
                    label_map.insert(b.clone(), a.clone());
                    lhs
                },
                _ => b,
            }
        });
    });

    make::remake_reduce(reduce, &mut |reduce| match reduce {
        Reduce::Label(label) if label_map.contains_key(&label) => None,
        Reduce::Jump(Jump(label, cond)) => {
            let label = label_map.get(&label).cloned().unwrap_or(label);
            Some(Reduce::Jump(Jump(label, cond)))
        },
        _ => Some(reduce)
    }).unwrap()
}

pub fn quiet_unique_label_defs(reduce: Reduce<'_>) -> Reduce<'_> {
    let mut dup_counter = HashMap::new();

    reduce.walk_label_defs(&mut |l| {
        let count: &mut usize = dup_counter.entry(l.clone()).or_default();
        *count += 1;
    });
    make::remake_reduce(reduce, &mut |reduce| match reduce {
        Reduce::Label(label) if dup_counter.get(&label).is_some_and(|&n| n > 1) => {
            *dup_counter.get_mut(&label).unwrap() -= 1;
            None
        },
        _ => Some(reduce),
    }).unwrap()
}

pub fn unused_labels(reduce: Reduce<'_>) -> Reduce<'_> {
    let mut labels = HashSet::new();

    reduce.walk_label_usages(&mut |l| _ = labels.insert(l.clone()));

    fn each<'a, C>(reduces: impl IntoIterator<Item = Reduce<'a>>, labels: &HashSet<Label>) -> C
    where C: FromIterator<Reduce<'a>>
    {
        reduces.into_iter().filter_map(|reduce| match reduce {
            Reduce::Label(label) if !labels.contains(&label) => None,
            _ => Some(implement(reduce, labels)),
        }).collect()
    }

    fn implement<'a>(reduce: Reduce<'a>, labels: &HashSet<Label>) -> Reduce<'a> {
        match reduce {
            Reduce::Pure(..) => reduce,
            Reduce::Label(..) => reduce,
            Reduce::Jump(..) => reduce,
            Reduce::Break(..) => reduce,
            Reduce::Product(reduces) => each(reduces, labels),
            Reduce::Skip(cond, reduces) => {
                Reduce::Skip(cond, each(reduces.iter().cloned(), labels))
            },
            Reduce::DoWhile(cond, reduces) => {
                Reduce::DoWhile(cond, each(reduces.iter().cloned(), labels))
            },
            Reduce::While(cond, deps, reduces) => {
                Reduce::While(cond,
                              each(deps.iter().cloned(), labels),
                              each(reduces.iter().cloned(), labels))
            },
            Reduce::IfElse(cond, then, else_br) => {
                Reduce::IfElse(cond,
                               each(then.iter().cloned(), labels),
                               each(else_br.iter().cloned(), labels))
            },
            Reduce::GSwitch(var, cases) => {
                let reduces = cases.iter().map(|(_, reduce)| reduce.clone());
                let reduces = each::<Vec<_>>(reduces, labels);
                let cases = cases.iter()
                    .map(|(i, _)| *i).zip(reduces)
                    .collect();
                Reduce::GSwitch(var, cases)
            },
        }
    }

    implement(reduce, &labels)
}

pub fn jump_to_break(reduce: Reduce<'_>) -> Reduce<'_> {
    fn each<'a, C, I>(reduces: I, lab: Option<Label>) -> C
    where I: IntoIterator<Item = Reduce<'a>>,
          C: FromIterator<Reduce<'a>>,
    {
        let mut iter = reduces.into_iter().peekable();
        iter::from_fn(move || {
            match iter.next()? {
                it @ (Reduce::DoWhile(..) | Reduce::While(..) | Reduce::GSwitch(..)) => {
                    implement(it, iter.peek().and_then(Reduce::as_label).cloned())
                },
                it => implement(it, lab.clone()),
            }.into()
        }).collect()
    }
    fn implement(reduce: Reduce<'_>, lab: Option<Label>) -> Reduce<'_> {
        match reduce {
            Reduce::Pure(..) => reduce,
            Reduce::Product(reduces) => each(reduces, lab),
            Reduce::Jump(Jump(l, cond)) if Some(&l) == lab.as_ref() => {
                Reduce::Break(cond)
            },
            Reduce::DoWhile(cond, sub) => {
                Reduce::DoWhile(cond, each(sub.iter().cloned(), lab))
            },
            Reduce::While(cond, deps, sub) => {
                let deps = each(deps.iter().cloned(), lab.clone());
                let sub = each(sub.iter().cloned(), lab);
                Reduce::While(cond, deps, sub)
            },
            Reduce::IfElse(cond, then, else_br) => {
                let then = each(then.iter().cloned(), lab.clone());
                let else_br = each(else_br.iter().cloned(), lab);
                Reduce::IfElse(cond, then, else_br)
            },
            Reduce::Skip(cond, reduces) => {
                Reduce::Skip(cond, each(reduces.iter().cloned(), lab))
            },
            Reduce::GSwitch(var, cases) => {
                let reduces = cases.iter().map(|(_, reduce)| reduce.clone());
                let reduces = each::<Vec<_>, _>(reduces, lab);
                let cases = cases.iter()
                    .map(|(i, _)| *i).zip(reduces)
                    .collect();
                Reduce::GSwitch(var, cases)
            },
            Reduce::Label(_) => reduce,
            Reduce::Break(_) => reduce,
            Reduce::Jump(_) => reduce,
        }
    }
    implement(reduce, None)
}

#[cfg(test)]
mod tests {
    use crate::supp::{Cmp, CondOp};

    use super::*;

    #[test]
    fn test_dedup_dupnames_labels() {
        let deduped = dedup_labels(Reduce::from_iter([
            Reduce::Label(Label(1)),
            Reduce::Label(Label(2)),
            Reduce::Label(Label(2)),
            Reduce::Label(Label(3)),
            Reduce::Label(Label(3)),
            Reduce::Label(Label(3)),
            Reduce::Jump(Jump(Label(1), Cmp::Cond(CondOp::Always, vec!["0"].try_into().unwrap()))),
            Reduce::Jump(Jump(Label(2), Cmp::Cond(CondOp::Always, vec!["0"].try_into().unwrap()))),
            Reduce::Jump(Jump(Label(3), Cmp::Cond(CondOp::Always, vec!["0"].try_into().unwrap()))),
        ]));
        assert_eq!(deduped, Reduce::from_iter([
            Reduce::Label(Label(1)),
            Reduce::Jump(Jump(Label(1), Cmp::Cond(CondOp::Always, vec!["0"].try_into().unwrap()))),
            Reduce::Jump(Jump(Label(1), Cmp::Cond(CondOp::Always, vec!["0"].try_into().unwrap()))),
            Reduce::Jump(Jump(Label(1), Cmp::Cond(CondOp::Always, vec!["0"].try_into().unwrap()))),
        ]));

        let deduped = dedup_labels(Reduce::from_iter([
            Reduce::Label(Label(2)),
            Reduce::Label(Label(2)),
            Reduce::Label(Label(3)),
            Reduce::Label(Label(3)),
            Reduce::Label(Label(3)),
            Reduce::Jump(Jump(Label(2), Cmp::Cond(CondOp::Always, vec!["0"].try_into().unwrap()))),
            Reduce::Jump(Jump(Label(3), Cmp::Cond(CondOp::Always, vec!["0"].try_into().unwrap()))),
        ]));
        assert_eq!(deduped, Reduce::from_iter([
            Reduce::Label(Label(2)),
            Reduce::Jump(Jump(Label(2), Cmp::Cond(CondOp::Always, vec!["0"].try_into().unwrap()))),
            Reduce::Jump(Jump(Label(2), Cmp::Cond(CondOp::Always, vec!["0"].try_into().unwrap()))),
        ]));

        let deduped = dedup_labels(Reduce::from_iter([
            Reduce::Label(Label(3)),
            Reduce::Label(Label(3)),
            Reduce::Label(Label(3)),
            Reduce::Label(Label(3)),
            Reduce::Label(Label(3)),
            Reduce::Jump(Jump(Label(3), Cmp::Cond(CondOp::Always, vec!["0"].try_into().unwrap()))),
            Reduce::Jump(Jump(Label(3), Cmp::Cond(CondOp::Always, vec!["0"].try_into().unwrap()))),
        ]));
        assert_eq!(deduped, Reduce::from_iter([
            Reduce::Label(Label(3)),
            Reduce::Jump(Jump(Label(3), Cmp::Cond(CondOp::Always, vec!["0"].try_into().unwrap()))),
            Reduce::Jump(Jump(Label(3), Cmp::Cond(CondOp::Always, vec!["0"].try_into().unwrap()))),
        ]));

        let deduped = dedup_labels(Reduce::from_iter([
            Reduce::Label(Label(3)),
            Reduce::Label(Label(3)),
            Reduce::Label(Label(3)),
            Reduce::Label(Label(3)),
            Reduce::Label(Label(4)),
            Reduce::Jump(Jump(Label(3), Cmp::Cond(CondOp::Always, vec!["0"].try_into().unwrap()))),
            Reduce::Jump(Jump(Label(4), Cmp::Cond(CondOp::Always, vec!["0"].try_into().unwrap()))),
        ]));
        assert_eq!(deduped, Reduce::from_iter([
            Reduce::Label(Label(3)),
            Reduce::Jump(Jump(Label(3), Cmp::Cond(CondOp::Always, vec!["0"].try_into().unwrap()))),
            Reduce::Jump(Jump(Label(3), Cmp::Cond(CondOp::Always, vec!["0"].try_into().unwrap()))),
        ]));
    }

    #[test]
    fn test_dedup_unchunked_dupnames_labels() {
        let deduped = dedup_labels(Reduce::from_iter([
            Reduce::Label(Label(1)),
            Reduce::Label(Label(2)),
            Reduce::Product(vec![]),
            Reduce::Label(Label(2)),
            Reduce::Label(Label(3)),
            Reduce::Label(Label(3)),
            Reduce::Label(Label(3)),
            Reduce::Jump(Jump(Label(1), Cmp::Cond(CondOp::Always, vec!["0"].try_into().unwrap()))),
            Reduce::Jump(Jump(Label(2), Cmp::Cond(CondOp::Always, vec!["0"].try_into().unwrap()))),
            Reduce::Jump(Jump(Label(3), Cmp::Cond(CondOp::Always, vec!["0"].try_into().unwrap()))),
        ]));
        assert_eq!(deduped, Reduce::from_iter([
            Reduce::Label(Label(1)),
            Reduce::Product(vec![]),
            Reduce::Label(Label(2)),
            Reduce::Jump(Jump(Label(1), Cmp::Cond(CondOp::Always, vec!["0"].try_into().unwrap()))),
            Reduce::Jump(Jump(Label(2), Cmp::Cond(CondOp::Always, vec!["0"].try_into().unwrap()))),
            Reduce::Jump(Jump(Label(2), Cmp::Cond(CondOp::Always, vec!["0"].try_into().unwrap()))),
        ]));
    }
}
