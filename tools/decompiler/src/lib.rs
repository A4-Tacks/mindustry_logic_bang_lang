use std::{collections::HashSet, iter::once, rc::Rc};

use tag_code::logic_parser::{Args, Var};

use crate::{quality::Loss, supp::Cmp};

pub mod display_impl;
pub mod make;
pub mod quality;
pub mod supp;
pub mod clean;
pub mod walk;
pub mod patterns;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Jump<'a>(pub Label, pub Cmp<'a>);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Label(pub u16);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Reduce<'a> {
    Pure(Rc<[Args<'a>]>),
    Product(Vec<Reduce<'a>>),
    Label(Label),
    Jump(Jump<'a>),
    Break(Cmp<'a>),
    Skip(Cmp<'a>, Rc<[Reduce<'a>]>),
    DoWhile(Cmp<'a>, Rc<[Reduce<'a>]>),
    While(Cmp<'a>, Rc<[Reduce<'a>]>, Rc<[Reduce<'a>]>),
    IfElse(Cmp<'a>, Rc<[Reduce<'a>]>, Rc<[Reduce<'a>]>),
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
    pub guidance: bool,
}

impl<'a> Finder<'a> {
    pub fn iterate(&mut self) {
        let cases: Vec<_> = if self.guidance {
            self.current.drain().collect()
        } else {
            self.current.iter().cloned().collect()
        };

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

        if self.current.is_empty() {
            self.current.extend(cases);
        }
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

    pub fn current_reduces(&self) -> impl Iterator<Item = Reduce<'a>> + use<'a, '_> {
        self.current.iter()
            .map(|reduces| {
                reduces.iter().cloned().collect()
            })
    }

    pub fn current_cleaned(&self) -> impl Iterator<Item = Reduce<'a>> + use<'a, '_> {
        self.current_reduces()
            .map(|reduce| {
                let cleaned = clean::dedup_labels(reduce);
                let cleaned = clean::jump_to_break(cleaned);
                let cleaned = clean::unused_labels(cleaned);

                cleaned
            })
    }
}

#[cfg(test)]
mod tests {
    use expect_test::Expect;
    use tag_code::logic_parser;

    use crate::quality::Loss;

    use super::*;

    mod tests;

    #[track_caller]
    fn check(logic: &str, expect: Expect) {
        let mut fakeout = String::new();
        macro_rules! log {
            ($($t:tt)*) => {
                std::fmt::write(&mut fakeout, format_args!($($t)*)).unwrap();
                fakeout.push('\n');
            };
        }
        let mut lines = logic_parser::parser::lines(logic).unwrap();

        lines.index_label_popup();
        lines.dup_label_pairs();

        let lines = lines.lines().iter()
            .map(|x| &x.value);
        let reduces = make::make_reduce(lines);

        let mut finder = Finder {
            current: once(reduces.into()).collect(),
            losses_cache: vec![],
            limit: 900,
            guidance: false,
        };

        let mut prev_limite = None;
        let [mut itering, iterate] = [1, 30usize];
        loop {
            finder.iterate();

            let raw_len = finder.current.len();
            let (happy, limite) = finder.limite();

            log!("{itering:>3}/{iterate:<3} limite {raw_len:>8} \
                    -> {curlen:<8} <{happy:.5} $ {limite:.5}>",
                curlen=finder.current.len(),
            );

            if Some((happy, limite, raw_len)) == prev_limite {
                log!("-- Early Reconstruction Completed");
                break;
            }
            prev_limite = Some((happy, limite, raw_len));

            itering += 1;
            if itering > iterate {
                break
            }
        }

        log!("");

        let mut sorted = finder.current.iter().collect::<Vec<_>>();
        sorted.sort_by(|a, b| a.loss().total_cmp(&b.loss()));

        for (i, &reduces) in sorted.iter().enumerate() {
            let loss = reduces.loss();
            let reduce = reduces.iter().cloned().collect::<Reduce<'_>>();

            let cleaned = clean::dedup_labels(reduce);
            let cleaned = clean::jump_to_break(cleaned);
            let result = clean::unused_labels(cleaned);

            let def = walk::label_defs(&result);
            let used = walk::label_usages(&result);

            if i == 0 {
                log!("#---------- reduce[{def}/{used}] case {i} <{loss}> ----------");
                log!("{result:x}");
            }

            println!("#---------- reduce[{def}/{used}] case {i} <{loss}> ----------");
            println!("{result:#x}");
        }

        expect.assert_eq(&fakeout);
    }
}
