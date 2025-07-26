use std::str::FromStr;

pub const FUN_PREC: u32 = 20;
pub const ATOM_PREC: u32 = 40;

pub trait OpMeta {
    fn prec(&self) -> u32;

    fn comb(&self) -> Comb {
        Comb::Never
    }
}

pub trait Oper: OpMeta {
    fn name(&self) -> &'static str;

    fn punc(&self) -> &'static str;

    fn is_func(&self) -> bool {
        self.punc().is_empty()
    }

    fn expr_display(&self) -> &'static str {
        if self.is_func() {
            self.name()
        } else {
            self.punc()
        }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub enum Comb {
    Left,
    Never,
    Right,
}

impl Default for Comb {
    fn default() -> Self {
        Self::Never
    }
}

impl Comb {
    /// Returns `true` if the comb is [`Left`].
    ///
    /// [`Left`]: Comb::Left
    #[must_use]
    pub fn is_left(self) -> bool {
        matches!(self, Self::Left)
    }

    /// Returns `true` if the comb is [`Never`].
    ///
    /// [`Never`]: Comb::Never
    #[must_use]
    pub fn is_never(self) -> bool {
        matches!(self, Self::Never)
    }

    /// Returns `true` if the comb is [`Right`].
    ///
    /// [`Right`]: Comb::Right
    #[must_use]
    pub fn is_right(self) -> bool {
        matches!(self, Self::Right)
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub enum Cmp {
    Equal,
    NotEqual,
    LessThan,
    LessThanEq,
    GreaterThan,
    GreaterThanEq,
    StrictEqual,
}

impl TryFrom<Bin> for Cmp {
    type Error = Bin;

    fn try_from(value: Bin) -> Result<Self, Self::Error> {
        Ok(match value {
            Bin::Equal => Self::Equal,
            Bin::NotEqual => Self::NotEqual,
            Bin::LessThan => Self::LessThan,
            Bin::LessThanEq => Self::LessThanEq,
            Bin::GreaterThan => Self::GreaterThan,
            Bin::GreaterThanEq => Self::GreaterThanEq,
            Bin::StrictEqual => Self::StrictEqual,
            _ => return Err(value),
        })
    }
}

impl OpMeta for Cmp {
    fn prec(&self) -> u32 {
        Bin::from(*self).prec()
    }

    fn comb(&self) -> Comb {
        Bin::from(*self).comb()
    }
}

impl Oper for Cmp {
    fn name(&self) -> &'static str {
        Bin::from(*self).name()
    }

    fn punc(&self) -> &'static str {
        Bin::from(*self).punc()
    }
}

impl FromStr for Cmp {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "equal" => Ok(Self::Equal),
            "notEqual" => Ok(Self::NotEqual),
            "lessThan" => Ok(Self::LessThan),
            "lessThanEq" => Ok(Self::LessThanEq),
            "greaterThan" => Ok(Self::GreaterThan),
            "greaterThanEq" => Ok(Self::GreaterThanEq),
            "strictEqual" => Ok(Self::StrictEqual),
            _ => Err(s.into()),
        }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub enum Sin {
    Not,
    Abs,
    Sign,
    Log,
    Log10,
    Floor,
    Ceil,
    Round,
    Sqrt,
    Rand,
    Sin,
    Cos,
    Tan,
    Asin,
    Acos,
    Atan,
}

impl FromStr for Sin {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "not" => Ok(Self::Not),
            "abs" => Ok(Self::Abs),
            "sign" => Ok(Self::Sign),
            "log" => Ok(Self::Log),
            "log10" => Ok(Self::Log10),
            "floor" => Ok(Self::Floor),
            "ceil" => Ok(Self::Ceil),
            "round" => Ok(Self::Round),
            "sqrt" => Ok(Self::Sqrt),
            "rand" => Ok(Self::Rand),
            "sin" => Ok(Self::Sin),
            "cos" => Ok(Self::Cos),
            "tan" => Ok(Self::Tan),
            "asin" => Ok(Self::Asin),
            "acos" => Ok(Self::Acos),
            "atan" => Ok(Self::Atan),
            _ => Err(s.into())
        }
    }
}

impl OpMeta for Sin {
    fn prec(&self) -> u32 {
        match self {
            Sin::Not => 13,
            Sin::Abs
            | Sin::Sign
            | Sin::Log
            | Sin::Log10
            | Sin::Floor
            | Sin::Ceil
            | Sin::Round
            | Sin::Sqrt
            | Sin::Rand
            | Sin::Sin
            | Sin::Cos
            | Sin::Tan
            | Sin::Asin
            | Sin::Acos
            | Sin::Atan => FUN_PREC,
        }
    }

