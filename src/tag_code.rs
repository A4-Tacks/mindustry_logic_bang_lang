use std::{
    collections::{HashMap, HashSet},
    mem::swap,
    ops::{Deref, DerefMut},
    str::FromStr, num::ParseIntError,
};

pub type Tag = usize;
pub type TagsTable = Vec<usize>;
pub const UNINIT_TAG_TARGET: usize = usize::MAX;

/// 传入`TagsTable`, 生成逻辑代码
pub trait Compile {
    fn compile(&self, tags_table: &TagsTable) -> String;
}

/// 带有`Tag`信息的封装一个数据
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct TagBox<T> {
    tag: Option<Tag>,
    data: T,
}
impl<T> Deref for TagBox<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}
impl<T> DerefMut for TagBox<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}
impl<T> TagBox<T> {
    pub fn tag(&self) -> Option<Tag> {
        self.tag
    }

    pub fn data(&self) -> &T {
        &self.data
    }

    pub fn into_inner(self) -> (Option<Tag>, T) {
        (self.tag, self.data)
    }

    pub fn into_data(self) -> T {
        self.into_inner().1
    }
}
impl<T> From<(Option<Tag>, T)> for TagBox<T> {
    fn from((tag, data): (Option<Tag>, T)) -> Self {
        Self { tag, data }
    }
}
impl<T> From<(Tag, T)> for TagBox<T> {
    fn from((tag, value): (Tag, T)) -> Self {
        (Some(tag), value).into()
    }
}
impl<T> From<T> for TagBox<T> {
    fn from(value: T) -> Self {
        (None, value).into()
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Jump(pub Tag, pub String);
impl From<(Tag, &str)> for Jump {
    fn from((tag, value): (Tag, &str)) -> Self {
        let value: String = value.into();
        (tag, value).into()
    }
}
impl From<(Tag, String)> for Jump {
    fn from((tag, value): (Tag, String)) -> Self {
        Self(tag, value)
    }
}
impl Compile for Jump {
    fn compile(&self, tags_table: &TagsTable) -> String {
        assert!(self.0 < tags_table.len()); // 越界检查
        assert_ne!(tags_table[self.0], UNINIT_TAG_TARGET); // 确保要跳转的目标有效
        format!("jump {} {}", tags_table[self.0], self.1)
    }
}

impl Compile for String {
    fn compile(&self, _tags_table: &TagsTable) -> String {
        self.clone()
    }
}

#[derive(PartialEq, Eq, Clone)]
pub enum TagLine {
    Jump(TagBox<Jump>),
    Line(TagBox<String>),
    TagDown(Tag),
}
impl std::fmt::Debug for TagLine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fn push_tag(s: &mut String, tag: Option<usize>, tail: bool) {
            if let Some(tag) = tag {
                s.push(':');
                s.push_str(&tag.to_string());
                if tail {
                    s.push(' ');
                }
            }
        }
        let mut res = String::new();
        match self {
            Self::Jump(jump) => {
                push_tag(&mut res, jump.tag, true);
                res.push_str(&format!("jump :{} {:?}", jump.data().0, jump.data().1));
            },
            Self::Line(line) => {
                push_tag(&mut res, line.tag, true);
                res.push_str(&format!("{:?}", line.data()));
            },
            &Self::TagDown(tag) => push_tag(&mut res, Some(tag), false),
        }
        write!(f, "TagLine({})", res)
    }
}
impl TagLine {
    /// Returns `true` if the tag line is [`Jump`].
    ///
    /// [`Jump`]: TagLine::Jump
    #[must_use]
    pub fn is_jump(&self) -> bool {
        matches!(self, Self::Jump(..))
    }

