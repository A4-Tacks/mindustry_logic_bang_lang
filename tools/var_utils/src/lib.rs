use lazy_regex::{regex,Lazy,Regex};
use std::{
    collections::HashSet,
    thread_local,
};

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
    "land", "len", "lessThan", "lessThanEq", "lnot", "log", "max",
    "min", "mod", "mul", "noise", "noop", "not", "notEqual",
    "op", "or", "pow", "print", "rand", "select", "set",
    "setres", "shl", "shr", "sin", "skip", "sqrt", "strictEqual",
    "strictNotEqual", "sub", "switch", "take", "tan", "while", "xor",
]};

/// 判断是否是一个标识符(包括数字)关键字
pub fn is_ident_keyword(s: &str) -> bool {
    thread_local! {
        static VAR_KEYWORDS_SET: HashSet<&'static str>
            = HashSet::from_iter(VAR_KEYWORDS.into_iter().copied());
    }
    VAR_KEYWORDS_SET.with(|var_keywords| {
        var_keywords.get(s).is_some()
    })
}


#[cfg(test)]
mod tests;
