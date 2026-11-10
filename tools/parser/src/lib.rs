mod parser;
pub use parser::*;
pub use ::lalrpop_util;
use std::iter::once;
use std::collections::HashSet;
use tag_code::logic_parser::IdxBox;

use lalrpop_util::ParseError;
use ::syntax::{
    Expand,
    Select,
    Goto,
    Var,
    Value,
    ValueBind,
    ValueBindRef,
    ValueBindRefTarget,
    LogicLine,
    InlineBlock,
    SwitchCatch,
    JumpCmp,
    CmpTree,
    Const,
    Take,
    ConstKey,
    DExp,
    Op,
    Args,
    impl_enum_froms,
    UNNAMED_VAR,
};

/// 进行`词法&语法`分析时所依赖的元数据
#[derive(Debug)]
pub struct Meta {
    tmp_var_count: usize,
    tag_number: usize,
    /// 被跳转的label
    defined_labels: Vec<HashSet<Var>>,
    break_labels: Vec<Option<Var>>,
    continue_labels: Vec<Option<Var>>,
    /// 用于op-expr的引用栈, 可以使用占位表达式引用最顶层的句柄
    op_expr_refs: Vec<Var>,
    unnamed_var: Var,
    line_pack: Vec<(Vec<LogicLine>, Vec<LogicLine>)>,
    pub testing: bool,
}
impl Default for Meta {
    fn default() -> Self {
        let unnamed_var = Var::from(UNNAMED_VAR);
        Self {
            tmp_var_count: 0,
            tag_number: 0,
            defined_labels: vec![HashSet::new()],
            break_labels: Vec::new(),
            continue_labels: Vec::new(),
            op_expr_refs: vec![unnamed_var.clone()],
            unnamed_var,
            line_pack: vec![],
            testing: false,
        }
    }
}
impl Meta {
    /// use [`Self::default()`]
    ///
    /// [`Self::default()`]: Self::default
    pub fn new() -> Self {
        Self::default()
    }

    /// 返回一个临时变量, 不会造成重复
    pub fn get_tmp_var(&mut self) -> Var {
        let var = self.tmp_var_count;
        self.tmp_var_count += 1;
        utils::gen_anon_name(var, |arg| format!("___{}", arg)).into()
    }

    /// 获取一个标签, 并且进行内部自增以保证不会获取到获取过的
    pub fn get_tag(&mut self) -> Var {
        let tag = self.tag_number;
        self.tag_number += 1;
        let fmtted_arg = utils::gen_anon_name(tag, |arg|
            format!("___{}", arg));
        self.add_defined_label(fmtted_arg.into())
    }

    /// 添加一个被跳转的label到当前作用域
    /// 使用克隆的形式
    pub fn add_defined_label(&mut self, label: Var) -> Var {
        // 至少有一个基层定义域
        self.defined_labels.last_mut().unwrap().insert(label.clone());
        label
    }

    /// 添加一个被跳转标签, 顺便向meta声明
    /// 拒绝裸创建Label, 因为会干扰常量被注册Label
    pub fn create_defined_label(&mut self, label: Var) -> LogicLine {
        LogicLine::Label(self.add_defined_label(label))
    }

    pub fn create_take_unused(&self, value: IdxBox<Value>) -> Take {
        let unused = value.new_value(self.unnamed_var());
        Take(ConstKey::Unused(unused), value.value)
    }

    /// 添加一个标签作用域,
    /// 用于const定义起始
    pub fn add_label_scope(&mut self) {
        self.defined_labels.push(HashSet::new())
    }

    /// 弹出一个标签作用域,
    /// 用于const定义完成收集信息
    pub fn pop_label_scope(&mut self) -> HashSet<Var> {
        self.defined_labels.pop().unwrap()
    }

    pub fn ext_label_scope(&mut self) {
        let scope = self.defined_labels.pop().unwrap();
        self.defined_labels.last_mut().unwrap()
            .extend(scope);
    }

