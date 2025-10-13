use std::{collections::{HashSet}, iter::once, rc::Rc};

use tag_code::logic_parser::{Args, Var};

use crate::{quality::Loss, supp::Cond};

pub mod display_impl;
pub mod make;
pub mod quality;
pub mod supp;
pub mod clean;
pub mod walk;
pub mod patterns;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Jump<'a>(pub Label, pub Cond<'a>);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Label(pub u16);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Reduce<'a> {
    Pure(Rc<[Args<'a>]>),
    Product(Vec<Reduce<'a>>),
    Label(Label),
    Jump(Jump<'a>),
    Break(Cond<'a>),
    Skip(Cond<'a>, Rc<[Reduce<'a>]>),
    DoWhile(Cond<'a>, Rc<[Reduce<'a>]>),
    While(Cond<'a>, Rc<[Reduce<'a>]>, Rc<[Reduce<'a>]>),
    IfElse(Cond<'a>, Rc<[Reduce<'a>]>, Rc<[Reduce<'a>]>),
    GSwitch(Var, Rc<[(usize, Reduce<'a>)]>)
}
impl<'a> From<Jump<'a>> for Reduce<'a> {
    fn from(v: Jump<'a>) -> Self {
        Self::Jump(v)
    }
}

impl<'a> Reduce<'a> {
    pub fn as_label(&self) -> Option<&Label> {
        if let Self::Label(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn as_jump(&self) -> Option<&Jump<'a>> {
        if let Self::Jump(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone)]
pub struct Finder<'a> {
    pub current: HashSet<Rc<[Reduce<'a>]>>,
    pub losses_cache: Vec<f32>,
    pub limit: usize,
}

impl<'a> Finder<'a> {
    pub fn iterate(&mut self) {
        let cases = self.current.iter().cloned().collect::<Vec<_>>();

        cases.iter()
            .flat_map(|case| {
                (0..case.len()).map(|i| case.split_at(i))
            })
            .flat_map(|subcase| Self::patterns().iter()
                .map(move |&pattern| (subcase, pattern)))
            .for_each(|((unprocess, subcase), pattern)|
        {
            if let Some((prefix, reduced, suffix)) = pattern(self, subcase) {
                let new_case = unprocess.iter()
                    .cloned()
                    .chain(prefix)
                    .chain(once(reduced))
                    .chain(suffix.iter().cloned())
                    .collect();
                self.current.insert(new_case);
            }
        });
    }

    pub fn limite(&mut self) -> (f32, f32) {
        let losses = self.current.iter().map(|x| x.loss());
        self.losses_cache.clear();
        self.losses_cache.extend(losses);
        let losses = &mut self.losses_cache;

        let mut count = 0;
        losses.sort_by(|a, b| a.total_cmp(&b));
        let Some(&bound) = losses.get(self.limit) else {
            return (losses[0], losses.last().copied().unwrap())
        };
        self.current.retain(|elem| {
            let loss = elem.loss();
            let retain = match loss.total_cmp(&bound) {
                std::cmp::Ordering::Less => true,
                std::cmp::Ordering::Equal => count < self.limit,
                std::cmp::Ordering::Greater => false,
            };
            if retain {
                count += 1;
            }
            retain
        });

        (losses[0], bound)
    }
}

#[cfg(test)]
mod tests {
    use crate::walk::label_defs;
    use crate::walk::label_usages;
    use tag_code::logic_parser;

    use crate::{display_impl::fmt_reduces, quality::Loss};

    use super::*;

    #[test]
    fn it_works() {
        let logic = r#"
        op mul x input 3
        op add @counter @counter x
        set i 2
        set j 3
        jump end always 0 0
        set i 4
        set j 5
        jump end always 0 0
        set i 7
        set j 9
        jump end always 0 0
        end:
        print i
        "#;
        let mut lines = logic_parser::parser::lines(logic).unwrap();
        lines.index_label_popup();
        println!("{lines:#}");
        lines.unique_label_pairs();
        println!("{lines:#}");
        let reduces = make::make_reduce(lines.lines().iter().map(|x| &x.value));
        let mut finder = Finder {
            current: once(reduces.into()).collect(),
            losses_cache: vec![],
            limit: 900,
        };
        for (i, reduces) in finder.current.iter().enumerate() {
            let loss = reduces.loss();
            println!("\x1b[94m----- reduce {i} <{loss}> -----\x1b[0m");
            println!("{}", fmt_reduces(reduces))
        }
        let mut prev = None;
        for i in 0..35 {
            println!("iterate {i}");
            finder.iterate();
            print!("limite {i} : {}", finder.current.len());
            let (happy, limite) = finder.limite();
            println!(" -> {} <{happy} $ {limite}>", finder.current.len());

            if prev == Some((happy, limite)) { break }

            prev = Some((happy, limite))
        }
        println!("\x1b[92m========== iterate ==========\x1b[0m");
        let mut sorted = finder.current.iter().collect::<Vec<_>>();
        sorted.sort_by(|a, b| a.loss().total_cmp(&b.loss()));
        for (i, reduces) in sorted.iter().take(1).enumerate() {
            let loss = reduces.loss();
            println!("\x1b[94m----- reduce {i} <{loss}> -----\x1b[0m");
            println!("{}", fmt_reduces(reduces))
        }
        println!("\x1b[94m----- clean -----\x1b[0m");
        let cleaned = clean::clean_dedup_labels(sorted[0].iter().cloned().collect());
        let cleaned = clean::clean_jump_to_break(cleaned);
        println!("{}", fmt_reduces(&[cleaned.clone()]));
        let label_usage_count = label_usages(&cleaned);
        println!("label usage count: {}", label_usage_count);

        let label_def_count = label_defs(&cleaned);
        println!("label def count: {}", label_def_count);
    }
}