    pub fn as_jump(&self) -> Option<&TagBox<Jump>> {
        if let Self::Jump(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn as_jump_mut(&mut self) -> Option<&mut TagBox<Jump>> {
        if let Self::Jump(v) = self {
            Some(v)
        } else {
            None
        }
    }

    /// Returns `true` if the tag line is [`Line`].
    ///
    /// [`Line`]: TagLine::Line
    #[must_use]
    pub fn is_line(&self) -> bool {
        matches!(self, Self::Line(..))
    }

    pub fn as_line(&self) -> Option<&TagBox<String>> {
        if let Self::Line(v) = self {
            Some(v)
        } else {
            None
        }
    }

    /// 从[`Line`]或者[`Jump`]变体获取其`Tag`, 但是这不包括[`TagDown`]变体
    /// 因为此方法是为了获取当前行的`Tag`
    /// 如果是[`TagDown`]变体则会触发`panic`
    ///
    /// [`Line`]: `Self::Line`
    /// [`Jump`]: `Self::Jump`
    /// [`TagDown`]: `Self::TagDown`
    pub fn tag(&self) -> Option<usize> {
        match self {
            Self::Jump(jump) => jump.tag(),
            Self::Line(line) => line.tag(),
            other => panic!("take_tag failed: {:?}", other),
        }
    }

    pub fn as_line_mut(&mut self) -> Option<&mut TagBox<String>> {
        if let Self::Line(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn as_tag_down(&self) -> Option<&Tag> {
        if let Self::TagDown(v) = self {
            Some(v)
        } else {
            None
        }
    }

    /// Returns `true` if the tag line is [`TagDown`].
    ///
    /// [`TagDown`]: TagLine::TagDown
    #[must_use]
    pub fn is_tag_down(&self) -> bool {
        matches!(self, Self::TagDown(..))
    }
}
impl From<TagBox<String>> for TagLine {
    fn from(value: TagBox<String>) -> Self {
        Self::Line(value)
    }
}
impl From<TagBox<Jump>> for TagLine {
    fn from(value: TagBox<Jump>) -> Self {
        Self::Jump(value)
    }
}
impl From<&str> for TagLine {
    fn from(value: &str) -> Self {
        let str: String = value.into();
        str.into()
    }
}
impl From<String> for TagLine {
    fn from(value: String) -> Self {
        TagBox::from(value).into()
    }
}
impl From<Jump> for TagLine {
    fn from(value: Jump) -> Self {
        Self::Jump(value.into())
    }
}
impl Compile for TagLine {
    fn compile(&self, tags_table: &TagsTable) -> String {
        match self {
            Self::Jump(jump) => jump.data().compile(tags_table),
            Self::Line(line) => line.data().compile(tags_table),
            Self::TagDown(tag) => panic!("未被处理的 TagDown {}", tag),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ParseTagCodesError {
    ParseIntError(ParseIntError),
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct TagCodes {
    lines: Vec<TagLine>,
}
impl FromStr for TagCodes {
    type Err = (usize, ParseTagCodesError);

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // K: jump所在行, jump所用tag为第n个jump
        let mut jumps: HashSet<usize> = HashSet::new();
        // K: 被跳转行, V: tag
        let mut tags: HashMap<usize, Vec<Tag>> = HashMap::new();
        let lines =
            || s.lines().filter(|s| !s.trim().is_empty());

        for (i, line) in lines().enumerate() {
            if line.starts_with("jump") {
                let target = take_jump_target(line);
                if target == "-1" {
                    // no target jump
                    continue;
                }
                let tag = jumps.len();
                jumps.insert(i);
                let target_idx = target
                    .parse::<usize>()
                    .map_err(|e| (
                            i,
                            ParseTagCodesError::ParseIntError(e)
                    ))?;
                tags.entry(target_idx).or_default().push(tag);
            }
        }

        let mut res_lines: Vec<TagLine> = Vec::new();
        let mut jump_count = 0;
        for (i, line) in lines().enumerate() {
            if let Some(self_tags) = tags.get(&i) {
                for &tag in self_tags {
                    res_lines.push(TagLine::TagDown(tag))
                }
            }
            if jumps.get(&i).is_some() {
                // 该行为一个jump
                let jump_body = take_jump_body(line);
                res_lines.push(TagLine::Jump(
                        Jump(jump_count, jump_body).into()
                ));
                jump_count += 1;
            } else {
                res_lines.push(TagLine::Line(line.to_string().into()))
            }
        }
        // 需要先给jump建立索引
        // 记录每个被跳转点, 记录每个jump及其跳转目标标记
        //
        // 遍历每行, 如果是一个被记录的jump行则构建jump
        // 同时, 如果该行为一个被跳转点, 那么构建时在其上增加一个`TagDown`
        Ok(res_lines.into())
    }
}
impl From<Vec<TagLine>> for TagCodes {
    fn from(lines: Vec<TagLine>) -> Self {
        Self { lines }
    }
}
impl IntoIterator for TagCodes {
    type Item = TagLine;
    type IntoIter = std::vec::IntoIter<TagLine>;

    fn into_iter(self) -> Self::IntoIter {
        self.lines.into_iter()
    }
}
impl Extend<TagLine> for TagCodes {
    fn extend<T: IntoIterator<Item = TagLine>>(&mut self, iter: T) {
        self.lines.extend(iter)
    }
}
impl TagCodes {
    pub fn new() -> Self {
        Self {
            lines: Vec::new()
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            lines: Vec::with_capacity(capacity)
        }
    }

    pub fn push(&mut self, line: TagLine) {
        self.lines.push(line)
    }

    /// 构建, 将[`TagDown`]消除, 如果是最后一行裸`Tag`则被丢弃
    /// 如果目标`Tag`重复则将其返回
    /// > jump :a
    /// > :a
    /// > :b
    /// > foo
    /// 会变成
    /// > jump :b
    /// > :b foo
    /// ---
    /// > jump :a
    /// > :a
    /// > :b foo
    /// 会变成
    /// > jump :b
    /// > :b foo
    pub fn build_tagdown(&mut self) -> Result<(), (usize, Tag)> {
        let mut tag_alias_map: HashMap<Tag, Tag> = HashMap::new();
        let mut lines: Vec<TagLine> = Vec::new();
        let mut map_stack: Vec<Tag> = Vec::new();

        swap(&mut lines, &mut self.lines); // 将`self.lines`换出

        // 构建索引, 如果有重复tag则将其返回
        for (i, line) in lines.iter_mut().enumerate() {
            let ( TagLine::Jump(TagBox { tag, .. })
                | TagLine::Line(TagBox { tag, .. })
                ) = line
            else {
                let &mut TagLine::TagDown(tag) = line else { panic!() };
                if tag_alias_map.get(&tag).is_some() {
                    // 重复的标记, 当然, 如果是一组则不报错, 因为没进行插入
                    // 例如:
                    // ```
                    // :a
                    // :b
                    // :b
                    // ```
                    return Err((i, tag));
                }
                map_stack.push(tag);
                continue
            };
            // 如果是没被自标记的行则尝试将栈中一个标记取过来做标记
            // 如果是有自标记的行则将栈中所有标记映射到它
            if tag.is_none() {
                // 尝试将映射栈中最后一个标记取过来当做本行标记
                // 这会填补上方有`TagDown`但自身没有`Tag`的行
                *tag = map_stack.pop()
            }
            if let &mut Some(tag) = tag {
                if tag_alias_map.get(&tag).is_some() {
                    // 映射到的目标是重复的
                    return Err((i, tag));
                }
                tag_alias_map.insert(tag, tag); // 将自己插入
                while let Some(from) = map_stack.pop() {
                    tag_alias_map.insert(from, tag);
                }
            }
        }

        drop(map_stack);

        for mut line in lines {
            let tag_refs: Vec<&mut Tag> = match &mut line {
                TagLine::Jump(TagBox {
                    tag: Some(tag),
                    data: Jump(j_dst, ..)
                }) => vec![tag, j_dst],
                TagLine::Jump(TagBox {
                    tag: None,
                    data: Jump(j_dst, ..)
                }) => vec![j_dst],
                TagLine::Line(TagBox {
                    tag: Some(tag),
                    ..
                }) => vec![tag],
                TagLine::Line(TagBox {
                    tag: None,
                    ..
                }) => vec![],
                TagLine::TagDown(..) => continue
            };
            for tag in tag_refs {
                // 将每个`Tag`(包括jump目标)进行映射
                tag_alias_map.get(&tag).map(|&dst| *tag = dst);
            }
            // 将非`TagDown`行加入
            self.lines.push(line);
        }

        Ok(())
    }

    /// 编译为逻辑行码
    /// 如果有重复的`Tag`, 返回其行下标及重复`Tag`
    /// 会调用[`build_tagdown`]来改变源码
    ///
    /// [`build_tagdown`]: `TagCodes::build_tagdown`
    pub fn compile(&mut self) -> Result<Vec<String>, (usize, Tag)> {
        self.build_tagdown()?; // 构建为行内跳转标记, 而不是`TagDown`

        let mut tags_table: TagsTable = TagsTable::new();

        for (num, code) in self.lines.iter().enumerate() {
            // 构建索引
            if let Some(tag) = code.tag() {
                for _ in tags_table.len()..=tag {
                    // 使用`Tag::MAX`来代表初始化无效值
                    tags_table.push(UNINIT_TAG_TARGET)
                }
                // 确保是未初始化的, 不能有多个重复`Tag`目标
                assert_eq!(tags_table[tag], UNINIT_TAG_TARGET);
                tags_table[tag] = num
            }
        }

        let mut logic_lines = Vec::with_capacity(self.lines.len());
        for line in &self.lines {
            logic_lines.push(line.compile(&tags_table))
        }
        Ok(logic_lines)
    }
}

fn take_jump_target(line: &str) -> String {
    line.chars()
        .skip(4)
        .skip_while(|c| c.is_whitespace()) // ` `
        .take_while(|c| !c.is_whitespace())
        .collect()
}
fn take_jump_body(line: &str) -> String {
    line
        .chars()
        .skip(4) // `jump`
        .skip_while(|c| c.is_whitespace()) // ` `
        .skip_while(|c| !c.is_whitespace()) // `123`
        .skip_while(|c| c.is_whitespace()) // ` `
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! tag_line {
        ($(: $tag:literal)? jump $dst:literal $str:literal ) => {
            TagLine::Jump(TagBox::from(($( $tag as usize, )? Jump::from(($dst, $str)))))
        };
        ($(: $tag:literal)? $str:literal ) => {
            TagLine::Line(TagBox::from(($( $tag as usize, )? ($str).to_string())))
        };
        (: $tag:literal) => {
            TagLine::TagDown(($tag as usize).into())
        };
    }
    macro_rules! tag_lines {
        ($( [$($t:tt)*] ; )*) => {{
            let mut lines = TagCodes::new();
            $(
                lines.push(tag_line!($($t)*));
            )*
            lines
        }};
    }

    const MY_INSERT_SORT_LOGIC_LINES: [&str; 19] = {[
        "sensor enabled switch1 @enabled",
        "wait 0.1",
        "jump 0 equal enabled true",
        "read length cell1 0",
        "set i 1",
        "jump 17 always 0 0",
        "read num bank1 i",
        "set j i",
        "jump 12 always 0 0",
        "read num_1 bank1 j",
        "jump 15 lessThanEq num_1 num",
        "write num_1 bank1 c",
        "set c j",
        "op sub j j 1",
        "jump 9 greaterThanEq j 0",
        "write num bank1 c",
        "op add i i 1",
        "jump 6 lessThan i length",
        "control enabled switch1 true 0 0 0",
    ]};

    #[test]
    fn build_tagdown_test() {
        let mut lines = tag_lines! {
            [:0 "start"];
            [jump 0 "to start"];
            [:1];
            [:2];
            ["1&2"];
            [:3];
            [:4];
            [:5 "3&4&5"];
            [:6 jump 1 "self(6) to 1"];
            [:7];
            [jump 2 "self(7) to 2"];
            [:8];
            [:9];
        };
        assert_eq!(lines.build_tagdown(), Ok(()));
        assert_eq!(lines, tag_lines! {
            [:0 "start"];
            [jump 0 "to start"];
            [:2 "1&2"];
            [:5 "3&4&5"];
            [:6 jump 2 "self(6) to 1"];
            [:7 jump 2 "self(7) to 2"];
        });
    }

    #[test]
    fn jump_tag_test() {
        let src = MY_INSERT_SORT_LOGIC_LINES; // 我写的插排
        let mut lines = tag_lines! {
            [:0];
            ["sensor enabled switch1 @enabled"];
            [jump 0 "equal enabled true"];
            ["read length cell1 0"];
            ["set i 1"];
            [jump 5 "always 0 0"];
            [:1];
            ["read num bank1 i"];
            ["set j i"];
            [jump 2 "always 0 0"];
            [:4];
            ["read num_1 bank1 j"];
            [jump 3 "lessThanEq num_1 num"];
            ["write num_1 bank1 c"];
            [:2 "set c j"];
            ["op sub j j 1"];
            [jump 4 "greaterThanEq j 0"];
            [:3];
            ["write num bank1 c"];
            ["op add i i 1"];
            [:5 jump 1 "lessThan i length"];
            ["control enabled switch1 true 0 0 0"];
        };
        assert_eq!(lines.compile().unwrap(), src);
    }

    #[test]
    fn take_jump_test() {
        assert_eq!(take_jump_body("jump -1 abc"), "abc");
        assert_eq!(take_jump_body("jump   -1     abc"), "abc");
        assert_eq!(take_jump_body("jump   1234     abc  def"), "abc  def");

        assert_eq!(take_jump_target("jump -1 always 0 0"), "-1");
        assert_eq!(take_jump_target("jump   -1      always 0 0"), "-1");
        assert_eq!(take_jump_target("jump   1      always 0 0"), "1");
        assert_eq!(take_jump_target("jump   176      always 0 0"), "176");
    }

    #[test]
    fn from_str_test() {
        let mut tag_lines: TagCodes = MY_INSERT_SORT_LOGIC_LINES
            .join("\n")
            .parse()
            .unwrap();
        let lines = tag_lines.compile().unwrap();
        assert_eq!(lines, &MY_INSERT_SORT_LOGIC_LINES);
    }
}
