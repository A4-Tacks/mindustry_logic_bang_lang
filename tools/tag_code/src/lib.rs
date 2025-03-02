use std::{
    collections::HashMap,
    fmt::Display,
    mem::{replace, swap},
    ops::{Deref, DerefMut},
    panic::catch_unwind,
    process::exit,
};

use logic_parser::{IdxBox, ParseLine, ParseLines};

pub mod logic_parser;

pub type Tag = usize;
pub type TagsTable = Vec<usize>;
pub const UNINIT_TAG_TARGET: usize = usize::MAX;

/// 带有错误前缀, 并且文本为红色的eprintln
macro_rules! err {
    ( $fmtter:expr $(, $args:expr)* $(,)? ) => {
        eprintln!(concat!("\x1b[1;31m", "TagCodeError: ", $fmtter, "\x1b[22;39m"), $($args),*);
    };
}

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
    pub fn new(tag: Option<Tag>, data: T) -> Self {
        Self { tag, data }
    }

    pub fn tag(&self) -> Option<Tag> {
        self.tag
    }

    pub fn tag_mut(&mut self) -> &mut Option<Tag> {
        &mut self.tag
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

macro_rules! panic_tag_out_of_range {
    ($id:expr, $tags_table:expr $(,)?) => {
        err!(
            concat!(
                "进行了越界的跳转标签构建, ",
                "你可以查看是否在尾部编写了空的被跳转标签\n",
                "标签id: {}, 目标行表:\n",
                "id \t-> target\n",
                "{}",
            ),
            $id,
            $tags_table.iter()
                .enumerate()
                .map(|(id, &target)| {
                    format!("\t{} \t-> {},", id, if target == UNINIT_TAG_TARGET {
                        "{unknown}".to_string()
                    } else {
                        target.to_string()
                    })
                })
                .collect::<Vec<_>>()
                .join("\n")
        );
        panic!("显式恐慌");
    };
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Jump(pub Tag, pub String);
impl Jump {
    pub fn new_always(target: Tag) -> Self {
        Self(target, "always 0 0".into())
    }

    /// 尝试转换到即将编译的状态, 可以在此处进行一些优化
    pub fn try_to_start_compile(&self) -> Option<Self> {
        let mut res = None;
        if !self.is_literal_always_jump() && self.is_always_jump() {
            res = Self::new_always(self.0).into()
        }
        res
    }

    /// 判断自身是否为一个无条件跳转
    ///
    /// 在两运算成员相同的情况下, 匹配以下运算
    /// - 相等
    /// - 严格相等
    /// - 小于等于
    /// - 大于等于
    ///
    /// 在其余情况下, 返回[`is_literal_always_jump`]的结果
    ///
    /// [`is_literal_always_jump`]: Self::is_literal_always_jump
    pub fn is_always_jump(&self) -> bool {
        let jump_body = self.1.as_str();
        let jump_args = mdt_logic_split_unwraped(jump_body);
        match jump_args[..] {
            [
                | "equal"
                | "strictEqual"
                | "lessThanEq"
                | "greaterThanEq"
                ,
                a,
                b
            ] if a == b => true,
            _ => self.is_literal_always_jump(),
        }
    }

    /// 不考虑运算成员, 仅考虑字面上是否是一个无条件跳转
    pub fn is_literal_always_jump(&self) -> bool {
        let jump_body = self.1.as_str();
        let jump_args = mdt_logic_split_unwraped(jump_body);
        matches!(jump_args[..], ["always", ..])
    }

    /// 校验跳转目标是否在行表中, 如果不在则进行恐慌
    pub fn check_target_unwrap(&self, tags_table: &TagsTable) {
        if self.0 >= tags_table.len() || tags_table[self.0] == UNINIT_TAG_TARGET {
            panic_tag_out_of_range!(self.0, tags_table);
        }
    }
}
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
        let mut this = self;
        let try_new_this = this.try_to_start_compile();
        if let Some(new_this) = try_new_this.as_ref() { this = new_this }

        this.check_target_unwrap(tags_table);
        assert!(this.0 < tags_table.len()); // 越界检查
        assert_ne!(tags_table[this.0], UNINIT_TAG_TARGET); // 确保要跳转的目标有效
        format!("jump {} {}", tags_table[this.0], this.1)
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
impl Display for TagLine {
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
                res.push_str(&format!("jump :{} {}", jump.data().0, jump.data().1));
            },
            Self::Line(line) => {
                push_tag(&mut res, line.tag, true);
                res.push_str(&line.data().to_string());
            },
            &Self::TagDown(tag) => push_tag(&mut res, Some(tag), false),
        }
        write!(f, "{}", res)
    }
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
    pub fn tag(&self) -> Option<Tag> {
        match self {
            Self::Jump(jump) => jump.tag(),
            Self::Line(line) => line.tag(),
            other => panic!("take_tag failed: {:?}", other),
        }
    }

    /// 从[`Line`]或者[`Jump`]变体获取其`Tag`的可变引用, 但是这不包括[`TagDown`]变体
    /// 因为此方法是为了获取当前行的`Tag`
    /// 如果是[`TagDown`]变体则会触发`panic`
    ///
    /// [`Line`]: `Self::Line`
    /// [`Jump`]: `Self::Jump`
    /// [`TagDown`]: `Self::TagDown`
    pub fn tag_mut(&mut self) -> &mut Option<Tag> {
        match self {
            Self::Jump(jump) => jump.tag_mut(),
            Self::Line(line) => line.tag_mut(),
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

    /// 返回是否生成行, 用于未生成时预测生成后长度
    #[must_use]
    pub fn can_generate_line(&self) -> bool {
        match self {
            TagLine::Jump(_) | TagLine::Line(_) => true,
            TagLine::TagDown(_) => false,
        }
    }

    /// 如果是一个有被跳转标记的[`Line`]或者[`Jump`], 将其跳转标记分离出来
    /// 如果是一个[`TagDown`], 则返回[`None`]
    ///
    /// [`Line`]: `Self::Line`
    /// [`Jump`]: `Self::Jump`
    /// [`TagDown`]: `Self::TagDown`
    pub fn pop_tag(&mut self) -> Option<Tag> {
        match self {
            Self::Jump(TagBox { tag, .. })
                | Self::Line(TagBox { tag, .. })
                => tag.take(),
            Self::TagDown(..) => None
        }
    }

    /// 传入一行tag码
    /// 如果是以`:`开头则构建为[`TagDown`]
    /// 如果是以jump开头则拿第二个参数建表构建为[`Jump`]
    /// 否则构建为[`Line`]
    ///
    /// [`Line`]: `Self::Line`
    /// [`Jump`]: `Self::Jump`
    /// [`TagDown`]: `Self::TagDown`
    pub fn from_tag_str(s: &str, tag_map: &mut HashMap<String, usize>) -> Self {
        fn get_tag_body(s: &str) -> String {
            s.chars()
                .skip(1)
                .take_while(|c| !c.is_whitespace())
                .collect::<String>()
        }

        let len = tag_map.len(); // 插入前的表长, 做插入用id

        macro_rules! get_or_insert_tag {
            ($tag:expr) => {
                *tag_map.entry($tag)
                    .or_insert_with(
                        move || len
                    )
            };
        }

        if s.starts_with(':') {
            let tag = get_tag_body(s);
            Self::TagDown(get_or_insert_tag!(tag))
        } else if s.starts_with("jump") {
            let tag = get_tag_body(&take_jump_target(s));
            let body = take_jump_body(s);
            Jump(get_or_insert_tag!(tag), body).into()
        } else {
            Self::Line(s.to_string().into())
        }
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
    RepeatLabel(String),
    MissedLabel(String),
}
impl Display for ParseTagCodesError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseTagCodesError::RepeatLabel(lab) => {
                write!(f, "重复的标签 `{lab}`")
            },
            ParseTagCodesError::MissedLabel(lab) => {
                write!(f, "未命中的标签 `{lab}`")
            },
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct TagCodes {
    lines: Vec<TagLine>,
}
impl<'a> TryFrom<ParseLines<'a>> for TagCodes {
    type Error = IdxBox<ParseTagCodesError>;

    fn try_from(mut codes: ParseLines<'a>) -> Result<Self, Self::Error> {
        codes.index_label_popup();

        let mut lab2idx = HashMap::with_capacity(codes.modif_count());
        codes.iter()
            .filter_map(|x| x.as_ref()
                .and_then(|x| x.as_label()))
            .try_for_each(|label|
        {
            if lab2idx.insert(*label, lab2idx.len()).is_some() {
                return Err(label.new_value(ParseTagCodesError::RepeatLabel(
                    label.to_string()
                )));
            }
            Ok(())
        })?;

        let get = |lab: IdxBox<&str>| {
            lab2idx.get(*lab)
                .copied()
                .ok_or_else(|| {
                    lab.new_value(ParseTagCodesError::MissedLabel(
                        lab.to_string()
                    ))
                })
        };

        let lines = codes.iter().map(|line| {
            match *line.as_ref() {
                ParseLine::Label(lab) => {
                    let tag = get(line.new_value(lab.as_ref()))?;
                    Ok(TagLine::TagDown(tag))
                },
                ParseLine::Jump(tgt, args) => {
                    let tag = get(line.new_value(tgt.as_ref()))?;
                    Ok(TagLine::Jump(Jump(tag, args.join(" ")).into()))
                },
                ParseLine::Args(args) => {
                    Ok(TagLine::Line(args.join(" ").into()))
                },
            }
        }).collect::<Result<_, IdxBox<ParseTagCodesError>>>()?;

        Ok(Self { lines })
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

    pub fn pop(&mut self) -> Option<TagLine> {
        self.lines.pop()
    }

    /// 直接在指定位置插入语句, 慎用!
    pub fn insert(&mut self, index: usize, line: TagLine) {
        self.lines.insert(index, line)
    }

    pub fn lines(&self) -> &Vec<TagLine> {
        &self.lines
    }

    pub fn lines_mut(&mut self) -> &mut Vec<TagLine> {
        &mut self.lines
    }

    pub fn clear(&mut self) {
        self.lines.clear()
    }

    /// 构建, 将[`TagDown`]消除,
    /// 如果目标`Tag`重复则将其返回
    ///
    /// > jump :a
    /// > :a
    /// > :b
    /// > foo
    ///
    /// 会变成
    /// > jump :b
    /// > :b foo
    /// ---
    /// > jump :a
    /// > :a
    /// > :b foo
    ///
    /// 会变成
    /// > jump :b
    /// > :b foo
    ///
    /// [`TagDown`]: TagLine::TagDown
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
                if tag_alias_map.contains_key(&tag) {
                    // 重复的标记, 当然, 如果是一组则不报错, 因为没进行插入
                    // 例如:
                    // ```
                    // :a
                    // :b
                    // :b
                    // ```
                    self.lines = lines;
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
                if tag_alias_map.contains_key(&tag) {
                    // 映射到的目标是重复的
                    self.lines = lines;
                    return Err((i, tag));
                }
                tag_alias_map.insert(tag, tag); // 将自己插入
                while let Some(from) = map_stack.pop() {
                    tag_alias_map.insert(from, tag);
                }
            }
        }

        // 对于尾部索引进行构建
        if let Some(first) = lines.iter_mut().find(|line| !line.is_tag_down()) {
            if first.is_jump() || first.is_line() {
                let tag = first.tag_mut();
                if tag.is_none() {
                    *tag = map_stack.pop();
                }
                if let &mut Some(tag) = tag {
                    // 将尾部tag映射到头部语句
                    for other in map_stack {
                        tag_alias_map.insert(other, tag)
                            .ok_or(())
                            .unwrap_err()
                    }
                }
            }
        }

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
                if let Some(&dst) = tag_alias_map.get(tag) {
                    *tag = dst
                }
            }
            // 将非`TagDown`行加入
            self.lines.push(line);
        }

        Ok(())
    }

    /// 对目标跳转表中的无条件跳转链进行跟踪,
    /// 这可以优化掉跳转表中的一些无意义跳转
    pub fn follow_always_jump_chain(&self, tags_table: &mut TagsTable) {
        // 当本标签对应行为无条件跳转, 且不是跳转其自身
        // 那么将本标签跳转目标改为这个无条件跳转的目标
        let lines = self.lines();
        let mut tag_target_history = Vec::new();
        let mut is_loop_jumps = Vec::new();
        let add_history
            = |history: &mut Vec<_>, tag: usize| {
                if tag >= history.len() {
                    history.resize(tag+1, false)
                }
                let res = history[tag];
                history[tag] = true;
                res
            };
        for tag in 0..tags_table.len() {
            if is_loop_jumps.get(tag).copied().unwrap_or_default() { continue }
            let init_line_idx = tags_table[tag];
            let mut line_idx = init_line_idx;
            if line_idx == UNINIT_TAG_TARGET { continue }
            tag_target_history.fill(false); // init
            loop {
                let line @ &TagLine::Jump(TagBox {
                    tag: self_tag,
                    data: ref jump @ Jump(target_tag, _)
                }) = &lines[line_idx] else { break };
                assert!(line.as_tag_down().is_none());
                if Some(target_tag) == self_tag || ! jump.is_always_jump() { break }
                jump.check_target_unwrap(tags_table);
                if add_history(&mut tag_target_history, target_tag) {
                    // 如果是环路, 那么不要去处理
                    line_idx = init_line_idx;
                    let iter = tag_target_history
                        .iter()
                        .enumerate()
                        .filter(|&(_, &x)| x)
                        .map(|(i, _)| i);
                    for looped_tag in iter {
                        add_history(&mut is_loop_jumps, looped_tag);
                    }
                    break
                }
                line_idx = tags_table[target_tag];
            }
            tags_table[tag] = line_idx
        }
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

        self.follow_always_jump_chain(&mut tags_table);

        let mut logic_lines = Vec::with_capacity(self.lines.len());
        for line in &self.lines {
            match catch_unwind(|| line.compile(&tags_table)) {
                Ok(line) => logic_lines.push(line),
                Err(_e) => {
                    err!(
                        concat!(
                            "构建行时出现了恐慌, 全部须构建的行:\n{}\n",
                            "已经构建完毕的行:\n{}\n",
                            "恐慌的行:\n\t{}\n",
                        ),
                        self.lines()
                            .iter()
                            .map(|x| format!("\t{}", x))
                            .collect::<Vec<_>>()
                            .join("\n"),
                        logic_lines.iter()
                            .map(|x| format!("\t{}", x))
                            .collect::<Vec<_>>()
                            .join("\n"),
                        line,
                    );
                    exit(8);
                },
            };
        }
        Ok(logic_lines)
    }

    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    /// 获取内部代码条数
    pub fn len(&self) -> usize {
        self.lines.len()
    }

    /// 获取不是[`TagDown`]的代码条数
    ///
    /// [`TagDown`]: `TagLine::TagDown`
    pub fn count_no_tag(&self) -> usize {
        // 这可以通过封装并内部维护值来实现, 但是目前是使用遍历的低效实现
        // 因为这个语言实在是太快了, 处理的数据量也太少了
        self.lines
            .iter()
            .filter(|line| line.can_generate_line())
            .count()
    }

    /// 给定一个在没有考虑[`TagDown`]也就是忽略时的索引
    /// 返回不忽略[`TagDown`]时的真正索引
    /// 当给定索引超出时会返回[`None`]
    ///
    /// [`TagDown`]: `TagLine::TagDown`
    pub fn no_tag_index_to_abs_index(&self, index: usize) -> Option<usize> {
        self
            .iter()
            .enumerate()
            .filter(|(_i, line)| line.can_generate_line())
            .nth(index)
            .map(|(i, _line)| i)
    }

    pub fn iter(&self) -> std::slice::Iter<TagLine> {
        self.lines.iter()
    }

    /// 将所有被跳转的行内`Tag`进行上提为[`TagDown`]
    /// 原有的[`TagDown`]保持不变
    ///
    /// [`TagDown`]: `TagLine::TagDown`
    pub fn tag_up(&mut self) {
        let len = self.len();
        let lines
            = replace(&mut self.lines, Vec::with_capacity(len));
        for mut line in lines {
            if let Some(tag_down) = line.pop_tag() {
                self.lines.push(TagLine::TagDown(tag_down))
            }
            self.lines.push(line)
        }
    }

    /// 对字符串中每行调用[`TagLine::from_tag_str`]来构建
    pub fn from_tag_lines(s: &str) -> Self {
        let mut lines = Vec::new();
        let mut tag_map = HashMap::new();

        for line in s.lines() {
            lines.push(TagLine::from_tag_str(line, &mut tag_map))
        }

        lines.into()
    }
}
impl Default for TagCodes {
    fn default() -> Self {
        Self::new()
    }
}
impl Display for TagCodes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut iter = self.lines().iter();
        if let Some(first) = iter.next() {
            write!(f, "{first}")?;
        }
        for line in iter {
            write!(f, "\n{line}")?;
        }
        Ok(())
    }
}

