use lazy_regex::{regex,Lazy,Regex};

pub fn is_ident(s: &str) -> bool {
    static REGEX: &Lazy<Regex> = regex!(
        r#"^(?:(?:[_\p{XID_Start}]\p{XID_Continue}*)|(?:@[_\p{XID_Start}][\p{XID_Continue}\-]*)|(?:0(?:x-?[\da-fA-F][_\da-fA-F]*|b-?[01][_01]*)|-?\d[_\d]*(?:\.\d[\d_]*)?))$"#
    );
    REGEX.is_match(s)
}

#[cfg(test)]
mod tests;
