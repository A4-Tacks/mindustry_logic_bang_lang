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
        let lines = s.lines()
            .map(str::trim_start)
            .filter(|line| !(line.starts_with('#') || line.is_empty()))
            .enumerate()
            .map(|(lineno, line)| Line::from_line(lineno, line))
            .filter(|line| {
                line.args().len() != 1 || !line.args()[0].ends_with(':')
            })
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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


#[cfg(test)]
mod tests {
    use lints::{ErrorLint, WarningLint};

    use super::*;

    #[test]
    fn do_works() {
        let s = "set x _1";
        let src = Source::from_str(s);
        assert_eq!(src.lint(), vec![
            Lint::new(
                &Var::new(0, 1, "x"),
                WarningLint::NeverUsed,
            ),
            Lint::new(
                &Var::new(0, 2, "_1"),
                WarningLint::UsedRawArgs,
            ),
        ]);
    }

    #[test]
    fn unknown_cmds() {
        let s = "foo _0 A";
        let src = Source::from_str(s);
        assert_eq!(src.lint(), vec![
            Lint::new(
                &Var::new(0, 1, "_0"),
                WarningLint::UsedRawArgs,
            ),
            Lint::new(
                &Var::new(0, 2, "A"),
                WarningLint::SuspectedConstant,
            ),
        ]);
    }

    #[test]
    fn world_processor_test() {
        let s = r#"
            getblock block result 0 0
            setblock block @air 0 0 @derelict 0
            spawn @dagger 10 10 90 @sharded result1
            status false wet unit 10
            weathersense result2 @rain
            weatherset @rain true
            spawnwave 10 10 false
            setrule waveSpacing 10 0 0 100 100
            message announce 3 @wait
            cutscene pan 100 100 0.06 0
            effect warn 0 0 2 %ffaaff
            explosion @crux 0 0 5 50 true true false true
            setrate 10
            fetch unit result3 @sharded 0 @conveyor
            sync var
            getflag result4 "flag"
            setflag "flag" true
            setprop @copper block1 0
            playsound false @sfx-pew 1 1 0 @thisx @thisy true
            setmarker pos 0 0 0 0
            makemarker shape 0 0 0 true
            localeprint "name"
            status fales wet unit 10
        "#;
        let src = Source::from_str(s);
        assert_eq!(src.lint(), vec![
            Lint::new(
                &Var::new(0, 2, "result"),
                WarningLint::NeverUsed,
            ),
            Lint::new(
                &Var::new(2, 6, "result1"),
                WarningLint::NeverUsed,
            ),
            Lint::new(
                &Var::new(4, 1, "result2"),
                WarningLint::NeverUsed,
            ),
            Lint::new(
                &Var::new(13, 2, "result3"),
                WarningLint::NeverUsed,
            ),
            Lint::new(
                &Var::new(15, 1, "result4"),
                WarningLint::NeverUsed,
            ),
            Lint::new(
                &Var::new(22, 1, "fales"),
                ErrorLint::InvalidOper { expected: &["true", "false"] },
            ),
        ]);
    }
}
