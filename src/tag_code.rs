pub type Tag = usize;
pub type TagsTable = Vec<usize>;
pub const UNINIT_TAG_TARGET: usize = usize::MAX;

/// 传入`TagsTable`, 生成逻辑代码
pub trait Compile {
    fn compile(self, tags_table: &TagsTable) -> String;
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
    fn compile(self, tags_table: &TagsTable) -> String {
        assert!(self.0 < tags_table.len()); // 越界检查
        assert_ne!(tags_table[self.0], UNINIT_TAG_TARGET); // 确保要跳转的目标有效
        format!("jump {} {}", tags_table[self.0], self.1)
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Line(pub Option<Tag>, pub String);
impl From<(Option<Tag>, String)> for Line {
    fn from((tag, value): (Option<Tag>, String)) -> Self {
        Self(tag, value)
    }
}
impl From<(Tag, &str)> for Line {
    fn from((tag, value): (Tag, &str)) -> Self {
        (Some(tag), value.into()).into()
    }
}
impl From<(Tag, String)> for Line {
    fn from((tag, value): (Tag, String)) -> Self {
        (Some(tag), value).into()
    }
}
impl From<String> for Line {
    fn from(value: String) -> Self {
        (None, value).into()
    }
}
impl From<&str> for Line {
    fn from(value: &str) -> Self {
        let value: String = value.into();
        value.into()
    }
}
impl Compile for Line {
    fn compile(self, _tags_table: &TagsTable) -> String {
        self.1
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum TagLine {
    Jump(Jump),
    Line(Line),
}
impl From<Jump> for TagLine {
    fn from(value: Jump) -> Self {
        Self::Jump(value)
    }
}
impl From<Line> for TagLine {
    fn from(value: Line) -> Self {
        Self::Line(value)
    }
}
impl Compile for TagLine {
    fn compile(self, tags_table: &TagsTable) -> String {
        match self {
            Self::Jump(jump) => jump.compile(tags_table),
            Self::Line(line) => line.compile(tags_table),
        }
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

    pub fn as_jump(&self) -> Option<&Jump> {
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

    pub fn as_line(&self) -> Option<&Line> {
        if let Self::Line(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct TagCodes {
    lines: Vec<TagLine>,
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

    pub fn compile(self) -> Vec<String> {
        let mut tags_table: TagsTable = TagsTable::new();
        for (num, code) in self.lines.iter().enumerate() {
            // 构建索引
            if let Some(line) = code.as_line() {
                let Some(tag) = line.0 else { continue };
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
        for line in self.lines {
            logic_lines.push(line.compile(&tags_table))
        }
        logic_lines
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jump_tag_test() {
        let mut codes = TagCodes::new();
        codes.push(Line::from((0, "op add a a 1")).into());
        codes.push(Jump::from((0, "always 0 0")).into());
        codes.push(Jump::from((2, "lessThan 0 1")).into());
        codes.push(Line::from("op add b b 1").into());
        codes.push(Line::from((2, "noop")).into());
        assert_eq!(codes.compile(), vec![
            "op add a a 1",
            "jump 0 always 0 0",
            "jump 4 lessThan 0 1",
            "op add b b 1",
            "noop",
        ]);
    }
}