    pub fn add_control_break_level(&mut self, r#break: Option<Var>) {
        self.break_labels.push(r#break);
    }

    pub fn add_control_continue_level(&mut self, r#continue: Option<Var>) {
        self.continue_labels.push(r#continue);
    }

    /// 添加一层用于`break`和`continue`的未使用控制层
    ///
    /// 需要在结构结束时将其销毁
    pub fn add_control_level(
        &mut self,
        r#break: Option<Var>,
        r#continue: Option<Var>,
    ) {
        self.add_control_break_level(r#break);
        self.add_control_continue_level(r#continue);
    }

    pub fn pop_control_break_level(&mut self) -> Option<Var> {
        self.break_labels.pop().unwrap()
    }

    pub fn pop_control_continue_level(&mut self) -> Option<Var> {
        self.continue_labels.pop().unwrap()
    }

    /// 将`break`和`continue`的标签返回
    ///
    /// 如果未使用那么返回的会为空
    pub fn pop_control_level(&mut self) -> (Option<Var>, Option<Var>) {
        (
            self.pop_control_break_level(),
            self.pop_control_continue_level(),
        )
    }

    /// 返回`break`的目标标签, 这会执行惰性初始化
    pub fn get_break(&mut self) -> &Var {
        // 由于设计上的懒惰与所有权系统的缺陷冲突, 所以这里的代码会略繁琐
        let new_lab
            = if self.break_labels.last().unwrap().is_none() {
                self.get_tag().into()
            } else {
                None
            };
        let label = self.break_labels.last_mut().unwrap();
        if let Some(new_lab) = new_lab {
            assert!(label.is_none());
            *label = Some(new_lab)
        }
        label.as_ref().unwrap()
    }

    /// 返回`continue`的目标标签, 这会执行惰性初始化
    pub fn get_continue(&mut self) -> &Var {
        // 由于设计上的懒惰与所有权系统的缺陷冲突, 所以这里的代码会略繁琐
        let new_lab
            = if self.continue_labels.last().unwrap().is_none() {
                self.get_tag().into()
            } else {
                None
            };
        let label = self.continue_labels.last_mut().unwrap();
        if let Some(new_lab) = new_lab {
            assert!(label.is_none());
            *label = Some(new_lab)
        }
        label.as_ref().unwrap()
    }

    pub fn push_some_label_to(
        &mut self,
        lines: &mut Vec<LogicLine>,
        label: Option<Var>,
    ) {
        if let Some(label) = label {
            lines.push(self.create_defined_label(label))
        }
    }

    /// 单纯的构建一个set语句
    pub fn build_set(var: Value, value: Value) -> LogicLine {
        LogicLine::Other(Args::Normal(vec![
                Value::ReprVar("set".into()),
                var,
                value,
        ]))
    }

    pub fn unnamed_var(&self) -> Var {
        self.unnamed_var.clone()
    }

    pub fn op_expr_ref(&self) -> Var {
        self.op_expr_refs
            .last()
            .cloned()
            .unwrap()
    }

    pub fn add_line_pack(&mut self) {
        self.line_pack.push((vec![], vec![]));
    }

    pub fn add_line_dep(&mut self, line: LogicLine) {
        let last = self.line_pack.last_mut().expect("add line dep to empty");
        last.0.push(line);
    }

    pub fn add_line_post(&mut self, line: LogicLine) {
        self.line_pack.last_mut().unwrap().1.push(line);
    }

    #[track_caller]
    pub fn pack_line(&mut self, line: LogicLine) -> LogicLine {
        let (deps, posts) = self.line_pack.pop().unwrap();
        if deps.is_empty() && posts.is_empty() {
            return line;
        }
        let packed = deps
            .into_iter()
            .chain(once(line))
            .chain(posts)
            .collect();
        InlineBlock(packed).into()
    }
}

fn trim_complete(v: &Var) -> Var {
    if let Some(real) = v.strip_suffix(syntax::LSP_DEBUG) {
        real.into()
    } else {
        v.clone()
    }
}

fn first_comp(values: &[Value]) -> bool {
    let Some(first) = values.first() else { return false };
    match first {
        Value::ValueBindRef(ValueBindRef { bind_target: ValueBindRefTarget::NameBind(name), .. }) |
        Value::ValueBind(ValueBind(_, name)) |
        Value::Var(name) => name.ends_with(syntax::LSP_DEBUG),
        _ => false,
    }
}

fn make_take_destructs(
    meta: &mut Meta,
    mut key: ConstKey,
    val: Value,
    de: Vec<(bool, Var, Var)>,
) -> LogicLine {
    let pre = if let ConstKey::ValueBind(ValueBind(binder, _)) = &mut key {
        let tmp = meta.get_tmp_var();
        let binder = core::mem::replace(&mut **binder, tmp.clone().into());
        Some(Take(tmp.into(), binder).into())
    } else {
        None
    };

    let lines = pre.into_iter()
        .chain(once(Take(key.clone(), val).into()))
        .chain(de.into_iter().map(|(c, dst, src)|
        {
            if c {
                Const(dst.into(), ValueBindRef::new(
                    Box::new(key.clone().into()),
                    ValueBindRefTarget::NameBind(src),
                ).into(), vec![]).into()
            } else {
                Take(dst.into(), ValueBind(
                    Box::new(key.clone().into()),
                    src,
                ).into()).into()
            }
        }))
        .collect();
    InlineBlock(lines).into()
}

fn make_skip(
    meta: &mut Meta,
    cmp: CmpTree,
    body: LogicLine,
) -> LogicLine {
    let lab = meta.get_tag();
    Expand(vec![
        Goto(lab.clone(), cmp).into(),
        body,
        meta.create_defined_label(lab),
    ]).into()
}

fn make_do_while(
    meta: &mut Meta,
    body: LogicLine,
    (break_lab, continue_lab): (Option<Var>, Option<Var>),
    cmp: CmpTree,
) -> LogicLine {
    let head = meta.get_tag();
    let mut res = Vec::with_capacity(5);

    res.extend([
        meta.create_defined_label(head.clone()),
        body,
    ]);
    meta.push_some_label_to(&mut res, continue_lab);
    res.push(Goto(head, cmp).into());
    meta.push_some_label_to(&mut res, break_lab);

    Expand(res).into()
}

fn make_while(
    meta: &mut Meta,
    cmp: CmpTree,
    body: LogicLine,
    (break_lab, continue_lab): (Option<Var>, Option<Var>),
) -> LogicLine {
    let [end, head] = [meta.get_tag(), meta.get_tag()];
    let rev_cmp = cmp.clone().reverse();
    let mut res = Vec::with_capacity(7);

    res.extend([
        Goto(end.clone(), rev_cmp).into(),
        meta.create_defined_label(head.clone()),
        body,
    ]);
    meta.push_some_label_to(&mut res, continue_lab);
    res.extend([
        Goto(head, cmp).into(),
        meta.create_defined_label(end),
    ]);
    meta.push_some_label_to(&mut res, break_lab);

    Expand(res).into()

}

fn make_gwhile(
    meta: &mut Meta,
    cmp: CmpTree,
    body: LogicLine,
    (break_lab, continue_lab): (Option<Var>, Option<Var>),
) -> LogicLine {
    let [to, head] = [meta.get_tag(), meta.get_tag()];
    let mut res = Vec::with_capacity(7);

    res.extend([
        Goto(to.clone(), JumpCmp::Always.into()).into(),
        meta.create_defined_label(head.clone()),
        body,
        meta.create_defined_label(to),
    ]);
    meta.push_some_label_to(&mut res, continue_lab);
    res.push(Goto(head, cmp).into());
    meta.push_some_label_to(&mut res, break_lab);

    Expand(res).into()
}

fn make_switch(
    meta: &mut Meta,
    value: Value,
    mut append: Vec<LogicLine>,
    catchs: Vec<(Vec<SwitchCatch>, Option<Var>, Expand)>,
    cases: Vec<(bool, Vec<usize>, Expand)>,
    (break_lab, continue_lab): (Option<Var>, Option<Var>),
) -> LogicLine {
    let catchs_is_empty = catchs.is_empty();

    let mut next_case_num = 0;
    let case_num_max = cases
        .iter()
        .map(
            |(_, nums, _)| {
                let num = nums
                    .iter()
                    .max()
                    .copied()
                    .unwrap_or(next_case_num);
                next_case_num = num + 1;
                num
            }
        )
        .max()
        .unwrap();

    // 用于填充填充case的行, 如果有追加在末尾的行则将其封装并替换填充
    let (mut fill_line, append) = match &append[..] {
        [] => (LogicLine::Ignore, None),
        [_] => (
            Expand(vec![append.last().unwrap().clone()]).into(),
            append.pop().unwrap().into(),
        ),
        [..] => (
            Expand(append.clone()).into(),
            Some(Expand(append).into()),
        ),
    };

    // 用于添加到头部的捕获块
    let mut catch_lines = Vec::new();
    let value_handle: Var = if catchs_is_empty {
        Var::new()
    } else { meta.get_tmp_var() };

    // 这里开始遍历捕获
    // 如果遇到了未命中捕获, 则改变fill_line为总是跳转到未命中捕获
    for (flags, name, lines) in catchs {
        let mut out_block = Vec::new();
        let skip_cmp = CmpTree::new_ands(
            flags
                .into_iter()
                .filter(|flag| {
                    if flag.is_misses() {
                        // 是一个未命中捕获
                        let tag = meta.get_tag();
                        out_block.push(meta.create_defined_label(tag.clone()));
                        fill_line = Goto(tag, JumpCmp::Always.into()).into();
                        false // 已处理, 过滤掉
                    } else {
                        true
                    }
                })
                .map(|flag|
                    flag.build(value_handle.as_str().into(), case_num_max)
                )
        ).unwrap_or(JumpCmp::Always.into());
        let skip_tag = meta.get_tag();
        out_block.insert(0, Goto(skip_tag.clone(), skip_cmp).into());
        if let Some(name) = name {
            // 如果有捕获变量则使用一个const进行映射
            // 这需要插入在头部, 也就是条件前
            // 防止`case (a) a:`时, a还没被const就进行了判断
            out_block.insert(
                0,
                Const::new(
                    name.into(),
                    value_handle.as_str().into()
                ).into()
            )
        }
        out_block.push(lines.into());
        out_block.push(meta.create_defined_label(skip_tag));

        catch_lines.push(Expand(out_block).into())
    }

    let mut cases_res = Vec::with_capacity(case_num_max + 1);
    let mut cases_res_isline = vec![false; case_num_max + 1];

    let mut next_ignored_num = 0;
    for (ignore_append, mut nums, mut expand) in cases {
        if let Some(append) = &append {
            if !ignore_append {
                expand.push(append.clone())
            }
        }
        if nums.is_empty() { nums.push(next_ignored_num) }
        for num in nums {
            for _ in cases_res.len()..=num {
                cases_res.push(LogicLine::Ignore);
            }
            cases_res[num] = expand.clone().into();
            cases_res_isline[num] = true;
            next_ignored_num = num + 1;
        }
    }
    // 将填充行填入填充case
    let mut iter = cases_res_isline.into_iter().enumerate().peekable();
    while let Some((idx, is_line)) = iter.next() {
        if is_line { continue }
        match iter.peek() {
            Some((_, true)) => cases_res[idx] = fill_line.clone(),
            _ => (),
        }
    }
    debug_assert_eq!(cases_res.len(), case_num_max + 1);
    debug_assert_eq!(cases_res.len(), cases_res.capacity());

    if catchs_is_empty {
        // 没有捕获块
        let mut res = Vec::with_capacity(3);

        meta.push_some_label_to(&mut res, continue_lab);
        res.push(Select(value, Expand(cases_res)).into());
        meta.push_some_label_to(&mut res, break_lab);

        if res.len() == 1 {
            res.pop().unwrap()
        } else {
            Expand(res).into()
        }
    } else {
        // 有捕获块
        // 保证我们拿到了一个临时返回句柄, 而不是一个空值
        assert_ne!(&value_handle, "");
        let mut res = Vec::with_capacity(5);

        meta.push_some_label_to(&mut res, continue_lab);
        res.extend([
            // 求值
            Take(value_handle.as_str().into(), value).into(),
            // 捕获
            Expand(catch_lines).into(),
            // 主体
            Select(value_handle.into(), Expand(cases_res)).into()
        ]);
        meta.push_some_label_to(&mut res, break_lab);

        Expand(res).into()
    }
}

fn make_select(
    meta: &mut Meta,
    value: Value,
    lines: Expand,
    (break_lab, continue_lab): (Option<Var>, Option<Var>),
) -> LogicLine {
    let mut res = Vec::with_capacity(3);

    meta.push_some_label_to(&mut res, continue_lab);
    res.push(Select(value, lines).into());
    meta.push_some_label_to(&mut res, break_lab);

    if res.len() == 1 {
        res.pop().unwrap()
    } else {
        Expand(res).into()
    }
}

fn get_locations<const N: usize>(src: &str, indexs: [usize; N]) -> [impl std::fmt::Display; N] {
    line_column::line_columns(src, indexs)
}

pub fn format_parse_err<const MAX_INVALID_TOKEN_VIEW: usize>(
    e: ParseError<usize, parser::Token<'_>, syntax::Error>,
    src: &str,
) -> String {
    use syntax::{Errors, Error};
    use std::ops::Deref;
    fn fmt_token<'a>(i: impl IntoIterator<Item = &'a str>)
    -> impl Iterator<Item = &'a str> {
        i.into_iter()
            .map(|s| get_token_name(s).unwrap_or(s))
    }
    match e {
        ParseError::UnrecognizedToken {
            token: (start, token, end),
            expected
        } => {
            let [start, end] = get_locations(src, [start, end]);
            format!(
                "在位置 {start} 至 {end} 处找到不应出现的令牌: {:?}\n\
                预期: [{}]",
                token.1,
                fmt_token(expected.iter().map(Deref::deref))
                    .collect::<Vec<_>>()
                    .join(", "),
            )
        },
        ParseError::ExtraToken { token: (start, token, end) } => {
            let [start, end] = get_locations(src, [start, end]);
            format!(
                "在位置 {start} 至 {end} 处找到多余的令牌: {:?}",
                fmt_token(Some(token.1)).next().unwrap(),
            )
        },
        ParseError::InvalidToken { location } => {
            let [loc] = get_locations(src, [location]);
            let view = &src[
                location
                ..
                src.len().min(
                    src[location..]
                        .char_indices()
                        .map(|(i, _ch)| location+i)
                        .take(MAX_INVALID_TOKEN_VIEW+1)
                        .last()
                        .unwrap_or(location))
            ];
            format!(
                "在位置 {loc} 处找到无效的令牌: {:?}",
                view.trim_end(),
            )
        },
        ParseError::UnrecognizedEof {
            location,
            expected
        } => {
            let [start] = get_locations(src, [location]);
            format!(
                "在位置 {start} 处意外的结束\n\
                预期: [{}]",
                fmt_token(expected.iter().map(Deref::deref))
                    .collect::<Vec<_>>()
                    .join(", "),
            )
        },
        ParseError::User {
            error: Error {
                start,
                end,
                err
            }
        } => {
            let [start, end]
                = get_locations(src, [start, end]);
            let out = |msg| format!(
                "在位置 {start} 至 {end} 处的错误:\n{}",
                msg
            );
            match err {
                Errors::NotALiteralUInteger(str, err) => {
                    out(format_args!(
                        "{:?} is not a valided unsigned integer: {}",
                        str,
                        err,
                    ))
                },
                Errors::UnpairBranches(left, right) => {
                    out(format_args!(
                        "unpair branches, left: {}, right: {}",
                        left,
                        right,
                    ))
                },
                Errors::OpExprInvalidResult { found, right } => {
                    out(format_args!(
                        "{} op-expr can't pattern {} results, expected 1 or {}",
                        right,
                        found,
                        found,
                    ))
                },
                Errors::MultipleOpExpr => {
                    out(format_args!(
                        "此处不应展开多个 op-expr",
                    ))
                },
                Errors::Emulate => {
                    out(format_args!(
                        "用于语言服务器的错误",
                    ))
                },
                #[allow(unreachable_patterns)]
                e => {
                    out(format_args!("未被枚举的错误: {:?}", e))
                },
            }
        },
    }
}

fn get_token_name(s: &str) -> Option<&'static str> {
    match s {
        r###"r#"[_\\p{XID_Start}]\\p{XID_Continue}*|__lsp_hover__@"#"###
            => "Identify",
        r###"r#"@[_\\p{XID_Start}][\\p{XID_Continue}\\-]*"#"###
            => "OIdentify",
        r###"r#"-?(?:0x-?[\\da-fA-F][_\\da-fA-F]*|0b-?[01][_01]*|\\d[_\\d]*(?:\\.\\d[\\d_]*|e[+\\-]?\\d[\\d_]*)?)"#"###
            => "Number",
        r###"r#"\"(?:\\\\\\r?\\n\\s*(?:\\\\ )?|\\r?\\n|\\\\(?:[n\"\\\\\\[]|u[0-9a-fA-F]{4})|[^\"\\r\\n\\\\])*\""#"###
            => "String",
        r###"r#"'[^'\\s]+'"#"###
            => "OtherVariable",
        _ => return None,
    }.into()
}

#[derive(Debug, Clone)]
pub struct OpExprInfo {
    pub exprs: Vec<OpExprType>,
}
impl From<OpExprInfo> for Vec<OpExprType> {
    fn from(value: OpExprInfo) -> Self {
        value.exprs
    }
}
impl From<OpExprType> for OpExprInfo {
    fn from(value: OpExprType) -> Self {
        Self { exprs: vec![value] }
    }
}
impl From<Value> for OpExprInfo {
    fn from(value: Value) -> Self {
        Self { exprs: vec![value.into()] }
    }
}
impl FromIterator<OpExprType> for OpExprInfo {
    fn from_iter<T: IntoIterator<Item = OpExprType>>(iter: T) -> Self {
        Self { exprs: iter.into_iter().collect() }
    }
}
impl FromIterator<OpExprInfo> for OpExprInfo {
    fn from_iter<T: IntoIterator<Item = OpExprInfo>>(iter: T) -> Self {
        Self { exprs: iter.into_iter().flat_map(|it| it.exprs).collect() }
    }
}
impl IntoIterator for OpExprInfo {
    type Item = OpExprType;
    type IntoIter = <Vec<OpExprType> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.exprs.into_iter()
    }
}
impl OpExprInfo {
    fn each<F, I>(mut self, f: F) -> Self
    where F: FnMut(OpExprType) -> I,
          I: IntoIterator<Item = OpExprType>,
    {
        self.exprs = self.exprs.into_iter()
            .flat_map(f)
            .collect();
        self
    }