    fn comb(&self) -> Comb {
        match self {
            Sin::Not
            | Sin::Abs
            | Sin::Sign
            | Sin::Log
            | Sin::Log10
            | Sin::Floor
            | Sin::Ceil
            | Sin::Round
            | Sin::Sqrt
            | Sin::Rand
            | Sin::Sin
            | Sin::Cos
            | Sin::Tan
            | Sin::Asin
            | Sin::Acos
            | Sin::Atan => Comb::Right,
        }
    }
}
impl Oper for Sin {
    fn name(&self) -> &'static str {
        match self {
            Sin::Not => "not",
            Sin::Abs => "abs",
            Sin::Sign => "sign",
            Sin::Log => "log",
            Sin::Log10 => "log10",
            Sin::Floor => "floor",
            Sin::Ceil => "ceil",
            Sin::Round => "round",
            Sin::Sqrt => "sqrt",
            Sin::Rand => "rand",
            Sin::Sin => "sin",
            Sin::Cos => "cos",
            Sin::Tan => "tan",
            Sin::Asin => "asin",
            Sin::Acos => "acos",
            Sin::Atan => "atan",
        }
    }

    fn punc(&self) -> &'static str {
        match self {
            Sin::Not => "~",
            Sin::Abs
            | Sin::Sign
            | Sin::Log
            | Sin::Log10
            | Sin::Floor
            | Sin::Ceil
            | Sin::Round
            | Sin::Sqrt
            | Sin::Rand
            | Sin::Sin
            | Sin::Cos
            | Sin::Tan
            | Sin::Asin
            | Sin::Acos
            | Sin::Atan => "",
        }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub enum Bin {
    Add,
    Sub,
    Mul,
    Div,
    Idiv,
    Mod,
    Emod,
    Pow,
    Land,
    Equal,
    NotEqual,
    LessThan,
    LessThanEq,
    GreaterThan,
    GreaterThanEq,
    StrictEqual,
    Shl,
    Shr,
    Ushr,
    Or,
    Xor,
    And,
    Max,
    Min,
    Angle,
    AngleDiff,
    Len,
    Noise,
    Logn,
    // extension funclike
    Read,
    Sense,
}

impl From<Cmp> for Bin {
    fn from(value: Cmp) -> Self {
        match value {
            Cmp::Equal => Self::Equal,
            Cmp::NotEqual => Self::NotEqual,
            Cmp::LessThan => Self::LessThan,
            Cmp::LessThanEq => Self::LessThanEq,
            Cmp::GreaterThan => Self::GreaterThan,
            Cmp::GreaterThanEq => Self::GreaterThanEq,
            Cmp::StrictEqual => Self::StrictEqual,
        }
    }
}

impl OpMeta for Bin {
    fn prec(&self) -> u32 {
        match self {
            Bin::Add | Bin::Sub => 11,
            Bin::Mul | Bin::Div | Bin::Idiv | Bin::Mod | Bin::Emod => 12,
            Bin::Pow => 14,
            Bin::Land => 4,
            Bin::Equal | Bin::NotEqual | Bin::StrictEqual => 5,
            Bin::LessThan
            | Bin::LessThanEq
            | Bin::GreaterThan
            | Bin::GreaterThanEq => 6,
            Bin::Shl | Bin::Shr | Bin::Ushr => 10,
            Bin::Or => 7,
            Bin::Xor => 8,
            Bin::And => 9,
            Bin::Max
            | Bin::Min
            | Bin::Angle
            | Bin::AngleDiff
            | Bin::Len
            | Bin::Noise
            | Bin::Logn
            | Bin::Read
            | Bin::Sense => ATOM_PREC,
        }
    }

