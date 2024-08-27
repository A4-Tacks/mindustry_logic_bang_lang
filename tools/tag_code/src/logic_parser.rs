use std::{
    borrow::Cow, collections::HashSet, fmt::Display, iter::{self, once}, mem, ops::{Deref, DerefMut}, slice, vec
};
use either::Either::{self, Left, Right};

pub use var_utils::Var;

peg::parser!(pub grammar parser() for str {
    rule _() = quiet!{ [' ' | '\t']+ } / expected!("whitespace")
    rule newline_raw() = quiet!{ "\r"? "\n" } / expected!("newline")
    rule comment() = quiet!{ "#" [^'\r' | '\n']* } / expected!("comment")
    pub(crate) rule nl()
        = ![_]
        / _? (
            ";"
            / newline_raw()
            / comment()
        ) nl()? _?

    rule string() -> &'input str
        = quiet!{ $("\"" [^'\r' | '\n' | '"']* "\"") }
        / expected!("string")

    rule norm_arg_ch() = [^' ' | '\t' | '\r' | '\n' | '"' | '#' | ';']
    rule norm_arg_inner()
        = norm_arg_ch() norm_arg_inner()
        / !":" norm_arg_ch()
    rule norm_arg() -> &'input str
        = quiet!{ $(norm_arg_inner()) }
        / expected!("norm-arg")

    pub rule arg() -> &'input str
        = norm_arg()
        / string()

    pub rule args() -> Args<'input>
        = args:(arg:arg() { Var::from(arg) }) ++ _ { args.try_into().unwrap() }

    pub rule label() -> &'input str
        = quiet!{ s:arg() ":" { s } / $(":" arg()) }
        / expected!("label")

    pub rule line() -> ParseLine<'input>
        = l:label()     { ParseLine::Label(l.into()) }
        / "jump" _ !"-1" target:arg() _ args:args()
                        { ParseLine::Jump(target.into(), args) }
        / args:args()   { args.into() }

    pub rule lines() -> ParseLines<'input>
        = nl()? lines:(
                pos:position!()
                l:line() nl() { (pos, l).into() }
            )* { lines.into() }
});

/// # Examples
/// ```
/// # use tag_code::{args, logic_parser::Args};
/// # use std::borrow::Cow;
/// assert_eq!(
///     args!("a", "b"),
///     Args::try_from(
///         Cow::Owned(vec!["a".into(), "b".into()])
///     ).unwrap(),
/// );
/// ```
#[macro_export]
macro_rules! args {
    ($($arg:expr),+ $(,)?) => {
        $crate::logic_parser::Args::try_from(::std::vec![
            $(
                $crate::logic_parser::Var::from($arg),
            )+
        ]).unwrap()
    };
}

macro_rules! i {
    (*$i:ident++) => {{
        let __res = *$i;
        *$i += 1;
        __res
    }};
}

/// `Vec<Cow<str>>` wrapper, but not empty
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Args<'a>(Cow<'a, [Var]>);
impl<'a> Args<'a> {
    pub fn first(&self) -> &str {
        self.0.first().unwrap()
    }

    pub fn into_owned(self) -> Args<'static> {
        Args(self.0.into_owned().into())
    }
}
impl<'a> IntoIterator for Args<'a> {
    type Item = Var;
    type IntoIter = Either<
        iter::Cloned<slice::Iter<'a, Var>>,
        vec::IntoIter<Var>
    >;

    fn into_iter(self) -> Self::IntoIter {
        match self.0 {
            Cow::Borrowed(slice) => {
                Left(slice.iter().cloned())
            },
            Cow::Owned(vec) => {
                Right(vec.into_iter())
            },
        }
    }
}
impl<'a> Deref for Args<'a> {
    type Target = [Var];

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}
impl<'a> From<Args<'a>> for Vec<Var> {
    fn from(value: Args<'a>) -> Self {
        value.0.into_owned()
    }
}
impl<'a> TryFrom<Vec<Var>> for Args<'a> {
    type Error = ();