    pub fn into_values(self, meta: &mut Meta) -> impl ExactSizeIterator<Item = Value> + use<'_> {
        self.exprs.into_iter()
            .map(|it| it.into_value(meta))
    }

    pub fn new_if_else(self, meta: &mut Meta, cmp: CmpTree, rhs: Self) -> Self {
        self.each(|lhs| rhs.clone().each(|rhs| {
            once(OpExprType::new_if_else(
                meta,
                cmp.clone(),
                lhs.clone(),
                rhs,
            ))
        }))
    }

    pub fn new_select(self, meta: &mut Meta, cond: JumpCmp, rhs: Self) -> Self {
        self.each(|lhs| rhs.clone().each(|rhs| {
            once(OpExprType::new_select(
                meta,
                cond.clone(),
                lhs.clone(),
                rhs,
            ))
        }))
    }

    pub fn new_op2<F>(self, rhs: Self, mut f: F) -> Self
    where F: FnMut(OpExprType, OpExprType) -> Op,
    {
        self.each(|lhs| rhs.clone().each(|rhs| {
            let op = f(lhs.clone(), rhs);
            once(OpExprType::Op(op))
        }))
    }

    pub fn new_op1<F>(self, mut f: F) -> Self
    where F: FnMut(OpExprType) -> Op,
    {
        self.each(|lhs| once(f(lhs).into()))
    }
}