    fn comb(&self) -> Comb {
        match self {
            Bin::Add => Comb::Left,
            Bin::Sub => Comb::Left,
            Bin::Mul => Comb::Left,
            Bin::Div => Comb::Left,
            Bin::Idiv => Comb::Left,
            Bin::Mod => Comb::Left,
            Bin::Emod => Comb::Left,
            Bin::Pow => Comb::Right,
            Bin::Land => Comb::Left,
            Bin::Equal => Comb::Never,
            Bin::NotEqual => Comb::Never,
            Bin::LessThan => Comb::Never,
            Bin::LessThanEq => Comb::Never,
            Bin::GreaterThan => Comb::Never,
            Bin::GreaterThanEq => Comb::Never,
            Bin::StrictEqual => Comb::Never,
            Bin::Shl => Comb::Left,
            Bin::Shr => Comb::Left,
            Bin::Ushr => Comb::Left,
            Bin::Or => Comb::Left,
            Bin::Xor => Comb::Left,
            Bin::And => Comb::Left,
            Bin::Max => Comb::Never,
            Bin::Min => Comb::Never,
            Bin::Angle => Comb::Never,
            Bin::AngleDiff => Comb::Never,
            Bin::Len => Comb::Never,
            Bin::Noise => Comb::Never,
            Bin::Logn => Comb::Never,
            Bin::Read => Comb::Left,
            Bin::Sense => Comb::Left,
        }
    }
}
impl Oper for Bin {
    fn name(&self) -> &'static str {
        match self {
            Bin::Add => "add",
            Bin::Sub => "sub",
            Bin::Mul => "mul",
            Bin::Div => "div",
            Bin::Idiv => "idiv",
            Bin::Mod => "mod",
            Bin::Emod => "emod",
            Bin::Pow => "pow",
            Bin::Land => "land",
            Bin::Equal => "equal",
            Bin::NotEqual => "notEqual",
            Bin::LessThan => "lessThan",
            Bin::LessThanEq => "lessThanEq",
            Bin::GreaterThan => "greaterThan",
            Bin::GreaterThanEq => "greaterThanEq",
            Bin::StrictEqual => "strictEqual",
            Bin::Shl => "shl",
            Bin::Shr => "shr",
            Bin::Ushr => "ushr",
            Bin::Or => "or",
            Bin::Xor => "xor",
            Bin::And => "and",
            Bin::Max => "max",
            Bin::Min => "min",
            Bin::Angle => "angle",
            Bin::AngleDiff => "angleDiff",
            Bin::Len => "len",
            Bin::Noise => "noise",
            Bin::Logn => "logn",
            Bin::Read => "read",
            Bin::Sense => "sense",
        }
    }

    fn punc(&self) -> &'static str {
        match self {
            Bin::Add => "+",
            Bin::Sub => "-",
            Bin::Mul => "*",
            Bin::Div => "/",
            Bin::Idiv => "//",
            Bin::Mod => "%",
            Bin::Emod => "%%",
            Bin::Pow => "**",
            Bin::Land => "&&",
            Bin::Equal => "==",
            Bin::NotEqual => "!=",
            Bin::LessThan => "<",
            Bin::LessThanEq => "<=",
            Bin::GreaterThan => ">",
            Bin::GreaterThanEq => ">=",
            Bin::StrictEqual => "===",
            Bin::Shl => "<<",
            Bin::Shr => ">>",
            Bin::Ushr => ">>>",
            Bin::Or => "|",
            Bin::Xor => "^",
            Bin::And => "&",
            Bin::Max
            | Bin::Min
            | Bin::Angle
            | Bin::AngleDiff
            | Bin::Len
            | Bin::Noise
            | Bin::Logn
            | Bin::Read
            | Bin::Sense => "",
        }
    }
}

impl FromStr for Bin {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "add" => Ok(Self::Add),
            "sub" => Ok(Self::Sub),
            "mul" => Ok(Self::Mul),
            "div" => Ok(Self::Div),
            "idiv" => Ok(Self::Idiv),
            "mod" => Ok(Self::Mod),
            "emod" => Ok(Self::Emod),
            "pow" => Ok(Self::Pow),
            "land" => Ok(Self::Land),
            "equal" => Ok(Self::Equal),
            "notEqual" => Ok(Self::NotEqual),
            "lessThan" => Ok(Self::LessThan),
            "lessThanEq" => Ok(Self::LessThanEq),
            "greaterThan" => Ok(Self::GreaterThan),
            "greaterThanEq" => Ok(Self::GreaterThanEq),
            "strictEqual" => Ok(Self::StrictEqual),
            "shl" => Ok(Self::Shl),
            "shr" => Ok(Self::Shr),
            "ushr" => Ok(Self::Ushr),
            "or" => Ok(Self::Or),
            "xor" => Ok(Self::Xor),
            "and" => Ok(Self::And),
            "max" => Ok(Self::Max),
            "min" => Ok(Self::Min),
            "angle" => Ok(Self::Angle),
            "angleDiff" => Ok(Self::AngleDiff),
            "len" => Ok(Self::Len),
            "noise" => Ok(Self::Noise),
            "logn" => Ok(Self::Logn),
            _ => Err(s.into()),
        }
    }
}
