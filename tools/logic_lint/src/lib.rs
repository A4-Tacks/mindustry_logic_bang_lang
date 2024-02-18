pub mod lints;

use core::fmt;
use std::{borrow::Cow, ops::Deref};

use crate::lints::{ShowLint, Lint};
use tag_code::mdt_logic_split_unwraped;

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
        self.args().into_iter()
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

    pub fn args(&self) -> &[Var<'_>] {
        self.args.as_ref()
    }
}

#[derive(Debug)]
pub struct Source<'a> {
    lines: Vec<Line<'a>>,
}
impl<'a> Source<'a> {
    pub fn from_str(s: &'a str) -> Self {
        let lines = s.lines().enumerate()
            .map(|(lineno, line)| Line::from_line(lineno, line))
            .collect();

        Self {
            lines,
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
            .map(|line| line.lint(self))
            .flatten()
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
}

#[derive(Debug)]
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

    pub fn value(&self) -> &str {
        self.value
    }

    pub fn arg_idx(&self) -> usize {
        self.arg_idx
    }

    pub fn lineno(&self) -> usize {
        self.lineno
    }
}