#[derive(Debug, Clone)]
pub enum OpExprType {
    Value(Value),
    Op(Op),
    IfElse {
        child_result: Var,
        cmp: CmpTree,
        true_line: LogicLine,
        false_line: LogicLine,
    },
    Select {
        cond: JumpCmp,
        true_value: Value,
        false_value: Value,
    },
    SufSelfOp {
        op: Op,
        value: Value,
        ext: Option<Box<OpExprType>>,
        handle: Var,
    },
    Ref(Var),
}
impl OpExprType {
    pub fn new_if_else(
        meta: &mut Meta,
        cmp: CmpTree,
        true_line: Self,
        false_line: Self,
    ) -> Self {
        let result = meta.get_tmp_var();
        Self::IfElse {
            child_result: result.clone(),
            cmp,
            true_line: true_line.into_logic_line(meta, result.clone().into()),
            false_line: false_line.into_logic_line(meta, result.into()),
        }
    }

    pub fn new_select(
        meta: &mut Meta,
        cond: JumpCmp,
        true_value: Self,
        false_value: Self,
    ) -> Self {
        Self::Select {
            cond,
            true_value: true_value.into_value(meta),
            false_value: false_value.into_value(meta),
        }
    }

    pub fn new_suf_selfop(
        meta: &mut Meta,
        op: Op,
        value: Value,
        ext: Option<Box<OpExprType>>,
    ) -> Self {
        Self::SufSelfOp {
            op,
            value,
            ext,
            handle: meta.op_expr_refs.pop().unwrap(),
        }
    }

