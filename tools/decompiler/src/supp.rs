use std::fmt;

use tag_code::logic_parser::Args;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum CondOp {
    Equal,
    NotEqual,
    StrictEqual,
    StrictNotEqual,
    LessThan,
    LessThanEq,
    GreaterThan,
    GreaterThanEq,
    Always,
    Never,
    Unknown,
    RevUnknown,
}

impl Default for CondOp {
    fn default() -> Self {
        Self::Always
    }
}

impl CondOp {
    pub(crate) fn apply_not(self) -> Self {
        match self {
            CondOp::Equal => Self::NotEqual,
            CondOp::NotEqual => Self::Equal,
            CondOp::StrictEqual => Self::StrictNotEqual,
            CondOp::StrictNotEqual => Self::StrictEqual,
            CondOp::LessThan => Self::GreaterThanEq,
            CondOp::LessThanEq => Self::GreaterThan,
            CondOp::GreaterThan => Self::LessThanEq,
            CondOp::GreaterThanEq => Self::LessThan,
            CondOp::Always => Self::Never,
            CondOp::Never => Self::Always,
            CondOp::Unknown => Self::RevUnknown,
            CondOp::RevUnknown => Self::Unknown,
        }
    }

    pub(crate) fn with_args(args: Args<'_>) -> (Self, Args<'_>) {
        if args.len() < 2 {
            return (Self::Unknown, args);
        }
        let body = Args::try_from(&args[1..]).unwrap();
        let cond = match args.first() {
            "equal" => CondOp::Equal,
            "notEqual" => CondOp::NotEqual,
            "strictEqual" => CondOp::StrictEqual,
            "lessThan" => CondOp::LessThan,
            "lessThanEq" => CondOp::LessThanEq,
            "greaterThan" => CondOp::GreaterThan,
            "greaterThanEq" => CondOp::GreaterThanEq,
            "always" => CondOp::Always,
            _ => CondOp::Unknown,
        };
        (cond, body.into_owned())
    }

    fn punct(&self) -> &'static str {
        match self {
            CondOp::Equal => "==",
            CondOp::NotEqual => "!=",
            CondOp::StrictEqual => "===",
            CondOp::StrictNotEqual => "!==",
            CondOp::LessThan => "<",
            CondOp::LessThanEq => "<=",
            CondOp::GreaterThan => ">",
            CondOp::GreaterThanEq => ">=",
            CondOp::Always => "_",
            CondOp::Never => "!_",
            CondOp::Unknown => "?",
            CondOp::RevUnknown => "!?",
        }
    }

    fn has_args(&self) -> bool {
        match self {
            CondOp::Always => false,
            CondOp::Never => false,
            CondOp::Unknown => false,
            CondOp::RevUnknown => false,
            _ => true,
        }
    }
}

pub(crate) fn sfind<T, F>(slice: &[T], predicate: F) -> Option<(&[T], &T, &[T])>
where F: FnMut(&T) -> bool,
{
    let pos = slice.iter().position(predicate)?;
    Some((&slice[..pos], &slice[pos], &slice[pos+1..]))
}

pub(crate) fn split_at<T, F>(slice: &[T], predicate: F) -> Option<(&[T], &[T])>
where F: FnMut(&T) -> bool,
{
    let index = slice.iter().position(predicate)?;
    Some((&slice[..index], &slice[index..]))
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum Cmp<'a> {
    Cond(CondOp, Args<'a>),
    And(Box<Cmp<'a>>, Box<Cmp<'a>>),
    Or(Box<Cmp<'a>>, Box<Cmp<'a>>),
}
pub use Cmp::Cond;

impl<'a> Cmp<'a> {
    pub fn is_always(&self) -> bool {
        match self {
            Cond(cond, _) => *cond == CondOp::Always,
            Cmp::And(a, b) => a.is_always() && b.is_always(),
            Cmp::Or(a, b) => a.is_always() || b.is_always(),
        }
    }
}

impl<'a> fmt::Display for Cmp<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Cond(cond_op, args) => write!(f, "({}) {args}", cond_op.punct()),
            Cmp::And(cmp, cmp1) => write!(f, "({cmp} && {cmp1})"),
            Cmp::Or(cmp, cmp1) => write!(f, "({cmp} || {cmp1})"),
        }
    }
}

impl<'a> fmt::LowerHex for Cmp<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fn out(f: &mut fmt::Formatter<'_>, this: &Cmp<'_>, needs_paren: bool) -> fmt::Result {
            if needs_paren {
                write!(f, "({this:x})")
            } else {
                write!(f, "{this:x}")
            }
        }
        match self {
            Cond(cond_op, args) => {
                if let Some(rest) = Args::try_from(&args[1..])
                    .ok()
                    .filter(|_| cond_op.has_args())
                {
                    write!(f, "{} {} {rest}", args[0], cond_op.punct())
                } else if cond_op.has_args() {
                    write!(f, "{} {args}", cond_op.punct())
                } else {
                    write!(f, "{}", cond_op.punct())
                }
            }
            Cmp::Or(a, b) => {
                out(f, a, false)?;
                write!(f, " || ")?;
                out(f, b, false)?;
                Ok(())
            },
            Cmp::And(a, b) => {
                out(f, a, a.is_or())?;
                write!(f, " && ")?;
                out(f, b, b.is_or())?;
                Ok(())
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sfind() {
        assert_eq!(sfind(&[0, 1, 2, 3, 4], |n| *n == 2), Some((&[0, 1][..], &2, &[3, 4][..])));
    }
}
