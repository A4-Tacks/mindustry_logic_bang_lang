pub mod lints;

use core::fmt;
use std::{borrow::Cow, collections::HashSet, ops::Deref};

use lints::get_useds;
use tag_code::mdt_logic_split_unwraped;

use crate::lints::{Lint, ShowLint};

const LIGHT_ARGS_BEGIN: &str = "\x1b[7m";
const LIGHT_ARGS_END: &str = "\x1b[27m";

#[derive(Debug)]
pub struct Line<'a> {
    args: Vec<Var<'a>>,
}
impl<'a> Line<'a> {
    pub fn from_line(lineno: usize, s: &'a str) -> Self {
        let logic_args = mdt_logic_split_unwraped(s);
        assert_ne!(
            logic_args.len(), 0,
            "line args count by zero ({})", lineno);
        let args = logic_args
            .into_iter()
            .enumerate()
            .map(|(i, arg)| Var::new(lineno, i, arg))
            .collect();

        Self { args }
    }

    pub fn hint_args(&self, hints: &[usize]) -> Vec<Cow<'_, str>> {
        self.args().iter()
            .enumerate()
            .map(|(i, arg)| if hints.contains(&i) {
                format!(
                    "{}{}{}",
                    LIGHT_ARGS_BEGIN,
                    arg.value(),
                    LIGHT_ARGS_END,
                ).into()
            } else {
                arg.value().into()
            })
            .collect()
    }

    pub fn lint(&'a self, src: &'a Source<'_>) -> Vec<lints::Lint> {
        lints::lint(src, self)
    }

    pub fn lineno(&self) -> usize {
        self.args.first().unwrap().lineno
    }

    pub fn args(&self) -> &[Var<'a>] {
        self.args.as_ref()
    }

    pub fn len(&self) -> usize {
        self.args().len()
    }

    pub fn is_empty(&self) -> bool {
        self.args().is_empty()
    }
}

#[derive(Debug)]
pub struct Source<'a> {
    lines: Vec<Line<'a>>,
    /// 行中只读的量表,
    /// 如果一行同时对一个量进行了读写,
    /// 那么它将不会出现在这个表中
    readonly_used_vars: HashSet<&'a str>,
}
impl<'a> Source<'a> {
    pub fn from_str(s: &'a str) -> Self {
        let env_assignables = &[
            "@counter",
        ][..];
        let lines = s.lines().enumerate()
            .map(|(lineno, line)| Line::from_line(lineno, line))
            .collect::<Vec<_>>();

        let readonly_used_vars = lines.iter()
            .filter_map(get_useds)
            .flat_map(|args| {
                args.clone()
                    .into_iter()
                    .filter(move |arg| {
                        if arg.method().is_assign() {
                            return false;
                        }
                        !args.iter().any(|x| {
                            x.method().is_assign()
                                && x.var().value() == arg.var().value()
                        })
                    })
            })
            .filter_map(|used| used.as_read()
                .map(Var::value))
            .chain(env_assignables.iter().copied())
            .collect();

        Self {
            lines,
            readonly_used_vars,
        }
    }

    /// 返回指定行周围的行, 而不包括指定行
    pub fn view_lines(
        &self,
        lineno: usize,
        rng: (usize, usize),
    ) -> (&[Line<'_>], &[Line<'_>]) {
        let (head, tail) = (
            &self.lines[..lineno],
            &self.lines[lineno+1..],
        );
        let head = &head[
            (head.len().checked_sub(rng.0).unwrap_or_default())..];
        let tail = &tail[..rng.1.min(tail.len())];
        (head, tail)
    }

    pub fn lint(&self) -> Vec<lints::Lint> {
        self.lines.iter()
            .flat_map(|line| line.lint(self))
            .collect()
    }

    pub fn lines(&self) -> &[Line<'_>] {
        self.lines.as_ref()
    }

    pub fn show_lints(&self) {
        struct LintFmtter<'a>(&'a Source<'a>, &'a Lint<'a>);
        impl fmt::Display for LintFmtter<'_> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.1.show_lint(self.0, f)
            }
        }
        for lint in self.lint() {
            let fmtter = LintFmtter(self, &lint);
            eprintln!("{}", fmtter)
        }
    }

    pub fn used_vars(&self) -> &HashSet<&'a str> {
        &self.readonly_used_vars
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Var<'a> {
    lineno: usize,
    arg_idx: usize,
    value: &'a str,
}
impl<'a> Deref for Var<'a> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.value()
    }
}

impl<'a> Var<'a> {
    pub fn new(lineno: usize, arg_idx: usize, value: &'a str) -> Self {
        Self { lineno, arg_idx, value }
    }

    pub fn new_nonlocation(value: &'a str) -> Self {
        Self { lineno: usize::MAX, arg_idx: 0, value }
    }

    pub fn value(&self) -> &'a str {
        self.value
    }

    pub fn arg_idx(&self) -> usize {
        self.arg_idx
    }

    pub fn lineno(&self) -> usize {
        self.lineno
    }
}