    fn try_from(value: Vec<Var>) -> Result<Self, Self::Error> {
        if value.is_empty() {
            return Err(());
        }
        Ok(Self(value.into()))
    }
}
impl<'a> TryFrom<Vec<&'a str>> for Args<'a> {
    type Error = ();

    fn try_from(value: Vec<&'a str>) -> Result<Self, Self::Error> {
        if value.is_empty() {
            return Err(());
        }
        let args = value.into_iter()
            .map(Into::into)
            .collect();
        Ok(Self(Cow::Owned(args)))
    }
}
impl<'a> TryFrom<Cow<'a, [Var]>> for Args<'a> {
    type Error = ();

    fn try_from(value: Cow<'a, [Var]>) -> Result<Self, Self::Error> {
        if value.is_empty() {
            return Err(());
        }
        Ok(Self(value))
    }
}
impl TryFrom<Vec<String>> for Args<'static> {
    type Error = ();

    fn try_from(value: Vec<String>) -> Result<Self, Self::Error> {
        if value.is_empty() {
            return Err(());
        }
        let args = value.into_iter()
            .map(Into::into)
            .collect();
        Ok(Self(Cow::Owned(args)))
    }
}
impl<'a> Display for Args<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self(args) = self;

        f.write_str(args.first().unwrap())?;
        for arg in args.iter().skip(1) {
            f.write_str(" ")?;
            f.write_str(&arg)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ParseLine<'a> {
    Label(Cow<'a, str>),
    Jump(Cow<'a, str>, Args<'a>),
    Args(Args<'a>),
}
impl<'a> ParseLine<'a> {
    /// new always jump
    pub fn new_always(target: Cow<'a, str>) -> Self {
        Self::Jump(target, args!("always", "0", "0"))
    }

    pub fn into_owned(self) -> ParseLine<'static> {
        match self {
            ParseLine::Label(lab) => {
                ParseLine::Label(lab.into_owned().into())
            },
            ParseLine::Jump(tgt, args) => {
                ParseLine::Jump(tgt.into_owned().into(), args.into_owned())
            },
            ParseLine::Args(args) => {
                args.into_owned().into()
            },
        }
    }

    pub fn is_solid(&self) -> bool {
        matches!(self, Self::Args(_) | Self::Jump(_, _))
    }

    /// Returns `true` if the parse line is [`Args`].
    ///
    /// [`Args`]: ParseLine::Args
    #[must_use]
    pub fn is_args(&self) -> bool {
        matches!(self, Self::Args(..))
    }

    pub fn as_args(&self) -> Option<&Args<'a>> {
        if let Self::Args(v) = self {
            Some(v)
        } else {
            None
        }
    }

    /// Returns `true` if the parse line is [`Label`].
    ///
    /// [`Label`]: ParseLine::Label
    #[must_use]
    pub fn is_label(&self) -> bool {
        matches!(self, Self::Label(..))
    }

    pub fn as_label(&self) -> Option<&str> {
        if let Self::Label(v) = self {
            Some(v)
        } else {
            None
        }
    }

    /// Returns `true` if the parse line is [`Jump`].
    ///
    /// [`Jump`]: ParseLine::Jump
    #[must_use]
    pub fn is_jump(&self) -> bool {
        matches!(self, Self::Jump(..))
    }

    pub fn as_jump_target(&self) -> Option<&str> {
        if let Self::Jump(target, _) = self {
            Some(target)
        } else {
            None
        }
    }

    pub fn as_jump_idx(&self) -> Option<usize> {
        self.as_jump_target()
            .map(str::parse)?
            .ok()
    }

    pub fn as_jump_args(&self) -> Option<&Args<'a>> {
        if let Self::Jump(_, args) = self {
            Some(args)
        } else {
            None
        }
    }
}
impl<'a> TryFrom<Vec<Var>> for ParseLine<'a> {
    type Error = <Args<'a> as TryFrom<Vec<Var>>>::Error;

