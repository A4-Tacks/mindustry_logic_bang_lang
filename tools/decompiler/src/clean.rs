use std::{collections::HashMap, iter};

use crate::{Jump, Label, Reduce, make};

pub fn clean_dedup_labels(reduce: Reduce<'_>) -> Reduce<'_> {
    let mut label_map = HashMap::new();

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

pub fn clean_jump_to_break(reduce: Reduce<'_>) -> Reduce<'_> {
    fn implement(reduce: Reduce<'_>, lab: Option<Label>) -> Reduce<'_> {
        match reduce {
            Reduce::Pure(..) => reduce,
            Reduce::Product(reduces) => {
                let mut iter = reduces.into_iter().peekable();
                iter::from_fn(|| {
                    match iter.next()? {
                        it @ (Reduce::DoWhile(..) | Reduce::While(..)) => {
                            implement(it, iter.peek().and_then(|x| x.as_label()).or(lab.as_ref()).cloned())
                        },
                        it => implement(it, lab.clone()),
                    }.into()
                }).collect()
            },
            Reduce::Jump(Jump(l, cond)) if Some(&l) == lab.as_ref() => {
                Reduce::Break(cond)
            },
            Reduce::DoWhile(cond, sub) => {
                Reduce::DoWhile(cond, sub.iter().cloned()
                    .map(|x| implement(x, lab.clone()))
                    .collect())
            },
            Reduce::While(cond, deps, sub) => {
                let deps = deps.iter().cloned()
                    .map(|x| implement(x, lab.clone()))
                    .collect();
                let sub = sub.iter().cloned()
                    .map(|x| implement(x, lab.clone()))
                    .collect();
                Reduce::While(cond, deps, sub)
            },
            Reduce::IfElse(cond, then, else_br) => {
                let then = then.iter().cloned()
                    .map(|x| implement(x, lab.clone()))
                    .collect();
                let else_br = else_br.iter().cloned()
                    .map(|x| implement(x, lab.clone()))
                    .collect();
                Reduce::IfElse(cond, then, else_br)
            },
            _ => reduce,
        }
    }
    implement(reduce, None)
}
