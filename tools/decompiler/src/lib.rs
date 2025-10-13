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
            count += 1;
            match loss.total_cmp(&bound) {
                std::cmp::Ordering::Less => true,
                std::cmp::Ordering::Equal => count < self.limit,
                std::cmp::Ordering::Greater => false,
            }
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
set links @links
jump 5 greaterThanEq links 2
wait 0.2
set links @links
jump 2 lessThan links 2
set unit_type @flare
set approach_range 3
getlink sorter 0
sensor my_item sorter @config
sensor sorter_ty sorter @type
op notEqual is_invert sorter_ty @sorter
jump 0 strictEqual my_item null
jump 22 always 0 0
ubind unit_type
jump 17 strictEqual @unit null
sensor __3 @unit @controlled
jump 21 equal __3 false
ubind unit_type
jump 17 strictEqual @unit null
sensor __4 @unit @controlled
jump 17 notEqual __4 false
set my_unit @unit
sensor __5 my_unit @dead
jump 13 notEqual __5 false
sensor ctrler my_unit @controller
jump 27 equal ctrler @this
jump 13 notEqual ctrler @unit
sensor __6 my_unit @controlled
jump 13 equal __6 @ctrlPlayer
ubind my_unit
sensor unit_item @unit @firstItem
sensor unit_item_cap @unit @itemCapacity
jump 34 equal unit_item null
jump 83 notEqual unit_item my_item
jump 51 strictEqual unit_item null
sensor __7 @unit my_item
jump 51 equal __7 false
jump 40 equal is_invert false
ulocate building core false 0 __9 __10 0 __8
jump 48 always 0 0
jump 45 equal links 2
op rand __12 links 0
op max __11 1 __12
getlink __8 __11
jump 46 always 0 0
getlink __8 1
sensor __9 __8 @x
sensor __10 __8 @y
ucontrol approach __9 __10 approach_range 0 0
ucontrol itemDrop __8 unit_item_cap 0 0 0
jump 0 always 0 0
jump 54 notEqual is_invert false
ulocate building core false 0 __15 __16 0 __14
jump 68 always 0 0
set i 1
jump 64 greaterThanEq i links
getlink __14 i
sensor __17 __14 my_item
jump 64 notEqual __17 0
op add i i 1
jump 64 greaterThanEq i links
getlink __14 i
sensor __18 __14 my_item
jump 59 equal __18 0
jump 66 notEqual i links
getlink __14 1
sensor __15 __14 @x
sensor __16 __14 @y
ucontrol approach __15 __16 approach_range 0 0
ucontrol itemTake __14 my_item unit_item_cap 0 0
jump 0 notEqual @links links
sensor __20 sorter @config
jump 0 notEqual __20 my_item
getlink __22 0
sensor __21 __22 @type
jump 0 notEqual __21 sorter_ty
sensor __23 @unit @controller
jump 0 notEqual __23 @this
sensor __24 @unit my_item
jump 51 lessThan __24 unit_item_cap
sensor __25 @unit my_item
jump 37 notEqual __25 false
jump 0 always 0 0
jump 86 notEqual is_invert false
ulocate building core false 0 __27 __28 0 __26
jump 100 always 0 0
set i 1
jump 96 greaterThanEq i links
getlink __26 i
sensor __29 __26 my_item
jump 96 notEqual __29 0
op add i i 1
jump 96 greaterThanEq i links
getlink __26 i
sensor __30 __26 my_item
jump 91 equal __30 0
jump 98 notEqual i links
getlink __26 1
sensor __27 __26 @x
sensor __28 __26 @y
ucontrol approach __27 __28 approach_range 0 0
ucontrol itemDrop @air unit_item_cap 0 0 0
jump 0 notEqual @links links
sensor __32 sorter @config
jump 0 notEqual __32 my_item
getlink __34 0
sensor __33 __34 @type
jump 0 notEqual __33 sorter_ty
sensor __35 @unit @controller
jump 0 notEqual __35 @this
sensor __36 @unit @firstItem
jump 101 notEqual __36 null
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
        for (i, reduces) in sorted[..1].iter().enumerate() {
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