    fn try_from(value: Vec<Var>) -> Result<Self, Self::Error> {
        Ok(Self::Args(value.try_into()?))
    }
}
impl<'a> From<&'a str> for ParseLine<'a> {
    fn from(v: &'a str) -> Self {
        Self::Label(v.into())
    }
}
impl<'a> From<String> for ParseLine<'a> {
    fn from(v: String) -> Self {
        Self::Label(v.into())
    }
}
impl<'a> From<Cow<'a, str>> for ParseLine<'a> {
    fn from(v: Cow<'a, str>) -> Self {
        Self::Label(v)
    }
}
impl<'a> From<Args<'a>> for ParseLine<'a> {
    fn from(v: Args<'a>) -> Self {
        Self::Args(v)
    }
}
impl<'a> Display for ParseLine<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseLine::Label(lab) => {
                if let Some('0'..='9') = lab.chars().next() {
                    f.write_str(":")?;
                }
                f.write_str(&*lab)?;
                f.write_str(":")
            },
            ParseLine::Jump(target, args) => {
                if f.alternate() { f.write_str("    ")? }
                f.write_str("jump ")?;
                if let Some('0'..='9') = target.chars().next() {
                    f.write_str(":")?;
                }
                f.write_fmt(format_args!("{target} {args}"))
            },
            ParseLine::Args(args) => {
                if f.alternate() { f.write_str("    ")? }
                args.fmt(f)
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct IdxBox<T> {
    pub index: usize,
    pub value: T,
}
impl<T: PartialEq> PartialEq<T> for IdxBox<T> {
    fn eq(&self, other: &T) -> bool {
        **self == *other
    }
    fn ne(&self, other: &T) -> bool {
        **self != *other
    }
}
impl<T: Display> Display for IdxBox<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.value.fmt(f)
    }
}
impl<T> From<(usize, T)> for IdxBox<T> {
    fn from((index, value): (usize, T)) -> Self {
        Self::new(index, value)
    }
}
impl<T> IdxBox<T> {
    pub fn new(index: usize, value: T) -> Self {
        Self { index, value }
    }

    /// Get line and column
    ///
    /// - `index >= s.len()` return last char next char location
    ///
    /// start line and column is 1
    ///
    /// newline char is LF (`\n` 0x0A), and next line after the newline char,
    /// e.g `("a\n", 1)` location is `(1, 2)`
    pub fn location(&self, s: &str) -> (u32, u32) {
        if self.index >= s.len() {
            let nl = s.chars().filter(|&ch| ch == '\n').count();
            let ch = s.chars()
                .rev()
                .take_while(|&ch| ch != '\n')
                .count();
            return ((nl + 1).try_into().unwrap(), (ch + 1).try_into().unwrap());
        }
        let mut cur = 0;
        let mut lnum = 1;
        for line in s.split_inclusive('\n') {
            if cur + line.len() > self.index {
                let lidx = self.index - cur;
                let col = line
                    .char_indices()
                    .filter(|&(i, _)| i <= lidx)
                    .count();
                return (lnum, col.try_into().unwrap());
            }
            cur += line.len();
            lnum += 1;
        }
        unreachable!()
    }

    pub fn as_ref(&self) -> IdxBox<&T> {
        let Self { index, value } = self;
        IdxBox::new(*index, value)
    }

    pub fn as_mut(&mut self) -> IdxBox<&mut T> {
        let Self { index, value } = self;
        IdxBox::new(*index, value)
    }

    pub fn map<U, F>(self, f: F) -> IdxBox<U>
    where F: FnOnce(T) -> U,
    {
        let Self { index, value } = self;
        IdxBox { index, value: f(value) }
    }

