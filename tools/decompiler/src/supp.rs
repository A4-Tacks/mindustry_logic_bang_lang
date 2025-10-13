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

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Cond<'a>(pub CondOp, pub Args<'a>);

impl<'a> fmt::Display for Cond<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}) {}", self.0.punct(), self.1)
    }
}

impl<'a> fmt::LowerHex for Cond<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(rest) = Args::try_from(&self.1[1..]).ok().filter(|_| self.0.has_args()) {
            write!(f, "{} {} {rest}", self.1[0], self.0.punct())
        } else if self.0.has_args() {
            write!(f, "{} {}", self.0.punct(), self.1)
        } else {
            write!(f, "{}", self.0.punct())
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