    pub fn add_suf_selfop_handle_to_meta(meta: &mut Meta) -> Var {
        let tmp = meta.get_tmp_var();
        meta.op_expr_refs.push(tmp.clone());
        tmp
    }

    pub fn into_value(self, meta: &mut Meta) -> Value {
        match self {
            Self::Op(op) => {
                assert!(op.get_result().is_result_handle());
                DExp::new_nores(vec![op.into()].into()).into()
            },
            Self::Value(value) => {
                value
            },
            Self::IfElse {
                child_result,
                cmp,
                true_line,
                false_line,
            } => {
                let (true_lab, skip_lab)
                    = (meta.get_tag(), meta.get_tag());
                DExp::new_nores(vec![
                    Take(child_result.into(), Value::ResultHandle(None)).into(),
                    Goto(true_lab.clone(), cmp).into(),
                    false_line,
                    Goto(skip_lab.clone(), JumpCmp::Always.into()).into(),
                    LogicLine::Label(true_lab),
                    true_line,
                    LogicLine::Label(skip_lab),
                ].into()).into()
            },
            Self::Select { cond, true_value, false_value } => {
                let cond = cond.do_start_compile_into();
                let cmp_str = cond.cmp_str();
                let (a, b) = cond.get_values();
                DExp::new_nores(vec![
                    LogicLine::Other(Args::Normal(vec![
                        Value::ReprVar("select".into()),
                        Value::ResultHandle(None),
                        Value::ReprVar(cmp_str.into()),
                        a,
                        b,
                        true_value,
                        false_value,
                    ]))
                ].into()).into()
            },
            Self::SufSelfOp {
                mut op,
                value,
                ext,
                handle,
            } => {
                let op_info = op.get_info_mut();
                *op_info.result = handle.clone().into();
                *op_info.arg1 = handle.clone().into();

                let set_line = ext
                    .map(|ext| *ext)
                    .unwrap_or_else(|| Self::Value(handle.clone().into()))
                    .into_logic_line(meta, Value::ResultHandle(None));

                DExp::new_nores(vec![
                    Take(handle.into(), value).into(),
                    set_line,
                    op.into(),
                ].into()).into()
            },
            Self::Ref(var) => var.into(),
        }
    }