    pub fn and_then<U, F>(self, f: F) -> Option<IdxBox<U>>
    where F: FnOnce(T) -> Option<U>,
    {
        let Self { index, value } = self;
        Some(IdxBox { index, value: f(value)? })
    }
}
impl<T> Deref for IdxBox<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}
impl<T> DerefMut for IdxBox<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct ParseLines<'a> {
    lines: Vec<IdxBox<ParseLine<'a>>>,
}
impl<'a> Display for ParseLines<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut iter = self.iter();
        if let Some(line) = iter.next() {
            line.fmt(f)?;
        }
        for line in iter {
            writeln!(f)?;
            line.fmt(f)?;
        }
        Ok(())
    }
}
impl<'a> FromIterator<IdxBox<ParseLine<'a>>> for ParseLines<'a> {
    fn from_iter<T: IntoIterator<Item = IdxBox<ParseLine<'a>>>>(iter: T) -> Self {
        Self::new(iter.into_iter().collect())
    }
}
impl<'a> IntoIterator for ParseLines<'a> {
    type Item = IdxBox<ParseLine<'a>>;
    type IntoIter = <Vec<Self::Item> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.lines.into_iter()
    }
}
impl<'a> ParseLines<'a> {
    pub fn new(lines: Vec<IdxBox<ParseLine<'a>>>) -> Self {
        Self { lines }
    }

    pub fn solid_count(&self) -> usize {
        self.lines
            .iter()
            .filter(|line| line.is_solid())
            .count()
    }

    pub fn modif_count(&self) -> usize {
        self.len() - self.solid_count()
    }

    pub fn into_owned(self) -> ParseLines<'static> {
        self.into_iter()
            .map(|line| line
                .map(|line| line.into_owned()))
            .collect()
    }

    pub fn lines(&self) -> &[IdxBox<ParseLine<'a>>] {
        &self.lines
    }

    pub fn lines_mut(&mut self) -> &mut Vec<IdxBox<ParseLine<'a>>> {
        &mut self.lines
    }

pub fn index_label_popup(&mut self) {
    let mut lines = mem::take(self.lines_mut());
    let poped: HashSet<usize> = lines.iter()
        .filter_map(|x| x.as_label())
        .filter_map(|x| x.parse().ok())
        .collect();
    let indexs: HashSet<usize> = lines.iter()
        .filter_map(|x| x.as_jump_idx())
        .collect();
    let pop_idxs = &indexs - &poped;
    while Some(true) == lines.last()
        .map(|x| x.is_label()) // move last label to first
    {
        self.lines.push(lines.pop().unwrap());
    }
    self.lines.extend(lines.into_iter()
        .scan(0, |i, line| {
            Some((line.is_solid().then(|| i!(*i++)), line))
        })
        .flat_map(|(i, line)| {
            i.and_then(|i| {
                pop_idxs.contains(&i).then(|| {
                    IdxBox::new(
                        line.index,
                        ParseLine::Label(i.to_string().into()),
                    )
                })
            }).into_iter().chain(once(line))
        })
    );
}
}
impl<'a> From<Vec<IdxBox<ParseLine<'a>>>> for ParseLines<'a> {
    fn from(lines: Vec<IdxBox<ParseLine<'a>>>) -> Self {
        Self::new(lines)
    }
}
impl<'a> From<ParseLines<'a>> for Vec<IdxBox<ParseLine<'a>>> {
    fn from(value: ParseLines<'a>) -> Self {
        value.lines
    }
}
impl<'a> Deref for ParseLines<'a> {
    type Target = Vec<IdxBox<ParseLine<'a>>>;

    fn deref(&self) -> &Self::Target {
        &self.lines
    }
}
impl<'a> DerefMut for ParseLines<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.lines
    }
}

#[cfg(test)]
mod tests {
    use crate::TagCodes;
    use super::*;

    #[test]
    fn newline_test() {
        let datas = [
            "",
            "\n",
            "\r\n",
            "#foo\n",
            "#foo\r\n",
            "# foo\n",
            "# foo\r\n",
            " # foo\n",
            " # foo\r\n",
            "   # foo\n",
            "   # foo\r\n",
            " \t  # foo\n",
            " \t  # foo\r\n",
            " \t  # foo\r\n",
            "\n ",
            "\n  ",
            " \n  ",
            " \r\n  ",
            "  \r\n  ",
            ";",
            "  ;",
            ";  ",
            "  ;  ",
            "  ;  ;",
            "  ;  ;  ",
            "\n\n",
            "  \n\n",
            "\n  \n",
            "\n\n  ",
            "  \n\n  ",
            "\n  \n  ",
            "  \n  \n  ",
            "  #foo\n  \n  ",
            "  #foo\n  #bar\n  ",
            "  #foo\n  #bar\n  #baz",
            " ; #foo\n  #bar\n  #baz\n",
        ];
        assert!(parser::nl(" ; #foo\n  #bar\n  #baz\nm").is_err());

        for src in datas {
            assert_eq!(parser::nl(&src), Ok(()), "src: {src}");
        }
    }

