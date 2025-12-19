use super::{Jump, Label, Reduce};
use crate::supp::{Cond, CondOp};
use std::collections::{HashMap, HashSet};
use tag_code::logic_parser::{Args, ParseLine};

pub fn make_reduce<'a, I>(lines: I) -> Vec<Reduce<'a>>
where
    I: IntoIterator<Item = &'a ParseLine<'a>> + Clone,
{
    check_parse_lines(lines.clone());
    let mut products = vec![];
    let mut pure = vec![];
    let mut label_map = HashMap::new();

    for line in lines {
        match line {
            ParseLine::Args(args) => pure.push(Args::from(args)),
            ParseLine::Label(cow) => {
                fetch_pure(&mut products, &mut pure);
                products.push(Reduce::Label(get_id(&mut label_map, cow)));
            },
            ParseLine::Jump(cow, args) => {
                fetch_pure(&mut products, &mut pure);
                let (cmp, args) = CondOp::with_args(args.into());
                let cond = Cond(cmp, args);
                let jump = Jump(get_id(&mut label_map, cow), cond);
                products.push(Reduce::Jump(jump));
            },
        }
    }
    fetch_pure(&mut products, &mut pure);

    products
}

fn check_parse_lines<'a, I>(lines: I)
where
    I: IntoIterator<Item = &'a ParseLine<'a>>,
{
    let mut label_def_set = HashSet::new();

    let mut prev_line = None;
    let iter = lines.into_iter()
        .filter(|line| {
            !line.is_label() || prev_line.replace(*line) != Some(line)
        });
    for line in iter {
        match line {
            ParseLine::Args(_) => (),
            ParseLine::Jump(_, _) => (),
            ParseLine::Label(cow) => if !label_def_set.insert(cow.as_ref()) {
                panic!("duplicate label `{cow}`")
            },
        }
    }
}

fn get_id<'a>(label_map: &mut HashMap<&'a str, u16>, cow: &'a str) -> Label {
    let cur_idx = label_map.len() as u16;
    let id = *label_map.entry(cow).or_insert(cur_idx);
    Label(id)
}

fn fetch_pure<'a>(products: &mut Vec<Reduce<'a>>, pure: &mut Vec<Args<'a>>) {
    if !pure.is_empty() {
        products.push(Reduce::Pure(pure.drain(..).collect()));
    }
}

impl<'a> FromIterator<Reduce<'a>> for Reduce<'a> {
    fn from_iter<T: IntoIterator<Item = Reduce<'a>>>(iter: T) -> Self {
        Self::Product(iter.into_iter().collect())
    }
}

pub(crate) fn remake_reduce<'a, F>(reduce: Reduce<'a>, f: &mut F) -> Option<Reduce<'a>>
where
    F: FnMut(Reduce<'a>) -> Option<Reduce<'a>>,
{
    let new = match reduce {
        Reduce::Pure(..) | Reduce::Label(..) | Reduce::Jump(..) |
        Reduce::Break(..) => reduce,
        Reduce::Product(reduces) => reduces.into_iter()
            .filter_map(|reduce| remake_reduce(reduce, f))
            .collect(),
        Reduce::Skip(cond, reduces) => {
            Reduce::Skip(cond, reduces.iter().cloned()
                .filter_map(|reduce| remake_reduce(reduce, f))
                .collect())
        },
        Reduce::DoWhile(cond, reduces) => {
            Reduce::DoWhile(cond, reduces.iter().cloned()
                .filter_map(|reduce| remake_reduce(reduce, f))
                .collect())
        },
        Reduce::While(cond, deps, reduces) => {
            let deps = deps.iter().cloned()
                .filter_map(|reduce| remake_reduce(reduce, f))
                .collect();
            let reduces = reduces.iter().cloned()
                .filter_map(|reduce| remake_reduce(reduce, f))
                .collect();
            Reduce::While(cond, deps, reduces)
        },
        Reduce::IfElse(cond, then_br, else_br) => {
            let then_br = then_br.iter().cloned()
                .filter_map(|reduce| remake_reduce(reduce, f))
                .collect();
            let else_br = else_br.iter().cloned()
                .filter_map(|reduce| remake_reduce(reduce, f))
                .collect();
            Reduce::IfElse(cond, then_br, else_br)
        },
        Reduce::GSwitch(var, cases) => {
            let cases = cases.iter()
                .cloned()
                .filter_map(|(i, case)| (i, remake_reduce(case, f)?).into())
                .collect();
            Reduce::GSwitch(var, cases)
        },
    };
    f(new)
}
