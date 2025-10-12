use std::{collections::{HashSet}, iter::once, rc::Rc};

use tag_code::logic_parser::Args;

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
pub struct Label(pub u32);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Reduce<'a> {
    Pure(Rc<[Args<'a>]>),
    Product(Vec<Reduce<'a>>),
    Label(Label),
    Jump(Jump<'a>),
    // TODO 在clean处理break
    Break(Cond<'a>),
    Skip(Cond<'a>, Rc<[Reduce<'a>]>),
    DoWhile(Cond<'a>, Rc<[Reduce<'a>]>),
    While(Cond<'a>, Rc<[Reduce<'a>]>, Rc<[Reduce<'a>]>),
    IfElse(Cond<'a>, Rc<[Reduce<'a>]>, Rc<[Reduce<'a>]>),
}

impl<'a> Reduce<'a> {
    pub fn as_label(&self) -> Option<&Label> {
        if let Self::Label(v) = self {
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
            .for_each(|((prefix, subcase), pattern)|
        {
            if let Some((reduced, suffix)) = pattern(self, subcase) {
                let new_case = prefix.iter()
                    .cloned()
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
sensor enabled switch1 @enabled
wait 0.1
jump 0 equal enabled true
jump 98 always 0 0
set fname "merge(x,y,z)"
set f2_k 0
jump 97 greaterThanEq x y
jump 97 greaterThanEq y z
jump 19 always 0 0
op sub c f2_j f2_i
op shr c c 1
op add f2_mid c f2_i
read f2_num bank1 f2_mid
jump 16 greaterThan f2_num f2_tgt
op add f2_i f2_mid 1
jump 17 always 0 0
set f2_j f2_mid
jump 9 lessThan f2_i f2_j
op add @counter f2_step 1
read f2_tgt bank1 y
set f2_i x
set f2_j y
set f2_step @counter
jump 17 always 0 0
set x f2_i
op sub f2_c y 1
read f2_tgt bank1 f2_c
set f2_i y
set f2_j z
set f2_step @counter
jump 17 always 0 0
set z f2_i
jump 97 greaterThanEq x y
jump 97 greaterThanEq y z
op sub f2_llen y x
op sub f2_rlen z y
jump 67 lessThan f2_llen f2_rlen
set f2_i y
read num bank1 f2_i
write num bank2 f2_k
op add f2_i f2_i 1
op add f2_k f2_k 1
jump 38 lessThan f2_i z
op sub f2_i f2_k 1
op sub f2_j y 1
set f2_k z
read num_1 bank2 f2_i
read num_2 bank1 f2_j
jump 57 always 0 0
jump 54 greaterThanEq num_1 num_2
write num_2 bank1 f2_k
op sub f2_j f2_j 1
read num_2 bank1 f2_j
jump 57 always 0 0
write num_1 bank1 f2_k
op sub f2_i f2_i 1
read num_1 bank2 f2_i
op sub f2_k f2_k 1
jump 60 lessThan f2_j x
jump 49 greaterThanEq f2_i 0
jump 65 always 0 0
read num_1 bank2 f2_i
write num_1 bank1 f2_k
op sub f2_i f2_i 1
op sub f2_k f2_k 1
jump 61 greaterThanEq f2_i 0
jump 97 always 0 0
set f2_i x
read num bank1 f2_i
write num bank2 f2_k
op add f2_i f2_i 1
op add f2_k f2_k 1
jump 68 lessThan f2_i y
set f2_iend f2_k
set f2_i 0
set f2_j y
set f2_k x
read num_1 bank2 f2_i
read num_2 bank1 f2_j
jump 89 always 0 0
jump 85 lessThanEq num_1 num_2
write num_2 bank1 f2_k
op add f2_j f2_j 1
read num_2 bank1 f2_j
jump 88 always 0 0
write num_1 bank1 f2_k
op add f2_i f2_i 1
read num_1 bank2 f2_i
op add f2_k f2_k 1
jump 91 greaterThanEq f2_j z
jump 80 lessThan f2_i f2_iend
jump 96 always 0 0
read num_1 bank2 f2_i
write num_1 bank1 f2_k
op add f2_i f2_i 1
op add f2_k f2_k 1
jump 92 lessThan f2_i f2_iend
op add @counter step 1
read length cell1 0
op sub p_length length 1
op shr c length 1
op add c c 2
set stack_floor c
op add stack_add_2 stack_floor 2
set stack_t bank2
set stack stack_floor
set min_run length
set r 0
jump 112 always 0 0
op and c min_run 1
op or r r c
op shr min_run min_run 1
jump 109 greaterThanEq min_run 16
op add min_run min_run r
set i 0
set start 0
jump 215 lessThan length 2
op add j i 1
op add run_stop i min_run
op min run_stop run_stop length
read old_num bank1 i
read next_num bank1 j
set start i
jump 135 greaterThan old_num next_num
jump 129 always 0 0
jump 130 greaterThan old_num next_num
op add j j 1
set old_num next_num
read next_num bank1 j
jump 125 lessThan j length
jump 146 always 0 0
jump 136 lessThanEq old_num next_num
op add j j 1
set old_num next_num
read next_num bank1 j
jump 131 lessThan j length
set l start
op sub r j 1
jump 145 always 0 0
read lnum bank1 l
read rnum bank1 r
write rnum bank1 l
write lnum bank1 r
op add l l 1
op sub r r 1
jump 139 lessThan l r
op add run_stop start min_run
op min run_stop run_stop length
set i j
set note "insert_sort"
jump 153 lessThan j run_stop
set run_stop j
jump 168 always 0 0
set f0_i j
set i run_stop
jump 167 always 0 0
read num bank1 f0_i
set f0_j f0_i
jump 162 always 0 0
read num_1 bank1 f0_j
jump 165 lessThanEq num_1 num
write num_1 bank1 c
set c f0_j
op sub f0_j f0_j 1
jump 159 greaterThanEq f0_j start
write num bank1 c
op add f0_i f0_i 1
jump 156 lessThan f0_i run_stop
jump 200 always 0 0
op sub c stack 1
op sub c1 stack 2
read X_s stack_t c
read Y_s stack_t c1
jump 179 equal stack stack_add_2
op sub c2 stack 3
read Z_s stack_t c2
op sub Z_len Y_s Z_s
op sub XY_add start Y_s
jump 183 lessThanEq Z_len XY_add
op sub X_len start X_s
op sub Y_len X_s Y_s
jump 193 lessThanEq Y_len X_len
jump 201 always 0 0
jump 193 greaterThan Z_len X_len
set note "merge to Z"
set x Z_s
set y Y_s
set z X_s
set step @counter
jump 5 always 0 0
write X_s stack_t c1
op sub stack stack 1
jump 201 always 0 0
set note "merge to X"
set x Y_s
set y X_s
set z start
set step @counter
jump 5 always 0 0
op sub stack stack 1
jump 169 greaterThanEq stack stack_add_2
write start stack_t stack
op add stack stack 1
jump 117 lessThan i length
jump 214 always 0 0
op sub stack stack 1
op sub c stack 1
read X_s stack_t stack
read Y_s stack_t c
set x Y_s
set y X_s
set z length
set step @counter
jump 5 always 0 0
jump 205 greaterThanEq stack stack_add_2
control enabled switch1 true 0 0 0

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
