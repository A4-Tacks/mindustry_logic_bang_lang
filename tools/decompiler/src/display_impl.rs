use std::{borrow::Cow, fmt};

use to_true::InTrue;

use crate::{Jump, Label};

use super::Reduce;

fn indent(n: usize) -> Cow<'static, str> {
    static INDENT: &str = if let Ok(s) = str::from_utf8(&[b' '; 64]) { s } else { unreachable!() };

    let spaces = n*4;
    if spaces < INDENT.len() {
        INDENT[..spaces].into()
    } else {
        " ".repeat(spaces).into()
    }
}

impl fmt::Display for Reduce<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let plus_i = 1 + f.precision().unwrap_or(0);
        let plus = indent(plus_i);
        let indent = indent(f.precision().unwrap_or(0));
        let mut is_rest = false;

        match self {
            Reduce::Label(Label(label)) => write!(f, "_{label}:"),
            Reduce::Jump(Jump(Label(label), cond)) => write!(f, "jump _{label} {cond};"),
            Reduce::Break(cond) => write!(f, "break {cond};"),
            Reduce::Skip(cond, reduces) => {
                write!(f, "skip {cond} {{")?;
                for reduce in reduces.as_ref() {
                    write!(f, "\n{plus}{reduce:.plus_i$}")?;
                }
                write!(f, "\n{indent}}}")
            },
            Reduce::DoWhile(cond, reduces) => {
                write!(f, "do {{")?;
                for reduce in reduces.as_ref() {
                    write!(f, "\n{plus}{reduce:.plus_i$}")?;
                }
                write!(f, "\n{indent}}} while {cond};")
            },
            Reduce::While(cond, deps, reduces) => {
                write!(f, "while[")?;
                for dep in deps.as_ref() {
                    write!(f, "\n{plus}{dep:.plus_i$}")?;
                }
                if !deps.is_empty() {
                    write!(f, "\n{indent}")?;
                }
                write!(f, "]")?;
                write!(f, " {cond} {{")?;
                for reduce in reduces.as_ref() {
                    write!(f, "\n{plus}{reduce:.plus_i$}")?;
                }
                write!(f, "\n{indent}}}")
            },
            Reduce::IfElse(cond, then_br, else_br) => {
                write!(f, "if {cond} {{")?;
                for reduce in then_br.as_ref() {
                    write!(f, "\n{plus}{reduce:.plus_i$}")?;
                }
                write!(f, "\n{indent}}} else {{")?;
                for reduce in else_br.as_ref() {
                    write!(f, "\n{plus}{reduce:.plus_i$}")?;
                }
                write!(f, "\n{indent}}}")
            },
            Reduce::Product(reduces) => reduces.iter().try_for_each(|it| {
                is_rest.in_true(|| write!(f, "\n{indent}")).transpose()?;
                it.fmt(f)
            }),
            Reduce::Pure(items) => items.iter().try_for_each(|it| {
                is_rest.in_true(|| write!(f, "\n{indent}")).transpose()?;
                it.fmt(f)?;
                write!(f, ";")
            }),
            Reduce::GSwitch(var, cases) => {
                write!(f, "gswitch {var} {{")?;
                for &(i, ref case) in cases.iter() {
                    write!(f, "\n{indent}case {i}:")?;
                    if matches!(case, Self::Product(it) if it.is_empty()) { continue }
                    write!(f, "\n{plus}{case:.plus_i$}")?;
                }
                write!(f, "\n{indent}}}")
            },
        }
    }
}

impl fmt::LowerHex for Reduce<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let plus_i = 1 + f.precision().unwrap_or(0);
        let plus = indent(plus_i);
        let indent = indent(f.precision().unwrap_or(0));
        let mut is_rest = false;

        match self {
            Reduce::Label(Label(label)) => write!(f, ":_{label}"),
            Reduce::Jump(Jump(Label(label), cond)) => write!(f, "goto :_{label} {cond:x};"),
            Reduce::Break(cond) => write!(f, "break {cond:x};"),
            Reduce::Skip(cond, reduces) => {
                write!(f, "skip {cond:x} {{")?;
                for reduce in reduces.as_ref() {
                    write!(f, "\n{plus}{reduce:.plus_i$x}")?;
                }
                if !reduces.is_empty() {
                    write!(f, "\n{indent}")?;
                }
                write!(f, "}}")
            },
            Reduce::DoWhile(cond, reduces) => {
                write!(f, "do {{")?;
                for reduce in reduces.as_ref() {
                    write!(f, "\n{plus}{reduce:.plus_i$x}")?;
                }
                write!(f, "\n{indent}}} while {cond:x};")
            },
            Reduce::While(cond, deps, reduces) => {
                write!(f, "while")?;
                if !deps.is_empty() {
                    write!(f, "({{")?;
                }
                for dep in deps.as_ref() {
                    write!(f, "\n{plus}{dep:.plus_i$x}")?;
                }
                if !deps.is_empty() {
                    write!(f, "\n{indent}}} =>")?;
                }
                write!(f, " {cond:x}")?;
                if !deps.is_empty() {
                    write!(f, ")")?;
                }
                write!(f, " {{")?;
                for reduce in reduces.as_ref() {
                    write!(f, "\n{plus}{reduce:.plus_i$x}")?;
                }
                write!(f, "\n{indent}}}")
            },
            Reduce::IfElse(cond, then_br, else_br) => {
                write!(f, "if {cond:x} {{")?;
                for reduce in then_br.as_ref() {
                    write!(f, "\n{plus}{reduce:.plus_i$x}")?;
                }
                write!(f, "\n{indent}}} else {{")?;
                for reduce in else_br.as_ref() {
                    write!(f, "\n{plus}{reduce:.plus_i$x}")?;
                }
                write!(f, "\n{indent}}}")
            },
            Reduce::Product(reduces) => reduces.iter().try_for_each(|it| {
                is_rest.in_true(|| write!(f, "\n{indent}")).transpose()?;
                it.fmt(f)
            }),
            Reduce::Pure(items) => items.iter().try_for_each(|it| {
                is_rest.in_true(|| write!(f, "\n{indent}")).transpose()?;
                write!(f, "{it};")
            }),
            Reduce::GSwitch(var, cases) => {
                write!(f, "gswitch {var} {{")?;
                for &(i, ref case) in cases.iter() {
                    write!(f, "\n{indent}case {i}:")?;
                    if matches!(case, Self::Product(it) if it.is_empty()) { continue }
                    write!(f, "\n{plus}{case:.plus_i$x}")?;
                }
                write!(f, "\n{indent}}}")
            },
        }
    }
}

pub fn fmt_reduces(reduces: &[Reduce<'_>]) -> String {
    let mut buf = String::new();
    let mut in_rest = false;
    for ele in reduces {
        in_rest.in_true(|| buf.push('\n'));
        fmt::Write::write_fmt(&mut buf, format_args!("{ele}")).unwrap();
    }
    buf
}
