mod var;

use lazy_regex::{regex, Lazy, Regex};
use std::{
    collections::HashSet,
    num::IntErrorKind,
    thread_local,
};

pub use var::Var;

/// 判断是否是一个标识符(包括数字)
pub fn is_ident(s: &str) -> bool {
    static REGEX: &Lazy<Regex> = regex!(
        r#"^(?:(?:[_\p{XID_Start}]\p{XID_Continue}*)|(?:@[_\p{XID_Start}][\p{XID_Continue}\-]*)|(?:0(?:x-?[\da-fA-F][_\da-fA-F]*|b-?[01][_01]*)|-?\d[_\d]*(?:\.\d[\d_]*|e[+\-]?\d[\d_]*)?))$"#
    );
    REGEX.is_match(s)
}

pub const VAR_KEYWORDS: &[&str] = {&[
    "_", "abs", "acos", "add", "always", "and", "angle",
    "asin", "atan", "break", "case", "ceil", "const", "continue",
    "cos", "div", "do", "elif", "else", "equal", "floor",
    "goto", "greaterThan", "greaterThanEq", "gwhile", "idiv", "if", "inline",
    "land", "len", "lessThan", "lessThanEq", "lnot", "log", "match", "max",
    "min", "mod", "mul", "noise", "noop", "not", "notEqual",
    "op", "or", "pow", "print", "rand", "select", "setres",
    "shl", "shr", "sin", "skip", "sqrt", "strictEqual",
    "strictNotEqual", "sub", "switch", "take", "tan", "while", "xor",
]};

/// 判断是否是一个标识符(包括数字)关键字
pub fn is_ident_keyword(s: &str) -> bool {
    thread_local! {
        static VAR_KEYWORDS_SET: HashSet<&'static str>
            = HashSet::from_iter(VAR_KEYWORDS.iter().copied());
    }
    VAR_KEYWORDS_SET.with(|var_keywords| {
        var_keywords.get(s).is_some()
    })
}