    #[test]
    fn arg_test() {
        let datas = [
            "a",
            "'",
            "a'b",
            "ab",
            "x",
            "A",
            "\"\"",
            "\"x\"",
            "\"foo\"",
            "\"f oo\"",
            "\"#\"",
            "\";\"",
        ];
        let fails = [
            " a",
            "a ",
            "a#",
            "a;",
            "a;",
            "",
            "\"",
            "\"\n\"",
            "\"\"\"",
            ";",
            " ",
            "foo:",
            "o:",
            ":",
        ];

        for src in datas {
            assert_eq!(parser::arg(src), Ok(src));
        }
        for src in fails {
            assert!(parser::arg(src).is_err());
        }
    }

    #[test]
    fn label_test() {
        let datas = [
            ("foo:", "foo"),
            ("x:", "x"),
            (":x", ":x"),
            (":foo", ":foo"),
            (":f:oo", ":f:oo"),
            ("fo:o:", "fo:o"),
            (":a:", ":a"),
        ];
        let fails = [
            ":",
            "::",
            ":::",
        ];

        for (src, dst) in datas {
            assert_eq!(parser::label(src), Ok(dst), "{src}");
        }
        for src in fails {
            assert!(parser::label(src).is_err());
        }
    }

    #[test]
    fn all_parse() {
        let src = r#"
        # find player unit
        set id 0
        loop:
            lookup unit unit_ty id
        restart:
            ubind unit_ty
            jump skip strictEqual @unit null
            set first @unit
            jump do_bind_loop_cond always 0 0
            # 这里会将该种单位全部绑定一遍, 直到找到玩家控制的那个
        do_bind_loop:
            sensor is_dead first @dead
            jump restart notEqual is_dead false # 头单位已经死亡, 我们永远无法到达, 所以我们需要重新开始
        do_bind_loop_cond:
            sensor ctrl_type @unit @controlled
            jump finded equal ctrl_type @ctrlPlayer # 绑定到的是玩家
            ubind unit_ty
            jump do_bind_loop notEqual @unit first
        skip:
            op add id id 1
        jump loop lessThan id @unitCount
        end
        finded:
        set player @unit
        follow_loop:
            sensor x player @x
            sensor y player @y
            sensor name player @name # 如果有, 则可以显示玩家名
            # 取两位小数省的小数部分太长
            op idiv x x 0.01 # 通过整除省去乘再向下取整运算
            op idiv y y 0.01
            op div x x 100
            op div y y 100
            print "player "; print name
            print "[]: "; print unit_ty
            print " -> "; print x; print ", "; print y
            printflush message1
        sensor ctrl_type player @controlled
        jump follow_loop equal ctrl_type @ctrlPlayer # 仅在玩家控制期间进行执行, 玩家解除控制重新寻找
        printflush message1 # clear message
        "#;
        let lines = parser::lines(&src).unwrap();
        for line in lines {
            println!("{line:#}");
        }
    }

    #[test]
    fn line_fmt_test() {
        let datas: [(ParseLine, _); 5] = [
            (args!("read").into(), "read"),
            (args!["read", "foo"].into(), "read foo"),
            (args!["read", "foo", "bar"].into(), "read foo bar"),
            (args!["read", "\"foo\"", "bar"].into(), "read \"foo\" bar"),
            ("label".into(), "label:"),
        ];
        for (src, dst) in datas {
            assert_eq!(src.to_string(), format!("{dst}"));
        }
    }

    #[test]
    fn location_eof_test() {
        let datas = [
            ("", (1, 1)),
            ("a", (1, 2)),
            ("a\n", (2, 1)),
            ("a\nx", (2, 2)),
            ("a\n\n", (3, 1)),
        ];
        for (src, loc) in datas {
            let box_ = IdxBox::new(src.len(), ());
            assert_eq!(box_.location(src), loc);
        }
    }