    pub fn into_logic_line(self, meta: &mut Meta, result: Value) -> LogicLine {
        match self {
            Self::Op(mut op) => {
                assert!(op.get_result().is_result_handle());
                *op.get_result_mut() = result;
                op.into()
            },
            Self::Value(value) => {
                Meta::build_set(result, value)
            },
            Self::IfElse {
                child_result,
                cmp,
                true_line,
                false_line,
            } => {
                let (true_lab, skip_lab)
                    = (meta.get_tag(), meta.get_tag());
                Expand(vec![
                    Take(child_result.into(), result).into(),
                    Goto(true_lab.clone(), cmp).into(),
                    false_line,
                    Goto(skip_lab.clone(), JumpCmp::Always.into()).into(),
                    LogicLine::Label(true_lab),
                    true_line,
                    LogicLine::Label(skip_lab),
                ]).into()
            },
            Self::Select { cond, true_value, false_value } => {
                let cond = cond.do_start_compile_into();
                let cmp_str = cond.cmp_str();
                let (a, b) = cond.get_values();
                LogicLine::Other(Args::Normal(vec![
                    Value::ReprVar("select".into()),
                    result,
                    Value::ReprVar(cmp_str.into()),
                    a,
                    b,
                    true_value,
                    false_value,
                ]))
            },
            Self::SufSelfOp {
                mut op,
                value,
                ext,
                handle,
            } => {
                let op_info = op.get_info_mut();
                *op_info.result = handle.clone().into();
                *op_info.arg1 = handle.clone().into();

                let set_line = ext
                    .map(|ext| *ext)
                    .unwrap_or_else(|| Self::Value(handle.clone().into()))
                    .into_logic_line(meta, result);

                Expand(vec![
                    Take(handle.into(), value).into(),
                    set_line,
                    op.into(),
                ]).into()
            },
            Self::Ref(var) => {
                Self::Value(var.into())
                    .into_logic_line(meta, result)
            },
        }
    }
}
impl_enum_froms!(impl From for OpExprType {
    Value => Value;
    Value => Var;
    Op => Op;
});