fn take_jump_target(line: &str) -> String {
    debug_assert!(line.starts_with("jump"));

    let is_whitespace = |ch: &char| ch.is_whitespace();
    let is_not_whitespace = |ch: &char| ! ch.is_whitespace();

    line.chars()
        .skip(4)
        .skip_while(is_whitespace) // ` `
        .take_while(is_not_whitespace)
        .collect()
}
fn take_jump_body(line: &str) -> String {
    debug_assert!(line.starts_with("jump"));

    let is_whitespace = |ch: &char| ch.is_whitespace();
    let is_not_whitespace = |ch: &char| ! ch.is_whitespace();

    line
        .chars()
        .skip(4) // `jump`
        .skip_while(is_whitespace) // ` `
        .skip_while(is_not_whitespace) // `123`
        .skip_while(is_whitespace) // ` `
        .collect()
}

/// 按照Mindustry中的规则进行切分
/// 也就是空白忽略, 字符串会被保留完整
/// 如果出现未闭合字符串则会返回其所在字符数(从1开始)
pub fn mdt_logic_split(s: &str) -> Result<Vec<&str>, usize> {
    fn get_next_char_idx(s: &str) -> Option<usize> {
        s
            .char_indices()
            .map(|(i, _)| i)
            .nth(1)
    }
    let mut res = Vec::new();
    let mut s1 = s.trim_start();
    while !s1.is_empty() {
        debug_assert!(! s1.chars().next().unwrap().is_whitespace());
        if s1.starts_with('"') {
            // string
            if let Some(mut idx) = s1.strip_prefix('"').unwrap().find('"') {
                idx += '"'.len_utf8();
                res.push(&s1[..=idx]);
                s1 = &s1[idx..];
                let Some(next_char_idx) = get_next_char_idx(s1) else { break };
                s1 = &s1[next_char_idx..]
            } else {
                let byte_idx = s.len() - s1.len();
                let char_idx = s
                    .char_indices()
                    .position(|(idx, _ch)| {
                        byte_idx == idx
                    })
                    .unwrap();
                return Err(char_idx + 1)
            }
        } else if s1.starts_with('#') {
            s1 = "";
        } else {
            let end = s1
                .find(|ch: char| ch.is_whitespace() || ch == '"')
                .unwrap_or(s1.len());
            res.push(&s1[..end]);
            s1 = &s1[end..]
        }
        s1 = s1.trim_start();
    }
    Ok(res)
}
#[must_use]
pub fn mdt_logic_split_unwraped(s: &str) -> Vec<&str> {
    mdt_logic_split(s).unwrap_or_else(|err_char_cl| {
        err!("未闭合的字符串, 在第 {} 个字符处, 行: {}", err_char_cl, s);
        exit(8)
    })
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
        ($( [$($t:tt)*] $(;)? )*) => {{
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
            ["wait 0.1"];
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
        let src = MY_INSERT_SORT_LOGIC_LINES.join("\n");
        let logic_lines = logic_parser::parser::lines(&src).unwrap();
        let mut tag_lines: TagCodes = logic_lines.try_into().unwrap();
        let lines = tag_lines.compile().unwrap();
        assert_eq!(lines, &MY_INSERT_SORT_LOGIC_LINES);
    }

    #[test]
    fn no_tag_index_to_abs_index_test() {
        let tag_codes = tag_lines! {
            [:0];
            [:1];
            ["noop"];
            [:1];
            ["noop"];
            ["noop"];
            [:1];
        };
        assert_eq!(tag_codes.no_tag_index_to_abs_index(0), Some(2));
        assert_eq!(tag_codes.no_tag_index_to_abs_index(1), Some(4));
        assert_eq!(tag_codes.no_tag_index_to_abs_index(2), Some(5));
        assert_eq!(tag_codes.no_tag_index_to_abs_index(3), None);
    }

    #[test]
    fn tail_tag_test() {
        let mut tag_codes = tag_lines! {
            [:0];
            [:1 "a"];
            ["noop"];
            [:2];
        };
        tag_codes.build_tagdown().unwrap();
        assert_eq!(tag_codes, tag_lines! {
            [:1 "a"];
            ["noop"];
        });

        let mut tag_codes = tag_lines! {
            [:0];
            [:1 "a"];
            ["noop"];
            [jump 2 "foo"];
            [:2];
        };
        tag_codes.build_tagdown().unwrap();
        assert_eq!(tag_codes, tag_lines! {
            [:1 "a"];
            ["noop"];
            [jump 1 "foo"];
        });

        let mut tag_codes = tag_lines! {
            ["a"];
            ["noop"];
            [jump 2 "foo"];
            [:2];
        };
        tag_codes.build_tagdown().unwrap();
        assert_eq!(tag_codes, tag_lines! {
            [:2 "a"];
            ["noop"];
            [jump 2 "foo"];
        });

        let mut tag_codes = tag_lines! {
            ["a"];
            ["noop"];
            [jump 2 "foo"];
            [:2];
            [:3];
        };
        tag_codes.build_tagdown().unwrap();
        assert_eq!(tag_codes, tag_lines! {
            [:3 "a"];
            ["noop"];
            [jump 3 "foo"];
        });
    }

    #[test]
    fn tag_up_test() {
        let mut tag_codes = tag_lines! {
            [:0 "a"];
            [:1 "b"];
            ["c"];
            [:2];
            [:3 "d"];
        };
        tag_codes.tag_up();
        assert_eq!(tag_codes, tag_lines! {
            [:0];
            ["a"];
            [:1];
            ["b"];
            ["c"];
            [:2];
            [:3];
            ["d"];
        });
    }

    #[test]
    fn from_tag_str_test() {
        let mut tag_map = Default::default();
        assert_eq!(TagLine::from_tag_str(":a", &mut tag_map), tag_line!(:0));
        // 标记后方所有值无效, 只有`:a`被识别了
        assert_eq!(TagLine::from_tag_str(":a abc def", &mut tag_map), tag_line!(:0));
        assert_eq!(TagLine::from_tag_str(":b", &mut tag_map), tag_line!(:1));
        assert_eq!(TagLine::from_tag_str("jump :a foo", &mut tag_map), tag_line!(jump 0 "foo"));
        assert_eq!(TagLine::from_tag_str("jump :b bar", &mut tag_map), tag_line!(jump 1 "bar"));
        assert_eq!(TagLine::from_tag_str("jump :c foo", &mut tag_map), tag_line!(jump 2 "foo"));
        assert_eq!(TagLine::from_tag_str(":c", &mut tag_map), tag_line!(:2));
        assert_eq!(TagLine::from_tag_str("op add a a 1", &mut tag_map), tag_line!("op add a a 1"));
    }

    #[test]
    fn is_always_jump_test() {
        let false_data = [
            r#"always0 0"#,
            r#"always. 0"#,
            r#"equal always 0"#,
            r#"always."#,
            r#"Always"#,
            r#"equal a b"#,
            r#"equal 2 3"#,
            r#"equal "2" "3""#,
            r#"strictEqual a b"#,
            r#"strictEqual 2 3"#,
            r#"strictEqual "2" "3""#,
        ];
        let true_data = [
            r#"equal 0 0"#,
            r#"equal a a"#,
            r#"equal "0" "0""#,
            r#"equal "a" "a""#,
            r#"strictEqual 0 0"#,
            r#"strictEqual a a"#,
            r#"strictEqual "0" "0""#,
            r#"strictEqual "a" "a""#,
            r#"always 0 0 0"#,
            r#"always 0 0"#,
            r#"always a b"#,
            r#"always a"#,
            r#"always"#,
        ];
        for src in false_data {
            assert!(! Jump(0, src.into()).is_always_jump(), "err: {:?}", src);
        }
        for src in true_data {
            assert!(Jump(0, src.into()).is_always_jump(), "err: {:?}", src);
        }
    }

    #[test]
    fn empty_compile_test() {
        assert_eq!(tag_lines! { }.compile().unwrap(), Vec::<&str>::new());
    }

    #[test]
    fn follow_always_jump_chain_test() {
        assert_eq!(
            tag_lines! {
                [jump 0 "lessThan a b"];
                ["a"];
                [:0 jump 1 "always 0 0"];
                ["b"];
                [:1 jump 2 "always 0 0"];
                [:2 "c"];
            }.compile().unwrap(),
            [
                "jump 5 lessThan a b",
                "a",
                "jump 5 always 0 0",
                "b",
                "jump 5 always 0 0",
                "c",
            ]
        );
        assert_eq!(
            tag_lines! {
                [jump 0 "lessThan a b"];
                ["a"];
                [:0 jump 1 "notEqual 0 0"];
                ["b"];
                [:1 jump 2 "always 0 0"];
                [:2 "c"];
            }.compile().unwrap(),
            [
                "jump 2 lessThan a b",
                "a",
                "jump 5 notEqual 0 0",
                "b",
                "jump 5 always 0 0",
                "c",
            ]
        );
        assert_eq!(
            tag_lines! {
                [jump 0 "lessThan a b"];
                ["a"];
                [:0 jump 1 "always 0 0"];
                ["b"];
                [:1 jump 1 "always 0 0"];
            }.compile().unwrap(),
            [
                "jump 4 lessThan a b",
                "a",
                "jump 4 always 0 0",
                "b",
                "jump 4 always 0 0",
            ]
        );
        assert_eq!( // 环路不做处理
            tag_lines! {
                [:0 jump 1 "always 0 0"];
                [:1 jump 0 "always 0 0"];
            }.compile().unwrap(),
            [
                "jump 1 always 0 0",
                "jump 0 always 0 0",
            ]
        );
        assert_eq!( // 环路不做处理
            tag_lines! {
                [:0 jump 1 "always 0 0"];
                [:1 jump 3 "always 0 0"];
                [:3 jump 0 "always 0 0"];
            }.compile().unwrap(),
            [
                "jump 1 always 0 0",
                "jump 2 always 0 0",
                "jump 0 always 0 0",
            ]
        );
        assert_eq!( // 环路不做处理
            tag_lines! {
                [:0 jump 0 "always 0 0"];
            }.compile().unwrap(),
            [
                "jump 0 always 0 0",
            ]
        );
        assert_eq!( // 环路不做处理
            tag_lines! {
                [:0 jump 2 "always 0 0"];
                [:1 jump 3 "always 0 0"];
                [:2 jump 4 "always 0 0"];
                [:3 jump 5 "always 0 0"];
                [:4 jump 5 "always 0 0"];
                [:5 jump 0 "always 0 0"];
            }.compile().unwrap(),
            [
                "jump 2 always 0 0",
                "jump 3 always 0 0",
                "jump 4 always 0 0",
                "jump 5 always 0 0",
                "jump 5 always 0 0",
                "jump 0 always 0 0",
            ]
        );
        assert_eq!(
            tag_lines! {
                [:0 jump 2 "always 0 0"];
                [:1 jump 3 "always 0 0"];
                [:2 jump 4 "always 0 0"];
                [:3 jump 5 "always 0 0"];
                [:4 jump 5 "always 0 0"];
                [:5 jump 0 "lessThan a b"];
            }.compile().unwrap(),
            [
                "jump 5 always 0 0",
                "jump 5 always 0 0",
                "jump 5 always 0 0",
                "jump 5 always 0 0",
                "jump 5 always 0 0",
                "jump 5 lessThan a b",
            ]
        );
        assert_eq!(
            tag_lines! {
                [:0 "a"];
                [:1 jump 3 "always 0 0"];
                [:2 jump 4 "always 0 0"];
                [:3 jump 5 "always 0 0"];
                [:4 jump 5 "always 0 0"];
                [:5 jump 0 "lessThan a b"];
            }.compile().unwrap(),
            [
                "a",
                "jump 5 always 0 0",
                "jump 5 always 0 0",
                "jump 5 always 0 0",
                "jump 5 always 0 0",
                "jump 0 lessThan a b",
            ]
        );
    }

    #[test]
    fn is_literal_always_jump_test() {
        let false_data = [
            r#"always0 0"#,
            r#"always. 0"#,
            r#"equal always 0"#,
            r#"always."#,
            r#"Always"#,
            r#"equal a b"#,
            r#"equal 2 3"#,
            r#"equal "2" "3""#,
            r#"strictEqual a b"#,
            r#"strictEqual 2 3"#,
            r#"strictEqual "2" "3""#,
            r#"equal 0 0"#,
            r#"equal a a"#,
            r#"equal "0" "0""#,
            r#"equal "a" "a""#,
            r#"strictEqual 0 0"#,
            r#"strictEqual a a"#,
            r#"strictEqual "0" "0""#,
            r#"strictEqual "a" "a""#,
        ];
        let true_data = [
            r#"always 0 0 0"#,
            r#"always 0 0"#,
            r#"always a b"#,
            r#"always a"#,
            r#"always"#,
        ];
        for src in false_data {
            assert!(! Jump(0, src.into()).is_literal_always_jump(), "err: {:?}", src);
        }
        for src in true_data {
            assert!(Jump(0, src.into()).is_literal_always_jump(), "err: {:?}", src);
        }
    }

    #[test]
    fn jump_to_always_test() {
        assert_eq!(
            tag_lines! {
                [:0];
                [jump 0 "always 0 0"];
            }.compile().unwrap(),
            ["jump 0 always 0 0"]
        );

        assert_eq!(
            tag_lines! {
                [:0];
                [jump 0 "always a b"];
            }.compile().unwrap(),
            ["jump 0 always a b"]
        );

        assert_eq!(
            tag_lines! {
                [:0];
                [jump 0 "equal 0 0"];
            }.compile().unwrap(),
            ["jump 0 always 0 0"]
        );

        assert_eq!(
            tag_lines! {
                [:0];
                [jump 0 "equal a a"];
            }.compile().unwrap(),
            ["jump 0 always 0 0"]
        );

        assert_eq!(
            tag_lines! {
                [:0];
                [jump 0 "strictEqual 0 0"];
            }.compile().unwrap(),
            ["jump 0 always 0 0"]
        );

        assert_eq!(
            tag_lines! {
                [:0];
                [jump 0 "strictEqual a a"];
            }.compile().unwrap(),
            ["jump 0 always 0 0"]
        );

        assert_eq!(
            tag_lines! {
                [:0];
                [jump 0 "lessThanEq a a"];
            }.compile().unwrap(),
            ["jump 0 always 0 0"]
        );

        assert_eq!(
            tag_lines! {
                [:0];
                [jump 0 "greaterThanEq a a"];
            }.compile().unwrap(),
            ["jump 0 always 0 0"]
        );

    }

    #[test]
    fn mdt_logic_split_test() {
        let datas: &[(&str, &[&str])] = &[
            (r#""#,                         &[]),
            (r#"       "#,                  &[]),
            (r#"abc def ghi"#,              &["abc", "def", "ghi"]),
            (r#"abc   def ghi"#,            &["abc", "def", "ghi"]),
            (r#"   abc   def ghi"#,         &["abc", "def", "ghi"]),
            (r#"   abc   def ghi  "#,       &["abc", "def", "ghi"]),
            (r#"abc   def ghi  "#,          &["abc", "def", "ghi"]),
            (r#"abc   "def ghi"  "#,        &["abc", "\"def ghi\""]),
            (r#"abc   "def ghi"  "#,        &["abc", "\"def ghi\""]),
            (r#"  abc "def ghi"  "#,        &["abc", "\"def ghi\""]),
            (r#"abc"#,                      &["abc"]),
            (r#"a"#,                        &["a"]),
            (r#"a b"#,                      &["a", "b"]),
            (r#"ab"cd"ef"#,                 &["ab", "\"cd\"", "ef"]),
            (r#"ab"cd"e"#,                  &["ab", "\"cd\"", "e"]),
            (r#"ab"cd""#,                   &["ab", "\"cd\""]),
            (r#"ab"cd" e"#,                 &["ab", "\"cd\"", "e"]),
            (r#"ab"cd"      e"#,            &["ab", "\"cd\"", "e"]),
            (r#"ab"cd"  "#,                 &["ab", "\"cd\""]),
            (r#""cd""#,                     &["\"cd\""]),
            (r#""cd"  "#,                   &["\"cd\""]),
            (r#"    "cd"  "#,               &["\"cd\""]),
            (r#"    "你好"  "#,             &["\"你好\""]),
            (r#"甲乙"丙丁"  "#,             &["甲乙", "\"丙丁\""]),
            (r#"张三 李四"#,                &["张三", "李四"]),
            (r#"    ""  "#,                 &["\"\""]),
        ];
        for &(src, args) in datas {
            assert_eq!(&mdt_logic_split(src).unwrap(), args);
        }

        // 未闭合字符串的测试
        let faileds: &[(&str, usize)] = &[
            (r#"ab""#, 3),
            (r#"ab ""#, 4),
            (r#"ab cd ""#, 7),
            (r#"""#, 1),
            (r#"     ""#, 6),
            (r#""     "#, 1),
            (r#""ab" "cd"  "e"#, 12),
            (r#"甲乙""#, 3),
        ];
        for &(src, char_num) in faileds {
            assert_eq!(mdt_logic_split(src).unwrap_err(), char_num, "{src:?}");
        }
    }
}