    #[test]
    fn location_test() {
        let datas = [
            ("", 0, (1, 1)),
            ("\n", 0, (1, 1)),
            ("\n", 1, (2, 1)),
            ("a", 0, (1, 1)),
            ("ab", 0, (1, 1)),
            ("ab", 1, (1, 2)),
            ("a\n", 1, (1, 2)),
            ("a\nb", 1, (1, 2)),
            ("a\nb", 2, (2, 1)),
            ("a\nb", 3, (2, 2)),
            ("a\nbc", 3, (2, 2)),
            ("a\n\nc", 2, (2, 1)),
            ("a\n\nc", 3, (3, 1)),
            ("你", 0, (1, 1)),
            ("你", 1, (1, 1)),
            ("你", 2, (1, 1)),
            ("你", 3, (1, 2)),
            ("你x", 0, (1, 1)),
            ("你x", 1, (1, 1)),
            ("你x", 2, (1, 1)),
            ("你x", 3, (1, 2)),
        ];
        for (src, idx, loc) in datas {
            let box_ = IdxBox::new(idx, ());
            assert_eq!(box_.location(src), loc, "{src:?}[{idx}]");
        }
    }

    #[test]
    fn lines_parse_index_test() {
        let s = "a\nb\n c";
        let lines = parser::lines(s).unwrap();
        assert_eq!(lines[0].index, 0);
        assert_eq!(lines[1].index, 2);
        assert_eq!(lines[2].index, 5);
        assert_eq!(lines[0].location(s), (1, 1));
        assert_eq!(lines[1].location(s), (2, 1));
        assert_eq!(lines[2].location(s), (3, 2));
    }

    #[test]
    fn jump_target_test() {
        let s = r#"
        jump -1 x
        jump :-1 y
        jump 0 z
        jump :0 a
        :-1
        :0
        end
        "#;
        let lines = parser::lines(s).unwrap();
        assert!(lines[0].is_args());
        assert!(lines[1].is_jump());
        assert!(lines[2].is_jump());
        assert!(lines[3].is_jump());
        let mut tagcodes = TagCodes::try_from(lines).unwrap();
        tagcodes.build_tagdown().unwrap();
        assert!(tagcodes.lines[0].is_line());
        assert!(tagcodes.lines[1].is_jump());
        assert!(tagcodes.lines[2].is_jump());
        assert!(tagcodes.lines[3].is_jump());
    }

    #[test]
    fn jump_target_popup_test() {
        let s = r#"
        jump 0 always 0 0
        2:
        jump 1 always 0 0
        jump 2 always 0 0
        end
        jump 1 always 0 0
        "#;

        let mut lines = parser::lines(&s).unwrap();

        let inner_lines = lines.iter()
            .map(|line| line.value.clone())
            .collect::<Vec<_>>();
        assert_eq!(inner_lines, vec![
            ParseLine::Jump("0".into(), args!("always", "0", "0")),
            ParseLine::Label("2".into()),
            ParseLine::Jump("1".into(), args!("always", "0", "0")),
            ParseLine::Jump("2".into(), args!("always", "0", "0")),
            ParseLine::Args(args!("end")),
            ParseLine::Jump("1".into(), args!("always", "0", "0")),
        ]);

        lines.index_label_popup();

        let inner_lines = lines.iter()
            .map(|line| line.value.clone())
            .collect::<Vec<_>>();
        assert_eq!(inner_lines, vec![
            ParseLine::Label("0".into()),
            ParseLine::Jump("0".into(), args!("always", "0", "0")),
            ParseLine::Label("2".into()),
            ParseLine::Label("1".into()),
            ParseLine::Jump("1".into(), args!("always", "0", "0")),
            ParseLine::Jump("2".into(), args!("always", "0", "0")),
            ParseLine::Args(args!("end")),
            ParseLine::Jump("1".into(), args!("always", "0", "0")),
        ]);
    }
}