pub type OpExprAOperFun = fn(&mut Meta, Value, OpExprType) -> LogicLine;

#[allow(clippy::ptr_eq)]
pub fn op_expr_build_results(
    meta: &mut Meta,
    mut results: Vec<Value>,
    mut values: Vec<OpExprType>,
    f: OpExprAOperFun,
) -> LogicLine {
    match (results.len(), values.len()) {
        e @ ((0, _) | (_, 0)) => unreachable!("len by zero, {e:?}"),
        (1, 1) => {
            let (result, value) = (
                results.pop().unwrap(),
                values.pop().unwrap(),
            );
            f(meta, result, value)
        },
        (len, 1)
        if f as *const () == op_expr_tools::TOP_ASSIGN_OPER as *const () => {
            let mut lines = Vec::with_capacity(len + 1);
            let value = values.pop().unwrap();
            let mut results = results.into_iter();
            let first_result_handle = meta.get_tmp_var();
            lines.push(Take(
                first_result_handle.clone().into(), results.next().unwrap()
            ).into());
            lines.push(value.into_logic_line(
                meta,
                first_result_handle.clone().into()
            ));
            for result in results {
                let value
                    = OpExprType::Value(first_result_handle.clone().into());
                lines.push(value.into_logic_line(meta, result.clone()))
            }
            assert_eq!(lines.len(), len + 1);
            Expand(lines).into()
        },
        (len, 1) => {
            let mut lines = Vec::with_capacity(len + 1);
            let value = values.pop().unwrap()
                .into_value(meta);
            let tmp = meta.get_tmp_var();
            lines.push(Take(tmp.clone().into(), value).into());
            for result in results {
                let line = f(meta, result, tmp.clone().into());
                lines.push(line);
            }
            Expand(lines).into()
        },
        (res_len, val_len) => {
            assert_eq!(res_len, val_len);

            let mut lines = Vec::with_capacity(res_len);
            let ziped
                = results.into_iter().zip(values);

            for (result, value) in ziped {
                let line = f(meta, result, value);
                lines.push(line)
            }
            Expand(lines).into()
        },
    }
}