#[derive(Debug, PartialEq, Clone)]
pub enum VarType<'a> {
    Var(&'a str),
    String(&'a str),
    Number(f64),
}

impl<'a> VarType<'a> {
    /// Returns `true` if the var type is [`Number`].
    ///
    /// [`Number`]: VarType::Number
    #[must_use]
    pub fn is_number(&self) -> bool {
        matches!(self, Self::Number(..))
    }

    pub fn as_number(&self) -> Option<&f64> {
        if let Self::Number(v) = self {
            Some(v)
        } else {
            None
        }
    }

    /// Returns `true` if the var type is [`String`].
    ///
    /// [`String`]: VarType::String
    #[must_use]
    pub fn is_string(&self) -> bool {
        matches!(self, Self::String(..))
    }

    pub fn as_string(&self) -> Option<&&'a str> {
        if let Self::String(v) = self {
            Some(v)
        } else {
            None
        }
    }

    /// Returns `true` if the var type is [`Var`].
    ///
    /// [`Var`]: VarType::Var
    #[must_use]
    pub fn is_var(&self) -> bool {
        matches!(self, Self::Var(..))
    }

    pub fn as_var(&self) -> Option<&&'a str> {
        if let Self::Var(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

pub trait AsVarType {
    fn as_var_type(&self) -> VarType<'_>;
}
impl AsVarType for str {
    fn as_var_type(&self) -> VarType<'_> {
        fn as_string(s: &str) -> Option<&str> {
            if s.len() < 2 { return None; }
            if !(s.starts_with('"') && s.ends_with('"')) { return None; }
            Some(&s[1..s.len()-1])
        }
        fn as_number(s: &str) -> Option<f64> {
            static NUM_REGEX: &Lazy<Regex> = regex!(
                r"^-?(?:\d+(?:e[+\-]?\d+|\.\d*)?|\d*\.\d+)$"
            );
            static HEX_REGEX: &Lazy<Regex> = regex!(
                r"^0x-?[0-9A-Fa-f]+$"
            );
            static BIN_REGEX: &Lazy<Regex> = regex!(
                r"^0b-?[01]+$"
            );
            fn parse_radix(src: &str, radix: u32) -> Option<f64> {
                match i64::from_str_radix(src, radix) {
                    Ok(x) => Some(x as f64),
                    Err(e) => {
                        match e.kind() {
                            | IntErrorKind::PosOverflow
                            | IntErrorKind::NegOverflow
                            => None,
                            _ => unreachable!("parse hex err: {e}, ({src:?})"),
                        }
                    },
                }
            }
            if NUM_REGEX.is_match(s) {
                let res = match s.parse() {
                    Ok(n) => n,
                    Err(e) => panic!("{}, ({:?})", e, s),
                };
                Some(res)
            } else if HEX_REGEX.is_match(s) {
                parse_radix(&s[2..], 16)
            } else if BIN_REGEX.is_match(s) {
                parse_radix(&s[2..], 2)
            } else {
                None
            }
        }
        match self {
            "null"  => return VarType::Number(f64::NAN.into()),
            "true"  => return VarType::Number(1.0.into()),
            "false" => return VarType::Number(0.0.into()),
            _ => (),
        }
        if let Some(str) = as_string(self) {
            return VarType::String(str);
        }
        if let Some(num) = as_number(self) {
            if num <= i64::MAX as f64 && num >= (i64::MIN + 1) as f64 {
                return VarType::Number(num);
            }
        }
        VarType::Var(self)
    }
}

/// 处理字符串中的转义字符
///
/// 例如 `"\r"` -> `""`
/// 例如 `"a\\\\b"` -> `"a\\b"`
/// 例如换行 `"\n"`|`"\r\n"` -> `"\\n"`
/// 例如 `"a\\\n    \ b"` -> `"a b"`
pub fn string_escape(s: &str) -> String {
    let mut iter = s.chars()
        .filter(|&ch| ch != '\r')
        .peekable();
    let mut res = String::with_capacity(s.len() + (s.len() >> 4));
    while let Some(ch) = iter.next() {
        match ch {
            '\n' => res.push_str("\\n"),
            '\\' if iter.peek() == Some(&' ') => {
                res.push(iter.next().unwrap());
            },
            '\\' if iter.peek() == Some(&'\\') => {
                iter.next().unwrap();
                res.push('\\');
                if iter.peek() == Some(&'n') {
                    res.push_str("[]")
                }
            },
            '\\' if iter.peek() == Some(&'[') => {
                iter.next().unwrap();
                res.push_str(r"[[");
            },
            '\\' if iter.peek() == Some(&'\n') => {
                iter.next().unwrap();
                let mut f = ||
                    iter.next_if(|&ch| matches!(ch, ' ' | '\t'))
                    .is_some();
                while f() {}
            },
            ch => res.push(ch),
        }
    }
    res.shrink_to_fit();
    res
}

pub fn string_unescape(s: &str) -> String {
    let mut iter = s.chars().peekable();
    let mut res = String::with_capacity(s.len() + (s.len() >> 4));
    while let Some(ch) = iter.next() {
        match ch {
            // `\[]` -> `\\`, `\[` -> `\\[`
            '\\' if iter.peek() == Some(&'[') => {
                iter.next().unwrap();
                res.push_str(r"\\");
                if iter.peek() == Some(&']') {
                    iter.next().unwrap();
                } else {
                    res.push('[');
                }
            },
            '\\' if iter.peek() == Some(&'n') => res.push('\\'),
            '\\' => res.push_str(r"\\"),
            ch => res.push(ch),
        }
    }
    res.shrink_to_fit();
    res
}

/// 使用一种简单的转义允许双引号的输入
///
/// `\\` => `\`
/// `\'` => `"`
/// `\a` => `\a`
/// `x`  => `x`
pub fn escape_doublequote(s: &str) -> Result<String, &'static str> {
    let mut res = String::with_capacity(s.len());
    let mut chars = s.chars();

    loop {
        let Some(ch) = chars.next() else { break; };
        let escaped = match ch {
            '\\' => match chars.next() {
                Some('\'') => '"',
                Some('\\') => '\\',
                Some(ch) => {
                    res.push('\\');
                    ch
                },
                None => return Err("escaped eof"),
            }
            ch => ch,
        };
        res.push(escaped);
    }

    Ok(res)
}

#[cfg(test)]
mod tests;