#[macro_export]
macro_rules! make_assign_oper {
    ($oper:path) => {
        |meta, res, v| {
            let tres = meta.get_tmp_var();
            $crate::Expand(::std::vec![
                $crate::Take(tres.clone().into(), res).into(),
                $oper(
                    tres.clone().into(),
                    tres.into(),
                    v.into_value(meta),
                ).into(),
            ]).into()
        }
    };
}

/// if elif1 elif2 else -> gotos else elif2 elif1 if
pub fn make_if<I>(
    meta: &mut Meta,
    branches: I,
    else_: Option<LogicLine>,
) -> LogicLine
where I: IntoIterator<Item = (CmpTree, LogicLine)>,
      I::IntoIter: DoubleEndedIterator,
{
    let mut branches = branches.into_iter();

    let mut lines = vec![];
    let mut bodys = vec![];
    let end = meta.get_tag();
    let mut single = false;
    let mut single = |bodys: &mut Vec<LogicLine>| {
        if single {
            bodys.push(Goto(end.clone(), JumpCmp::Always.into()).into())
        }
        single = true;
    };

    let (last_elif_cmp, last_elif_body) = branches.next_back().unwrap();

    for (cmp, body) in branches {
        let tag = meta.get_tag();
        lines.push(Goto(tag.clone(), cmp).into());

        single(&mut bodys);
        bodys.push(body);
        bodys.push(LogicLine::Label(tag));
    }

    single(&mut bodys);
    if let Some(else_) = else_ {
        let lasttag = meta.get_tag();
        lines.push(Goto(lasttag.clone(), last_elif_cmp).into());

        bodys.push(last_elif_body);
        bodys.push(LogicLine::Label(lasttag));

        bodys.push(Goto(end.clone(), JumpCmp::Always.into()).into());
        bodys.push(else_);
    } else {
        bodys.push(last_elif_body);
        lines.push(Goto(end.clone(), last_elif_cmp.reverse()).into());
    }

    lines.extend(bodys.into_iter().rev());
    lines.push(LogicLine::Label(end));

    LogicLine::Expand(lines.into())
}

pub mod op_expr_tools {
    use super::{LogicLine, Meta, OpExprType, Value};

    pub const TOP_ASSIGN_OPER: fn(&mut Meta, Value, OpExprType) -> LogicLine
        = top_assign_oper;

    pub fn top_assign_oper(
        meta: &mut Meta,
        res: Value,
        value: OpExprType,
    ) -> LogicLine {
        value.into_logic_line(meta, res)
    }
}
