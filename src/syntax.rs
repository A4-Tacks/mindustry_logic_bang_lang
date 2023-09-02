use std::{
    ops::Deref,
    num::ParseIntError,
    collections::HashMap,
    iter::{
        zip,
        repeat_with,
    },
    process::exit,
    mem::replace,
    fmt::{Display, Debug},
};
use crate::tag_code::{
    Jump,
    TagCodes,
    TagLine
};
use display_source::{
    DisplaySource,
    DisplaySourceMeta,
};
pub use crate::tag_code::mdt_logic_split;


macro_rules! impl_enum_froms {
    (impl From for $ty:ty { $(
        $variant:ident => $target:ty;
    )* }) => { $(
        impl From<$target> for $ty {
            fn from(value: $target) -> Self {
                Self::$variant(value.into())
            }
        }
    )* };
}
macro_rules! impl_derefs {
    (impl $([$($t:tt)*])? for $ty:ty => ($self_:ident : $expr:expr): $res_ty:ty) => {
        impl $(<$($t)*>)? ::std::ops::Deref for $ty {
            type Target = $res_ty;

            fn deref(&$self_) -> &Self::Target {
                &$expr
            }
        }
        impl $(<$($t)*>)? ::std::ops::DerefMut for $ty {
            fn deref_mut(&mut $self_) -> &mut Self::Target {
                &mut $expr
            }
        }
    };
}
macro_rules! geter_builder {
    (
        $(
            $f_vis:vis fn $fname:ident($($paramtt:tt)*) -> $res_ty:ty ;
        )+
        $body:block
    ) => {
        $(
            $f_vis fn $fname($($paramtt)*) -> $res_ty $body
        )+
    };
}
/// 通过token匹配进行宏展开时的流分支
macro_rules! macro_if {
    (@yes ($($t:tt)*) else ($($f:tt)*)) => {
        $($t)*
    };
    (else ($($f:tt)*)) => {
        $($f)*
    };
}
macro_rules! do_return {
    ($e:expr $(=> $v:expr)?) => {
        if $e {
            return $($v)?
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Error {
    pub start: Location,
    pub end: Location,
    pub err: Errors,
}
impl From<(Location, Errors, Location)> for Error {
    fn from((start, err, end): (Location, Errors, Location)) -> Self {
        Self { start, end, err }
    }
}
impl From<([Location; 2], Errors)> for Error {
    fn from(([start, end], err): ([Location; 2], Errors)) -> Self {
        Self { start, end, err }
    }
}
impl From<((Location, Location), Errors)> for Error {
    fn from(((start, end), err): ((Location, Location), Errors)) -> Self {
        Self { start, end, err }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum Errors {
    NotALiteralUInteger(String, ParseIntError),
    SetVarNoPatternValue(usize, usize),
}

/// 带有错误前缀, 并且文本为红色的eprintln
macro_rules! err {
    ( $fmtter:expr $(, $args:expr)* $(,)? ) => {
        eprintln!(concat!("\x1b[1;31m", "CompileError:\n", $fmtter, "\x1b[0m"), $($args),*);
    };
}

pub type Var = String;
pub type Location = usize;
pub type Float = f64;

pub const COUNTER: &str = "@counter";
pub const FALSE_VAR: &str = "false";
pub const ZERO_VAR: &str = "0";
pub const UNUSED_VAR: &str = "0";

pub trait TakeHandle {
    /// 编译依赖并返回句柄
    fn take_handle(self, meta: &mut CompileMeta) -> Var;
}

impl TakeHandle for Var {
    fn take_handle(self, meta: &mut CompileMeta) -> Var {
        if let Some(value) = meta.const_expand_enter(&self) {
            // 是一个常量
            let res = if let Value::Var(var) = value {
                // 只进行单步常量追溯
                // 因为常量定义时已经完成了原有的多步
                var
            } else {
                value.take_handle(meta)
            };
            meta.const_expand_exit();
            res
        } else {
            self
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum Value {
    /// 一个普通值
    Var(Var),
    /// DExp
    DExp(DExp),
    /// 不可被常量替换的普通值
    ReprVar(Var),
    /// 编译时被替换为当前DExp返回句柄
    ResultHandle,
    ValueBind(ValueBind),
}
impl TakeHandle for Value {
    fn take_handle(self, meta: &mut CompileMeta) -> Var {
        // 改为使用空字符串代表空返回字符串
        // 如果空的返回字符串被编译将会被编译为tmp_var
        match self {
            Self::Var(var) => var.take_handle(meta),
            Self::DExp(dexp) => dexp.take_handle(meta),
            Self::ResultHandle => meta.dexp_handle().clone(),
            Self::ReprVar(var) => var,
            Self::ValueBind(val_bind) => val_bind.take_handle(meta),
        }
    }
}
impl DisplaySource for Value {
    fn display_source(&self, meta: &mut DisplaySourceMeta) {
        let replace_ident = Self::replace_ident;
        match self {
            Self::Var(s) => meta.push(&replace_ident(s)),
            Self::ReprVar(s) => meta.push(&format!("`{}`", replace_ident(s))),
            Self::ResultHandle => meta.push("$"),
            Self::DExp(dexp) => dexp.display_source(meta),
            Self::ValueBind(value_attr) => value_attr.display_source(meta),
        }
    }
}
impl Default for Value {
    /// 默认的占位值, 它是无副作用的, 不会被常量展开
    fn default() -> Self {
        Self::new_noeffect()
    }
}
impl Value {
    /// 新建一个占位符, 使用[`ReprVar`],
    /// 以保证它是真的不会有副作用的占位符
    pub fn new_noeffect() -> Self {
        Self::ReprVar(ZERO_VAR.into())
    }

    /// Returns `true` if the value is [`DExp`].
    ///
    /// [`DExp`]: Value::DExp
    #[must_use]
    pub fn is_dexp(&self) -> bool {
        matches!(self, Self::DExp(..))
    }

    pub fn as_dexp(&self) -> Option<&DExp> {
        if let Self::DExp(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn as_dexp_mut(&mut self) -> Option<&mut DExp> {
        if let Self::DExp(v) = self {
            Some(v)
        } else {
            None
        }
    }

    /// Returns `true` if the value is [`Var`].
    ///
    /// [`Var`]: Value::Var
    #[must_use]
    pub fn is_var(&self) -> bool {
        matches!(self, Self::Var(..))
    }

    pub fn as_var(&self) -> Option<&Var> {
        if let Self::Var(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn is_string(s: &str) -> bool {
        s.len() >= 2
            && s.starts_with('"')
            && s.ends_with('"')
    }

    /// 判断是否不应该由原始标识符包裹
    /// 注意是原始标识符(原始字面量), 不要与原始值混淆
    pub fn no_use_repr_var(s: &str) -> bool {
        Self::is_string(s)
    }

    /// 返回被规范化的标识符
    pub fn replace_ident(s: &str) -> String {
        if Self::no_use_repr_var(s) {
            s.into()
        } else {
            let var = s.replace('\'', "\"");
            format!("'{}'", var)
        }
    }

    /// Returns `true` if the value is [`ReprVar`].
    ///
    /// [`ReprVar`]: Value::ReprVar
    #[must_use]
    pub fn is_repr_var(&self) -> bool {
        matches!(self, Self::ReprVar(..))
    }

    pub fn as_repr_var(&self) -> Option<&Var> {
        if let Self::ReprVar(v) = self {
            Some(v)
        } else {
            None
        }
    }

    /// Returns `true` if the value is [`ResultHandle`].
    ///
    /// [`ResultHandle`]: Value::ResultHandle
    #[must_use]
    pub fn is_result_handle(&self) -> bool {
        matches!(self, Self::ResultHandle)
    }
}
impl Deref for Value {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Var(ref s) | Self::ReprVar(ref s) => s,
            Self::DExp(DExp { result, .. }) => &result,
            Self::ResultHandle =>
                panic!("未进行AST编译, 而DExp的返回句柄是进行AST编译时已知"),
            Self::ValueBind(..) =>
                panic!("未进行AST编译, 而ValueAttr的返回句柄是进行AST编译时已知"),
        }
    }
}
impl_enum_froms!(impl From for Value {
    Var => Var;
    Var => &str;
    DExp => DExp;
    ValueBind => ValueBind;
});

/// 带返回值的表达式
/// 其依赖被计算完毕后, 句柄有效
#[derive(Debug, PartialEq, Clone)]
pub struct DExp {
    result: Var,
    lines: Expand,
}
impl DExp {
    pub fn new(result: Var, lines: Expand) -> Self {
        Self { result, lines }
    }

    /// 新建一个可能指定返回值的DExp
    pub fn new_optional_res(result: Option<Var>, lines: Expand) -> Self {
        Self {
            result: result.unwrap_or_default(),
            lines
        }
    }

    /// 新建一个未指定返回值的DExp
    pub fn new_nores(lines: Expand) -> Self {
        Self {
            result: Default::default(),
            lines
        }
    }

    pub fn result(&self) -> &str {
        self.result.as_ref()
    }

    pub fn lines(&self) -> &Expand {
        &self.lines
    }

    pub fn lines_mut(&mut self) -> &mut Expand {
        &mut self.lines
    }
}
impl TakeHandle for DExp {
    fn take_handle(self, meta: &mut CompileMeta) -> Var {
        let DExp { mut result, lines } = self;

        if result.is_empty() {
            result = meta.get_tmp_var(); /* init tmp_var */
        } else if let
            Some((_, value))
                = meta.get_const_value(&result) {
            // 对返回句柄使用常量值的处理
            if let Some(dexp) = value.as_dexp() {
                err!(
                    concat!(
                        "{}\n尝试在`DExp`的返回句柄处使用值为`DExp`的const, ",
                        "此处仅允许使用`Var`\n",
                        "DExp: {:?}\n",
                        "名称: {:?}",
                    ),
                    meta.err_info().join("\n"),
                    dexp,
                    result
                );
                exit(5);
            }
            assert!(value.is_var());
            result = value.as_var().unwrap().clone()
        }
        assert!(! result.is_empty());
        meta.push_dexp_handle(result);
        lines.compile(meta);
        let result = meta.pop_dexp_handle();
        result
    }
}
impl DisplaySource for DExp {
    fn display_source(&self, meta: &mut DisplaySourceMeta) {
        meta.push("(");
        let has_named_res = !self.result.is_empty();
        if has_named_res {
            meta.push(&Value::replace_ident(&self.result));
            meta.push(":");
        }
        match self.lines.len() {
            0 => (),
            1 => {
                if has_named_res {
                    meta.add_space();
                }
                self.lines[0].display_source(meta);
            },
            _ => {
                meta.add_lf();
                meta.do_block(|meta| {
                    self.lines.display_source(meta);
                });
            }
        }
        meta.push(")");
    }
}
impl_derefs!(impl for DExp => (self: self.lines): Expand);

/// 将一个Value与一个Var以特定格式组合起来,
/// 可完成如属性调用的功能
#[derive(Debug, PartialEq, Clone)]
pub struct ValueBind(pub Box<Value>, pub Var);
impl TakeHandle for ValueBind {
    fn take_handle(self, meta: &mut CompileMeta) -> Var {
        // 以`__{}__bind__{}`的形式组合
        let handle = self.0.take_handle(meta);
        assert!(! Value::is_string(&self.1));
        if Value::is_string(&handle) {
            err!(
                "{}\nValueBind过程中, 左值句柄为字符串, ({}.{})",
                meta.err_info().join("\n"),
                Value::replace_ident(&handle),
                Value::replace_ident(&self.1),
            );
            exit(6)
        }
        format!("__{}__bind__{}", handle, self.1)
    }
}
impl DisplaySource for ValueBind {
    fn display_source(&self, meta: &mut DisplaySourceMeta) {
        self.0.display_source(meta);
        meta.push(".");
        meta.push(&Value::replace_ident(&self.1));
    }
}

/// 进行`词法&语法`分析时所依赖的元数据
#[derive(Debug)]
pub struct Meta {
    tmp_var_count: usize,
    tag_number: usize,
    /// 被跳转的label
    defined_labels: Vec<Vec<Var>>,
    break_labels: Vec<Option<Var>>,
    continue_labels: Vec<Option<Var>>,
}
impl Default for Meta {
    fn default() -> Self {
        Self {
            tmp_var_count: 0,
            tag_number: 0,
            defined_labels: vec![Vec::new()],
            break_labels: Vec::new(),
            continue_labels: Vec::new(),
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
        self.tmp_var_count+= 1;
        format!("___{}", var)
    }

    /// 获取一个标签, 并且进行内部自增以保证不会获取到获取过的
    pub fn get_tag(&mut self) -> String {
        let tag = self.tag_number;
        self.tag_number += 1;
        format!("___{}", tag)
    }

    /// 添加一个被跳转的label到当前作用域
    /// 使用克隆的形式
    pub fn add_defined_label(&mut self, label: Var) -> Var {
        // 至少有一个基层定义域
        self.defined_labels.last_mut().unwrap().push(label.clone());
        label
    }

    /// 添加一个标签作用域,
    /// 用于const定义起始
    pub fn add_label_scope(&mut self) {
        self.defined_labels.push(Vec::new())
    }

    /// 弹出一个标签作用域,
    /// 用于const定义完成收集信息
    pub fn pop_label_scope(&mut self) -> Vec<Var> {
        self.defined_labels.pop().unwrap()
    }

    /// 添加一层用于`break`和`continue`的未使用控制层
    ///
    /// 需要在结构结束时将其销毁
    pub fn add_control_level(
        &mut self, r#break: Option<Var>,
        r#continue: Option<Var>,
    ) {
        self.break_labels.push(r#break);
        self.continue_labels.push(r#continue);
    }

    /// 将`break`和`continue`的标签返回
    ///
    /// 如果未使用那么返回的会为空
    pub fn pop_control_level(&mut self) -> (Option<Var>, Option<Var>) {
        (
            self.break_labels.pop().unwrap(),
            self.continue_labels.pop().unwrap(),
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
            lines.push(LogicLine::new_label(label, self))
        }
    }

    /// 构建一个`sets`, 例如`a b c = 1 2 3;`
    /// 如果只有一个值与被赋值则与之前行为一致
    /// 如果值与被赋值数量不匹配则返回错误
    /// 如果值与被赋值数量匹配且大于一对就返回Expand中多个set
    pub fn build_sets(loc: [Location; 2], mut vars: Vec<Value>, mut values: Vec<Value>)
    -> Result<LogicLine, Error> {
        let build_set = Self::build_set;
        if vars.len() != values.len() {
            // 接受与值数量不匹配
            return Err((
                loc,
                Errors::SetVarNoPatternValue(vars.len(), values.len())
            ).into());
        }

        assert_ne!(vars.len(), 0);
        let len = vars.len();

        if len == 1 {
            // normal
            Ok(build_set(vars.pop().unwrap(), values.pop().unwrap()))
        } else {
            // sets
            let mut expand = Vec::with_capacity(len);
            expand.extend(zip(vars, values)
                .map(|(var, value)| build_set(var, value))
            );
            debug_assert_eq!(expand.len(), len);
            Ok(Expand(expand).into())
        }
    }

    /// 单纯的构建一个set语句
    pub fn build_set(var: Value, value: Value) -> LogicLine {
        LogicLine::Other(vec![
                Value::ReprVar("set".into()),
                var,
                value,
        ])
    }
}

pub trait FromMdtArgs
where Self: Sized
{
    type Err;

    /// 从逻辑参数构建
    fn from_mdt_args(args: &[&str]) -> Result<Self, Self::Err>;
}

/// `jump`可用判断条件枚举
#[derive(Debug, PartialEq, Clone)]
pub enum JumpCmp {
    Equal(Value, Value),
    NotEqual(Value, Value),
    LessThan(Value, Value),
    LessThanEq(Value, Value),
    GreaterThan(Value, Value),
    GreaterThanEq(Value, Value),
    /// 严格相等
    StrictEqual(Value, Value),
    /// 严格不相等
    StrictNotEqual(Value, Value),
    /// 总是
    Always,
    /// 总不是
    NotAlways,
}
impl JumpCmp {
    /// 将值转为`bool`来对待
    pub fn bool(val: Value) -> Self {
        Self::NotEqual(val, Value::ReprVar(FALSE_VAR.into()))
    }

    /// 获取反转后的条件
    pub fn reverse(self) -> Self {
        use JumpCmp::*;

        match self {
            Equal(a, b) => NotEqual(a, b),
            NotEqual(a, b) => Equal(a, b),
            LessThan(a, b) => GreaterThanEq(a, b),
            LessThanEq(a, b) => GreaterThan(a, b),
            GreaterThan(a, b) => LessThanEq(a, b),
            GreaterThanEq(a, b) => LessThan(a, b),
            StrictEqual(a, b) => StrictNotEqual(a, b),
            StrictNotEqual(a, b) => StrictEqual(a, b),
            Always => NotAlways,
            NotAlways => Always,
        }
    }

    /// 获取两个运算成员, 如果是没有运算成员的则返回[`Default`]
    pub fn get_values(self) -> (Value, Value) {
        match self {
            | Self::Equal(a, b)
            | Self::NotEqual(a, b)
            | Self::LessThan(a, b)
            | Self::LessThanEq(a, b)
            | Self::GreaterThan(a, b)
            | Self::StrictEqual(a, b)
            | Self::GreaterThanEq(a, b)
            | Self::StrictNotEqual(a, b)
            => (a, b),
            | Self::Always
            | Self::NotAlways
            // 这里使用default生成无副作用的占位值
            => Default::default(),
        }
    }

    /// 获取两个运算成员, 如果是没有运算成员的则返回空
    pub fn get_values_ref(&self) -> Option<(&Value, &Value)> {
        match self {
            | Self::Equal(a, b)
            | Self::NotEqual(a, b)
            | Self::LessThan(a, b)
            | Self::LessThanEq(a, b)
            | Self::GreaterThan(a, b)
            | Self::StrictEqual(a, b)
            | Self::GreaterThanEq(a, b)
            | Self::StrictNotEqual(a, b)
            => Some((a, b)),
            | Self::Always
            | Self::NotAlways
            => None,
        }
    }

    /// 获取需要生成的变体所对应的文本
    /// 如果是未真正对应的变体如严格不等则恐慌
    pub fn cmp_str(&self) -> &'static str {
        macro_rules! e {
            () => {
                panic!("这个变体并未对应最终生成的代码")
            };
        }
        macro_rules! build_match {
            ( $( $name:ident , $str:expr ),* $(,)? ) => {
                match self {
                    $(
                        Self::$name(..) => $str,
                    )*
                    Self::Always => "always",
                    Self::NotAlways => e!(),
                }
            };
        }

        build_match! {
            Equal, "equal",
            NotEqual, "notEqual",
            LessThan, "lessThan",
            LessThanEq, "lessThanEq",
            GreaterThan, "greaterThan",
            GreaterThanEq, "greaterThanEq",
            StrictEqual, "strictEqual",
            StrictNotEqual, e!()
        }
    }

    /// 构建两个值后将句柄送出
    pub fn build_value(self, meta: &mut CompileMeta) -> (Var, Var) {
        let (a, b) = self.get_values();
        (a.take_handle(meta), b.take_handle(meta))
    }

    /// 即将编译时调用, 将自身转换为可以正常编译为逻辑的形式
    /// 例如`严格不等`, `永不`等变体是无法直接被编译为逻辑的
    /// 所以需要进行转换
    pub fn do_start_compile_into(mut self) -> Self {
        self.inline_cmp_op();
        match self {
            // 转换为0永不等于0
            // 要防止0被const, 我们使用repr
            Self::NotAlways => {
                Self::NotEqual(Value::new_noeffect(), Value::new_noeffect())
            },
            Self::StrictNotEqual(a, b) => {
                Self::bool(
                    DExp::new_nores(vec![
                        Op::StrictEqual(Value::ResultHandle, a, b).into()
                    ].into()).into()
                ).reverse()
            },
            // 无需做转换
            cmp => cmp,
        }
    }

    /// 获取运算符号
    pub fn get_symbol_cmp_str(&self) -> &'static str {
        macro_rules! build_match {
            ( $( $name:ident , $str:expr ),* $(,)? ) => {
                match self {
                    $(
                        Self::$name(..) => $str,
                    )*
                    Self::Always => "_",
                    Self::NotAlways => "!_",
                }
            };
        }

        build_match! {
            Equal, "==",
            NotEqual, "!=",
            LessThan, "<",
            LessThanEq, "<=",
            GreaterThan, ">",
            GreaterThanEq, ">=",
            StrictEqual, "===",
            StrictNotEqual, "!==",
        }
    }

    pub fn inline_cmp_op(&mut self) {
        use {
            JumpCmp as JC,
            Value as V,
            LogicLine as LL,
        };
        let (dexp, invert) = match self {
            | JC::Equal(V::DExp(dexp), V::ReprVar(s))
            | JC::Equal(V::ReprVar(s), V::DExp(dexp))
            if dexp.result.is_empty() && s == FALSE_VAR
            => {
                (dexp.lines_mut(), true)
            }
            | JC::NotEqual(V::DExp(dexp), V::ReprVar(s))
            | JC::NotEqual(V::ReprVar(s), V::DExp(dexp))
            if dexp.result.is_empty() && s == FALSE_VAR
            => {
                (dexp.lines_mut(), false)
            }
            _ => return,
        };
        if dexp.0.len() != 1 { return }
        let LL::Op(op) = dexp.0.get_mut(0).unwrap() else { return };
        let do_reverse = |x: JC| if invert {
            x.reverse()
        } else {
            x
        };
        // 获取some后已经不可以中途返回了,
        // 因为根据try_into_cmp的约定, op已经被破坏, 不可再使用
        let Some(cmp) = op.try_into_cmp() else { return };
        *self = do_reverse(cmp);
        self.inline_cmp_op(); // 尝试继续内联
    }

}
impl DisplaySource for JumpCmp {
    fn display_source(&self, meta: &mut DisplaySourceMeta) {
        if let Self::Always | Self::NotAlways = self {
            meta.push(self.get_symbol_cmp_str())
        } else {
            let sym = self.get_symbol_cmp_str();
            let (a, b) = self.get_values_ref().unwrap();
            a.display_source(meta);
            meta.add_space();
            meta.push(sym);
            meta.add_space();
            b.display_source(meta);
        }
    }
}
impl FromMdtArgs for JumpCmp {
    type Err = LogicLineFromTagError;

    fn from_mdt_args(args: &[&str]) -> Result<Self, Self::Err> {
        let &[oper, a, b] = args else {
            return Err(JumpCmpRParseError::ArgsCountError(
                args.into_iter().cloned().map(Into::into).collect()
            ).into());
        };

        macro_rules! build_match {
            ( $( $name:ident , $str:pat ),* $(,)? ) => {
                match oper {
                    $(
                        $str => Ok(Self::$name(a.into(), b.into())),
                    )*
                    "always" => Ok(Self::Always),
                    cmper => {
                        Err(JumpCmpRParseError::UnknownComparer(
                            cmper.into(),
                            [a.into(), b.into()]
                        ).into())
                    },
                }
            };
        }

        build_match! {
            Equal, "equal",
            NotEqual, "notEqual",
            LessThan, "lessThan",
            LessThanEq, "lessThanEq",
            GreaterThan, "greaterThan",
            GreaterThanEq, "greaterThanEq",
            StrictEqual, "strictEqual",
        }
    }
}

/// JumpCmp语法树从字符串生成时的错误
#[derive(Debug, PartialEq, Clone)]
pub enum JumpCmpRParseError {
    ArgsCountError(Vec<String>),
    UnknownComparer(String, [String; 2]),
}
impl Display for JumpCmpRParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use JumpCmpRParseError::*;

        match self {
            ArgsCountError(args) => write!(
                f,
                "参数数量错误, 预期3个参数, 得到{}个参数: {:?}",
                args.len(),
                args
            ),
            UnknownComparer(oper, [a, b]) => write!(
                f,
                "未知的比较符: {:?}, 参数为: {:?}",
                oper,
                (a, b)
            ),
        }
    }
}

/// Op语法树从字符串生成时的错误
#[derive(Debug, PartialEq, Clone)]
pub enum OpRParseError {
    ArgsCountError(Vec<String>),
    UnknownOper(String, [String; 2]),
}
impl Display for OpRParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use OpRParseError::*;

        match self {
            ArgsCountError(args) => write!(
                f,
                "参数数量错误, 预期4个参数, 得到{}个参数: {:?}",
                args.len(),
                args
            ),
            UnknownOper(oper, [a, b]) => write!(
                f,
                "未知的运算符: {:?}, 参数为: {:?}",
                oper,
                (a, b)
            ),
        }
    }
}

/// LogicLine语法树从Tag码生成时的错误
/// 注意是从逻辑码而不是源码
#[derive(Debug, PartialEq, Clone)]
pub enum LogicLineFromTagError {
    JumpCmpRParseError(JumpCmpRParseError),
    OpRParseError(OpRParseError),
    StringNoStop {
        str: String,
        char_num: usize,
    },
}
impl Display for LogicLineFromTagError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::JumpCmpRParseError(e) =>
                Display::fmt(&e, f),
            Self::OpRParseError(e) =>
                Display::fmt(&e, f),
            Self::StringNoStop { str, char_num } => {
                write!(
                    f,
                    "未闭合的字符串, 在第{}个字符处起始, 行:[{}]",
                    char_num,
                    str
                )
            },
        }
    }
}
impl_enum_froms!(impl From for LogicLineFromTagError {
    JumpCmpRParseError => JumpCmpRParseError;
    OpRParseError => OpRParseError;
});

/// 一颗比较树,
/// 用于多条件判断.
/// 例如: `a < b && c < d || e == f`
#[derive(Debug, PartialEq, Clone)]
pub enum CmpTree {
    And(Box<Self>, Box<Self>),
    Or(Box<Self>, Box<Self>),
    Atom(JumpCmp),
}
impl CmpTree {
    /// 反转自身条件,
    /// 使用`德•摩根定律`进行表达式变换.
    ///
    /// 即`!(a&&b) == (!a)||(!b)`和`!(a||b) == (!a)&&(!b)`
    ///
    /// 例如表达式`(a && b) || c`进行反转变换
    /// 1. `!((a && b) || c)`
    /// 2. `!(a && b) && !c`
    /// 3. `(!a || !b) && !c`
    pub fn reverse(self) -> Self {
        match self {
            Self::Or(a, b)
                => Self::And(a.reverse().into(), b.reverse().into()),
            Self::And(a, b)
                => Self::Or(a.reverse().into(), b.reverse().into()),
            Self::Atom(cmp)
                => Self::Atom(cmp.reverse()),
        }
    }

    /// 构建条件树为goto
    pub fn build(self, meta: &mut CompileMeta, do_tag: Var) {
        use CmpTree::*;

        // 获取如果在常量展开内则被重命名后的标签
        let do_tag_expanded = meta.get_in_const_label(do_tag);

        match self {
            Or(a, b) => {
                a.build(meta, do_tag_expanded.clone());
                b.build(meta, do_tag_expanded);
            },
            And(a, b) => {
                let end = meta.get_tmp_tag();
                a.reverse().build(meta, end.clone());
                b.build(meta, do_tag_expanded);
                let tag_id = meta.get_tag(end);
                meta.push(TagLine::TagDown(tag_id));
            },
            Atom(cmp) => {
                // 构建为可以进行接下来的编译的形式
                let cmp = cmp.do_start_compile_into();

                let cmp_str = cmp.cmp_str();
                let (a, b) = cmp.build_value(meta);

                let jump = Jump(
                    meta.get_tag(do_tag_expanded).into(),
                    format!("{} {} {}", cmp_str, a, b)
                );
                meta.push(jump.into())
            },
        }
    }

    /// 以全部or组织一个条件树
    /// 是左至右结合的, 也就是说输入`[a, b, c]`会得到`(a || b) || c`
    /// 如果给出的条件个数为零则返回空
    pub fn new_ors<I>(iter: impl IntoIterator<IntoIter = I>) -> Option<Self>
    where I: Iterator<Item = Self>
    {
        let mut iter = iter.into_iter();
        let mut root = iter.next()?;

        for cmp in iter {
            root = Self::Or(root.into(), cmp.into())
        }

        root.into()
    }

    /// 以全部and组织一个条件树
    /// 是左至右结合的, 也就是说输入`[a, b, c]`会得到`(a && b) && c`
    /// 如果给出的条件个数为零则返回空
    pub fn new_ands<I>(iter: impl IntoIterator<IntoIter = I>) -> Option<Self>
    where I: Iterator<Item = Self>
    {
        let mut iter = iter.into_iter();
        let mut root = iter.next()?;

        for cmp in iter {
            root = Self::And(root.into(), cmp.into())
        }

        root.into()
    }
}
impl_enum_froms!(impl From for CmpTree {
    Atom => JumpCmp;
});
impl DisplaySource for CmpTree {
    fn display_source(&self, meta: &mut DisplaySourceMeta) {
        match self {
            Self::Atom(cmp) => cmp.display_source(meta),
            Self::Or(a, b) => {
                meta.push("(");
                a.display_source(meta);
                meta.add_space();
                meta.push("||");
                meta.add_space();
                b.display_source(meta);
                meta.push(")");
            },
            Self::And(a, b) => {
                meta.push("(");
                a.display_source(meta);
                meta.add_space();
                meta.push("&&");
                meta.add_space();
                b.display_source(meta);
                meta.push(")");
            }
        }
    }
}

/// 用于承载Op信息的容器
pub struct OpInfo<Arg> {
    pub oper_str: &'static str,
    pub oper_sym: Option<&'static str>,
    pub result: Arg,
    pub arg1: Arg,
    pub arg2: Option<Arg>,
}
impl<Arg> OpInfo<Arg> {
    pub fn new(
        oper_str: &'static str,
        oper_sym: Option<&'static str>,
        result: Arg,
        arg1: Arg,
        arg2: Option<Arg>,
    ) -> Self {
        Self {
            oper_str,
            oper_sym,
            result,
            arg1,
            arg2
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum Op {
    Add(Value, Value, Value),
    Sub(Value, Value, Value),
    Mul(Value, Value, Value),
    Div(Value, Value, Value),
    Idiv(Value, Value, Value),
    Mod(Value, Value, Value),
    Pow(Value, Value, Value),
    Equal(Value, Value, Value),
    NotEqual(Value, Value, Value),
    Land(Value, Value, Value),
    LessThan(Value, Value, Value),
    LessThanEq(Value, Value, Value),
    GreaterThan(Value, Value, Value),
    GreaterThanEq(Value, Value, Value),
    StrictEqual(Value, Value, Value),
    Shl(Value, Value, Value),
    Shr(Value, Value, Value),
    Or(Value, Value, Value),
    And(Value, Value, Value),
    Xor(Value, Value, Value),
    Max(Value, Value, Value),
    Min(Value, Value, Value),
    Angle(Value, Value, Value),
    Len(Value, Value, Value),
    Noise(Value, Value, Value),

    Not(Value, Value),
    Abs(Value, Value),
    Log(Value, Value),
    Log10(Value, Value),
    Floor(Value, Value),
    Ceil(Value, Value),
    Sqrt(Value, Value),
    Rand(Value, Value),
    Sin(Value, Value),
    Cos(Value, Value),
    Tan(Value, Value),
    Asin(Value, Value),
    Acos(Value, Value),
    Atan(Value, Value),
}
impl Op {
    geter_builder! {
        pub fn get_info(&self) -> OpInfo<&Value>;
        pub fn get_info_mut(&mut self) -> OpInfo<&mut Value>;
        pub fn into_info(self) -> OpInfo<Value>;
        {
            macro_rules! build_match {
                {
                    op1: [
                        $(
                            $oper1:ident =>
                                $oper1_str:literal
                                $($oper1_sym:literal)?
                        ),* $(,)?
                    ],
                    op2: [
                        $(
                            $oper2:ident =>
                                $oper2_str:literal
                                $($oper2_sym:literal)?
                        ),* $(,)?
                    ] $(,)?
                } => {
                    match self {
                        $(
                            Self::$oper1(result, a) => OpInfo::new(
                                $oper1_str,
                                macro_if!($(@yes (Some($oper1_sym)))? else (None)),
                                result,
                                a,
                                None,
                            ),
                        )*
                        $(
                            Self::$oper2(result, a, b) => OpInfo::new(
                                $oper2_str,
                                macro_if!($(@yes (Some($oper2_sym)))? else (None)),
                                result,
                                a,
                                b.into(),
                            ),
                        )*
                    }
                };
            }
            build_match! {
                op1: [
                    Not => "not" "~",
                    Abs => "abs",
                    Log => "log",
                    Log10 => "log10",
                    Floor => "floor",
                    Ceil => "ceil",
                    Sqrt => "sqrt",
                    Rand => "rand",
                    Sin => "sin",
                    Cos => "cos",
                    Tan => "tan",
                    Asin => "asin",
                    Acos => "acos",
                    Atan => "atan",
                ],
                op2: [
                    Add => "add" "+",
                    Sub => "sub" "-",
                    Mul => "mul" "*",
                    Div => "div" "/",
                    Idiv => "idiv" "//",
                    Mod => "mod" "%",
                    Pow => "pow" "**",
                    Equal => "equal" "==",
                    NotEqual => "notEqual" "!=",
                    Land => "land" "&&",
                    LessThan => "lessThan" "<",
                    LessThanEq => "lessThanEq" "<=",
                    GreaterThan => "greaterThan" ">",
                    GreaterThanEq => "greaterThanEq" ">=",
                    StrictEqual => "strictEqual" "===",
                    Shl => "shl" "<<",
                    Shr => "shr" ">>",
                    Or => "or" "|",
                    And => "and" "&",
                    Xor => "xor" "^",
                    Max => "max",
                    Min => "min",
                    Angle => "angle",
                    Len => "len",
                    Noise => "noise",
                ]
            }
        }
    }

    pub fn oper_str(&self) -> &'static str {
        self.get_info().oper_str
    }

    pub fn get_result(&self) -> &Value {
        self.get_info().result
    }

    pub fn get_result_mut(&mut self) -> &mut Value {
        self.get_info_mut().result
    }

    pub fn oper_symbol_str(&self) -> &'static str {
        let info = self.get_info();
        info.oper_sym.unwrap_or(info.oper_str)
    }

    pub fn generate_args(self, meta: &mut CompileMeta) -> Vec<String> {
        let info = self.into_info();
        let mut args: Vec<Var> = Vec::with_capacity(5);

        args.push("op".into());
        args.push(info.oper_str.into());
        args.push(info.result.take_handle(meta).into());
        args.push(info.arg1.take_handle(meta).into());
        args.push(
            info.arg2.map(|arg| arg.take_handle(meta))
                .unwrap_or(UNUSED_VAR.into())
        );

        debug_assert!(args.len() == 5);
        args
    }

    /// 根据自身运算类型尝试获取一个比较器
    pub fn get_cmper(&self) -> Option<fn(Value, Value) -> JumpCmp> {
        macro_rules! build_match {
            ($( $sname:ident : $cname:ident ),* $(,)?) => {{
                match self {
                    $(
                        Self::$sname(..) => {
                            Some(JumpCmp::$cname)
                        },
                    )*
                    _ => None,
                }
            }};
        }
        build_match! [
            LessThan: LessThan,
            LessThanEq: LessThanEq,
            GreaterThan: GreaterThanEq,
            GreaterThanEq: GreaterThanEq,
            Equal: Equal,
            NotEqual: NotEqual,
            StrictEqual: StrictEqual,
        ]
    }

    /// 尝试将自身内联为一个比较, 如果返回非空, 则self不可再被使用
    /// 考虑到短路会改变副作用条件
    /// 所以不再内联为逻辑与和逻辑或,仅内联顶层比较
    ///
    /// 需要返回句柄为返回句柄替换符
    pub fn try_into_cmp(&mut self) -> Option<JumpCmp> {
        fn get(value: &mut Value) -> Value {
            replace(value, Value::ResultHandle)
        }
        let cmper = self.get_cmper()?;
        let info = self.get_info_mut();
        do_return!(! info.result.is_result_handle() => None);
        cmper(get(info.arg1), get(info.arg2.unwrap())).into()
    }
}
impl Compile for Op {
    fn compile(self, meta: &mut CompileMeta) {
        let args = self.generate_args(meta);
        meta.tag_codes.push(args.join(" ").into())
    }
}
impl DisplaySource for Op {
    fn display_source(&self, meta: &mut DisplaySourceMeta) {
        macro_rules! build_match {
            {
                op1: [ $( $oper1:ident ),* $(,)?  ]
                op2: [ $( $oper2:ident ),* $(,)?  ]
                op2l: [ $( $oper2l:ident ),* $(,)?  ]
            } => {
                match self {
                    $(
                        Self::$oper1(_, a) => {
                            meta.push(self.oper_symbol_str());
                            meta.add_space();

                            a.display_source(meta);
                        },
                    )*
                    $(
                        Self::$oper2(_, a, b) => {
                            a.display_source(meta);
                            meta.add_space();

                            meta.push(self.oper_symbol_str());
                            meta.add_space();

                            b.display_source(meta);
                        },
                    )*
                    $(
                        Self::$oper2l(_, a, b) => {
                            meta.push(self.oper_symbol_str());
                            meta.add_space();

                            a.display_source(meta);
                            meta.add_space();

                            b.display_source(meta);
                        },
                    )*
                }
            };
        }
        meta.push("op");
        meta.add_space();
        self.get_info().result.display_source(meta);
        meta.add_space();

        build_match! {
            op1: [
                Not, Abs, Log, Log10, Floor, Ceil, Sqrt,
                Rand, Sin, Cos, Tan, Asin, Acos, Atan,
            ]
            op2: [
                Add, Sub, Mul, Div, Idiv,
                Mod, Pow, Equal, NotEqual, Land,
                LessThan, LessThanEq, GreaterThan, GreaterThanEq, StrictEqual,
                Shl, Shr, Or, And, Xor,
            ]
            op2l: [
                Max, Min, Angle, Len, Noise,
            ]
        };
        meta.push(";");
    }
}
impl FromMdtArgs for Op {
    type Err = OpRParseError;

    fn from_mdt_args(args: &[&str]) -> Result<Self, Self::Err> {
        let &[oper, res, a, b] = args else {
            return Err(OpRParseError::ArgsCountError(
                args.into_iter().cloned().map(Into::into).collect()
            ));
        };

        macro_rules! build_match {
            {
                op2: [
                    $(
                        $op2_variant:ident $op2_str:literal
                    ),* $(,)?
                ] $(,)?
                op1: [
                    $(
                        $op1_variant:ident $op1_str:literal
                    ),* $(,)?
                ]
            } => {
                match oper {
                    $(
                    $op1_str => Ok(Self::$op1_variant(res.into(), a.into())),
                    )*

                    $(
                    $op2_str => Ok(Self::$op2_variant(res.into(), a.into(), b.into())),
                    )*

                    oper => {
                        Err(OpRParseError::UnknownOper(
                            oper.into(),
                            [a.into(), b.into()]
                        ))
                    },
                }
            };
        }
        build_match! {
            op2: [
                Add "add",
                Sub "sub",
                Mul "mul",
                Div "div",
                Idiv "idiv",
                Mod "mod",
                Pow "pow",
                Equal "equal",
                NotEqual "notEqual",
                Land "land",
                LessThan "lessThan",
                LessThanEq "lessThanEq",
                GreaterThan "greaterThan",
                GreaterThanEq "greaterThanEq",
                StrictEqual "strictEqual",
                Shl "shl",
                Shr "shr",
                Or "or",
                And "and",
                Xor "xor",
                Max "max",
                Min "min",
                Angle "angle",
                Len "len",
                Noise "noise",
            ],

            op1: [
                Not "not",
                Abs "abs",
                Log "log",
                Log10 "log10",
                Floor "floor",
                Ceil "ceil",
                Sqrt "sqrt",
                Rand "rand",
                Sin "sin",
                Cos "cos",
                Tan "tan",
                Asin "asin",
                Acos "acos",
                Atan "atan",
            ]
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Goto(pub Var, pub CmpTree);
impl Compile for Goto {
    fn compile(self, meta: &mut CompileMeta) {
        self.1.build(meta, self.0)
    }
}
impl DisplaySource for Goto {
    fn display_source(&self, meta: &mut DisplaySourceMeta) {
        let Self(lab, cmp) = self;

        meta.push("goto");
        meta.add_space();
        meta.push(":");
        meta.push(&Value::replace_ident(lab));
        meta.add_space();
        cmp.display_source(meta);
        meta.push(";");
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Expand(pub Vec<LogicLine>);
impl Default for Expand {
    fn default() -> Self {
        Self(Default::default())
    }
}
impl Compile for Expand {
    fn compile(self, meta: &mut CompileMeta) {
        meta.block_enter();
        for line in self.0 {
            line.compile(meta)
        }
        meta.block_exit(); // 如果要获取丢弃块中的常量映射从此处
    }
}
impl From<Vec<LogicLine>> for Expand {
    fn from(value: Vec<LogicLine>) -> Self {
        Self(value)
    }
}
impl DisplaySource for Expand {
    fn display_source(&self, meta: &mut DisplaySourceMeta) {
        self.0
            .iter()
            .for_each(|line| {
                line.display_source(meta);
                meta.add_lf();
            })
    }
}
impl TryFrom<&TagCodes> for Expand {
    type Error = (usize, LogicLineFromTagError);

    fn try_from(codes: &TagCodes) -> Result<Self, Self::Error> {
        let mut lines = Vec::with_capacity(codes.lines().len());
        for (idx, code) in codes.lines().iter().enumerate() {
            lines.push(code.try_into().map_err(|e| (idx, e))?)
        }
        Ok(Self(lines))
    }
}
impl_derefs!(impl for Expand => (self: self.0): Vec<LogicLine>);

#[derive(Debug, PartialEq, Clone)]
pub struct InlineBlock(pub Vec<LogicLine>);
impl DisplaySource for InlineBlock {
    fn display_source(&self, meta: &mut DisplaySourceMeta) {
        self.0
            .iter()
            .for_each(|line| {
                line.display_source(meta);
                meta.add_lf();
            })
    }
}
impl Compile for InlineBlock {
    fn compile(self, meta: &mut CompileMeta) {
        for line in self.0 {
            line.compile(meta)
        }
    }
}
impl_derefs!(impl for InlineBlock => (self: self.0): Vec<LogicLine>);

/// 用于`switch`的`select`结构
/// 编译最后一步会将其填充至每个语句定长
/// 然后将`self.0`乘以每个语句的长并让`@counter += _`来跳转到目标
#[derive(Debug, PartialEq, Clone)]
pub struct Select(pub Value, pub Expand);
impl Compile for Select {
    fn compile(self, meta: &mut CompileMeta) {
        let mut cases: Vec<Vec<TagLine>> = self.1.0
            .into_iter()
            .map(
                |line| line.compile_take(meta)
            ).collect();
        let lens: Vec<usize> = cases.iter()
            .map(|lines| {
                lines.iter()
                    .filter(
                        |line| !line.is_tag_down()
                    )
                    .count()
            }).collect();
        let max_len = lens.iter().copied().max().unwrap_or_default();

        let counter = Value::ReprVar(COUNTER.into());

        // build head
        let head = match max_len {
            0 => {          // no op
                Take("__".into(), self.0).compile_take(meta)
            },
            1 => {          // no mul
                Op::Add(
                    counter.clone(),
                    counter,
                    self.0
                ).compile_take(meta)
            },
            // normal
            _ => {
                let tmp_var = meta.get_tmp_var();
                let mut head = Op::Mul(
                    tmp_var.clone().into(),
                    self.0,
                    Value::ReprVar(max_len.to_string())
                ).compile_take(meta);
                let head_1 = Op::Add(
                    counter.clone(),
                    counter,
                    tmp_var.into()
                ).compile_take(meta);
                head.extend(head_1);
                head
            }
        };

        // 填补不够长的`case`
        for (len, case) in zip(lens, &mut cases) {
            case.extend(
                repeat_with(Default::default)
                .take(max_len - len)
            )
        }

        let lines = meta.tag_codes.lines_mut();
        lines.extend(head);
        lines.extend(cases.into_iter().flatten());
    }
}
impl DisplaySource for Select {
    fn display_source(&self, meta: &mut DisplaySourceMeta) {
        meta.push("select");
        meta.add_space();

        self.0.display_source(meta);
        meta.add_space();

        meta.push("{");
        meta.add_lf();
        meta.do_block(|meta| {
            self.1.display_source(meta);
        });
        meta.push("}");
    }
}

/// 用于switch捕获器捕获目标的枚举
pub enum SwitchCatch {
    /// 上溢
    Overflow,
    /// 下溢
    Underflow,
    /// 未命中
    Misses,
    /// 自定义
    UserDefine(CmpTree),
}
impl SwitchCatch {
    /// 将自身构建为具体的不满足条件
    ///
    /// 也就是说直接可以将输出条件用于一个跳过捕获块的`Goto`中
    /// 需要给定需要跳转到第几个case目标的值与最大case
    ///
    /// 最大case, 例如最大的case为8, 那么传入8
    pub fn build(self, value: Value, max_case: usize) -> CmpTree {
        match self {
            // 对于未命中捕获, 应该在捕获块头部加上一个标记
            // 然后将填充case改为无条件跳转至该标记
            // 而不是使用该构建函数进行构建一个条件
            // 该捕获并不是一个条件捕获式
            Self::Misses => panic!(),
            // 用户定义的条件直接取反返回就好啦, 喵!
            Self::UserDefine(cmp) => cmp.reverse(),
            // 上溢, 捕获式为 `x > max_case`
            // 跳过式为`x <= max_case`
            Self::Overflow => JumpCmp::LessThanEq(
                value,
                Value::ReprVar(max_case.to_string())
            ).into(),
            // 下溢, 捕获式为 `x < 0`
            // 跳过式为`x >= 0`
            Self::Underflow => JumpCmp::GreaterThanEq(
                value,
                Value::ReprVar(ZERO_VAR.into())
            ).into(),
        }
    }

    /// Returns `true` if the switch catch is [`Misses`].
    ///
    /// [`Misses`]: SwitchCatch::Misses
    #[must_use]
    pub fn is_misses(&self) -> bool {
        matches!(self, Self::Misses)
    }
}

/// 在块作用域将Var常量为后方值, 之后使用Var时都会被替换为后方值
#[derive(Debug, PartialEq, Clone)]
pub struct Const(pub Var, pub Value, pub Vec<Var>);
impl Const {
    pub fn new(var: Var, value: Value) -> Self {
        Self(var, value, Default::default())
    }
}
impl Compile for Const {
    fn compile(self, meta: &mut CompileMeta) {
        // 对同作用域定义过的常量形成覆盖
        // 如果要进行警告或者将信息传出则在此处理
        meta.add_const_value(self);
    }
}
impl DisplaySource for Const {
    fn display_source(&self, meta: &mut DisplaySourceMeta) {
        meta.push("const");
        meta.add_space();

        meta.push(&Value::replace_ident(&self.0));
        meta.add_space();

        meta.push("=");
        meta.add_space();

        self.1.display_source(meta);

        meta.push(";");
        meta.add_space();

        let labs = self.2
            .iter()
            .map(|s| Value::replace_ident(&*s))
            .fold(
                Vec::with_capacity(self.2.len()),
                |mut labs, s| {
                    labs.push(s);
                    labs
                }
            );
        meta.push(&format!("# labels: [{}]", labs.join(", ")));
    }
}

/// 在此处计算后方的值, 并将句柄赋给前方值
/// 如果后方不是一个DExp, 而是Var, 那么自然等价于一个常量定义
#[derive(Debug, PartialEq, Clone)]
pub struct Take(pub Var, pub Value);
impl Take {
    /// 根据一系列构建一系列常量传参
    pub fn build_arg_consts(values: Vec<Value>, mut f: impl FnMut(Const)) {
        for (i, value) in values.into_iter().enumerate() {
            let name = format!("_{}", i);
            f(Const(name, value, Vec::with_capacity(0)))
        }
    }

    /// 将常量传参的行构建到Expand末尾
    pub fn build_arg_consts_to_expand(
        values: Vec<Value>,
        expand: &mut Vec<LogicLine>,
    ) {
        Self::build_arg_consts(
            values,
            |r#const| expand.push(r#const.into())
        )
    }

    /// 构建一个Take语句单元
    /// 可以带有参数与返回值
    ///
    /// 返回的是一个行, 因为实际上可能不止Take, 还有用于传参的const等
    ///
    /// - args: 传入参数
    /// - var: 绑定量
    /// - do_leak_res: 是否泄露绑定量
    /// - value: 被求的值
    pub fn new(
        args: Vec<Value>,
        var: Var,
        do_leak_res: bool,
        value: Value,
    ) -> LogicLine {
        if args.is_empty() {
            Take(var, value).into()
        } else {
            let mut len = args.len() + 1;
            if do_leak_res { len += 1 }
            let mut expand = Vec::with_capacity(len);
            Self::build_arg_consts_to_expand(args, &mut expand);
            if do_leak_res {
                expand.push(Take(var.clone(), value).into());
                expand.push(LogicLine::ConstLeak(var));
            } else {
                expand.push(Take(var, value).into())
            }
            debug_assert_eq!(expand.len(), len);
            Expand(expand).into()
        }
    }
}
impl Compile for Take {
    fn compile(self, meta: &mut CompileMeta) {
        Const::new(self.0, self.1.take_handle(meta).into()).compile(meta)
    }
}
impl DisplaySource for Take {
    fn display_source(&self, meta: &mut DisplaySourceMeta) {
        meta.push("take");
        meta.add_space();

        meta.push(&Value::replace_ident(&self.0));
        meta.add_space();

        meta.push("=");
        meta.add_space();

        self.1.display_source(meta);
        meta.push(";");
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum LogicLine {
    Op(Op),
    /// 不要去直接创建它, 而是使用`new_label`去创建
    /// 否则无法将它注册到可能的`const`
    Label(Var),
    Goto(Goto),
    Other(Vec<Value>),
    Expand(Expand),
    InlineBlock(InlineBlock),
    Select(Select),
    NoOp,
    /// 空语句, 什么也不生成
    Ignore,
    Const(Const),
    Take(Take),
    ConstLeak(Var),
    /// 将返回句柄设置为一个指定值
    SetResultHandle(Value),
}
impl Compile for LogicLine {
    fn compile(self, meta: &mut CompileMeta) {
        match self {
            Self::NoOp => meta.push("noop".into()),
            Self::Label(mut lab) => {
                // 如果在常量展开中, 尝试将这个标记替换
                lab = meta.get_in_const_label(lab);
                let data = TagLine::TagDown(meta.get_tag(lab));
                meta.push(data)
            },
            Self::Other(args) => {
                let handles: Vec<String> = args
                    .into_iter()
                    .map(|val| val.take_handle(meta))
                    .collect();
                meta.push(TagLine::Line(handles.join(" ").into()))
            },
            Self::SetResultHandle(value) => {
                let new_dexp_handle = value.take_handle(meta);
                meta.set_dexp_handle(new_dexp_handle);
            },
            Self::Select(select) => select.compile(meta),
            Self::Expand(expand) => expand.compile(meta),
            Self::InlineBlock(block) => block.compile(meta),
            Self::Goto(goto) => goto.compile(meta),
            Self::Op(op) => op.compile(meta),
            Self::Const(r#const) => r#const.compile(meta),
            Self::Take(take) => take.compile(meta),
            Self::ConstLeak(r#const) => meta.add_const_value_leak(r#const),
            Self::Ignore => (),
        }
    }
}
impl Default for LogicLine {
    fn default() -> Self {
        Self::NoOp
    }
}
impl LogicLine {
    /// 添加一个被跳转标签, 顺便向meta声明
    /// 拒绝裸创建Label, 因为会干扰常量被注册Label
    pub fn new_label(lab: Var, meta: &mut Meta) -> Self {
        Self::Label(meta.add_defined_label(lab))
    }

    /// Returns `true` if the logic line is [`Op`].
    ///
    /// [`Op`]: LogicLine::Op
    #[must_use]
    pub fn is_op(&self) -> bool {
        matches!(self, Self::Op(..))
    }

    pub fn as_op(&self) -> Option<&Op> {
        if let Self::Op(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn as_op_mut(&mut self) -> Option<&mut Op> {
        if let Self::Op(v) = self {
            Some(v)
        } else {
            None
        }
    }

    /// Returns `true` if the logic line is [`Label`].
    ///
    /// [`Label`]: LogicLine::Label
    #[must_use]
    pub fn is_label(&self) -> bool {
        matches!(self, Self::Label(..))
    }

    pub fn as_label(&self) -> Option<&Var> {
        if let Self::Label(v) = self {
            Some(v)
        } else {
            None
        }
    }

    /// Returns `true` if the logic line is [`Goto`].
    ///
    /// [`Goto`]: LogicLine::Goto
    #[must_use]
    pub fn is_goto(&self) -> bool {
        matches!(self, Self::Goto(..))
    }

    pub fn as_goto(&self) -> Option<&Goto> {
        if let Self::Goto(v) = self {
            Some(v)
        } else {
            None
        }
    }

    /// Returns `true` if the logic line is [`Other`].
    ///
    /// [`Other`]: LogicLine::Other
    #[must_use]
    pub fn is_other(&self) -> bool {
        matches!(self, Self::Other(..))
    }

    pub fn as_other(&self) -> Option<&Vec<Value>> {
        if let Self::Other(v) = self {
            Some(v)
        } else {
            None
        }
    }

    /// Returns `true` if the logic line is [`Expand`].
    ///
    /// [`Expand`]: LogicLine::Expand
    #[must_use]
    pub fn is_expand(&self) -> bool {
        matches!(self, Self::Expand(..))
    }

    pub fn as_expand(&self) -> Option<&Expand> {
        if let Self::Expand(v) = self {
            Some(v)
        } else {
            None
        }
    }
}
impl_enum_froms!(impl From for LogicLine {
    Op => Op;
    Goto => Goto;
    Expand => Expand;
    InlineBlock => InlineBlock;
    Select => Select;
    Const => Const;
    Take => Take;
});
impl DisplaySource for LogicLine {
    fn display_source(&self, meta: &mut DisplaySourceMeta) {
        match self {
            Self::Expand(expand) => {
                meta.push("{");
                meta.add_lf();
                meta.do_block(|meta| {
                    expand.display_source(meta);
                });
                meta.push("}");
            },
            Self::InlineBlock(block) => {
                meta.push("inline");
                meta.add_space();
                meta.push("{");
                meta.add_lf();
                meta.do_block(|meta| {
                    block.display_source(meta);
                });
                meta.push("}");
            },
            Self::Ignore => meta.push("# ignore line"),
            Self::NoOp => meta.push("noop;"),
            Self::Label(lab) => {
                meta.push(":");
                meta.push(&Value::replace_ident(lab))
            },
            Self::Goto(goto) => goto.display_source(meta),
            Self::Op(op) => op.display_source(meta),
            Self::Select(select) => select.display_source(meta),
            Self::Take(take) => take.display_source(meta),
            Self::Const(r#const) => r#const.display_source(meta),
            Self::ConstLeak(var) => {
                meta.push("# constleak");
                meta.add_space();
                meta.push(&Value::replace_ident(var));
                meta.push(";");
            },
            Self::SetResultHandle(val) => {
                meta.push("setres");
                meta.add_space();
                val.display_source(meta);
                meta.push(";");
            },
            Self::Other(args) => {
                assert_ne!(args.len(), 0);
                let mut iter = args.iter();
                iter.next().unwrap().display_source(meta);
                iter.for_each(|arg| {
                    meta.add_space();
                    arg.display_source(meta);
                });
                meta.push(";");
            },
        }
    }
}
impl TryFrom<&TagLine> for LogicLine {
    type Error = LogicLineFromTagError;

    fn try_from(line: &TagLine) -> Result<Self, Self::Error> {
        type Error = LogicLineFromTagError;
        fn mdt_logic_split_2(s: &str) -> Result<Vec<&str>, Error> {
            mdt_logic_split(s)
                .map_err(|char_num| Error::StringNoStop {
                    str: s.into(),
                    char_num
                })
        }
        match line {
            TagLine::Jump(jump) => {
                assert!(jump.tag().is_none());
                let jump = jump.data();
                let str = &jump.1;
                let (to_tag, args) = (
                    jump.0,
                    mdt_logic_split_2(&str)?
                );
                Ok(Goto(
                    to_tag.to_string(),
                    match JumpCmp::from_mdt_args(&args) {
                        Ok(cmp) => cmp.into(),
                        Err(e) => return Err(e.into()),
                    }
                ).into())
            },
            TagLine::TagDown(tag) => Ok(Self::Label(tag.to_string())),
            TagLine::Line(line) => {
                assert!(line.tag().is_none());
                let line = line.data();
                let args = mdt_logic_split_2(line)?;
                match args[0] {
                    "op" => Op::from_mdt_args(&args[1..])
                        .map(Into::into)
                        .map_err(Into::into),
                    _ => {
                        let mut args_value = Vec::with_capacity(args.len());
                        args_value.extend(args.into_iter().map(Into::into));
                        Ok(Self::Other(args_value))
                    },
                }
            },
        }
    }
}

/// 编译到`TagCodes`
pub trait Compile {
    fn compile(self, meta: &mut CompileMeta);

    /// 使用`compile`生成到尾部后再将其弹出并返回
    ///
    /// 使用时需要考虑其副作用, 例如`compile`并不止做了`push`至尾部,
    /// 它还可能做了其他事
    fn compile_take(self, meta: &mut CompileMeta) -> Vec<TagLine>
    where Self: Sized
    {
        let start = meta.tag_codes.len();
        self.compile(meta);
        meta.tag_codes.lines_mut().split_off(start)
    }
}

#[derive(Debug)]
pub struct CompileMeta {
    /// 标记与`id`的映射关系表
    tags_map: HashMap<String, usize>,
    tag_count: usize,
    tag_codes: TagCodes,
    tmp_var_count: usize,
    /// 块中常量, 且带有展开次数与内部标记
    /// 并且存储了需要泄露的常量
    ///
    /// `Vec<(leaks, HashMap<name, (Vec<Label>, Value)>)>`
    const_var_namespace: Vec<(Vec<Var>, HashMap<Var, (Vec<Var>, Value)>)>,
    /// 每层DExp所使用的句柄, 末尾为当前层
    dexp_result_handles: Vec<Var>,
    tmp_tag_count: usize,
    /// 每层const展开的标签
    /// 一个标签从尾部上寻, 寻到就返回找到的, 没找到就返回原本的
    /// 所以它支持在宏A内部展开的宏B跳转到宏A内部的标记
    const_expand_tag_name_map: Vec<HashMap<Var, Var>>,
}
impl Default for CompileMeta {
    fn default() -> Self {
        Self {
            tags_map: HashMap::new(),
            tag_count: 0,
            tag_codes: TagCodes::new(),
            tmp_var_count: 0,
            const_var_namespace: Vec::new(),
            dexp_result_handles: Vec::new(),
            tmp_tag_count: 0,
            const_expand_tag_name_map: Vec::new(),
        }
    }
}
impl CompileMeta {
    pub fn new() -> Self {
        Self::default()
    }

    /// 获取一个标记的编号, 如果不存在则将其插入并返回新分配的编号.
    /// 注: `Tag`与`Label`是混用的, 表示同一个意思
    pub fn get_tag(&mut self, label: String) -> usize {
        *self.tags_map.entry(label).or_insert_with(|| {
            let id = self.tag_count;
            self.tag_count += 1;
            id
        })
    }

    /// 获取一个临时的`tag`
    pub fn get_tmp_tag(&mut self) -> Var {
        let id = self.tmp_tag_count;
        self.tmp_tag_count += 1;
        format!("__{}", id)
    }

    pub fn get_tmp_var(&mut self) -> Var {
        let id = self.tmp_var_count;
        self.tmp_var_count += 1;
        format!("__{}", id)
    }

    /// 向已生成代码`push`
    pub fn push(&mut self, data: TagLine) {
        self.tag_codes.push(data)
    }

    /// 向已生成代码`pop`
    pub fn pop(&mut self) -> Option<TagLine> {
        self.tag_codes.pop()
    }

    /// 获取已生成的代码条数
    pub fn tag_code_count(&self) -> usize {
        self.tag_codes.len()
    }

    /// 获取已生成的非`TagDown`代码条数
    pub fn tag_code_count_no_tag(&self) -> usize {
        self.tag_codes.count_no_tag()
    }

    pub fn compile(self, lines: Expand) -> TagCodes {
        self.compile_res_self(lines).tag_codes
    }

    pub fn compile_res_self(mut self, lines: Expand) -> Self {
        self.tag_codes.clear();

        lines.compile(&mut self);
        self
    }

    pub fn tag_codes(&self) -> &TagCodes {
        &self.tag_codes
    }

    pub fn tag_codes_mut(&mut self) -> &mut TagCodes {
        &mut self.tag_codes
    }

    pub fn tags_map(&self) -> &HashMap<String, usize> {
        &self.tags_map
    }

    /// 进入一个子块, 创建一个新的子命名空间
    pub fn block_enter(&mut self) {
        self.const_var_namespace.push((Vec::new(), HashMap::new()))
    }

    /// 退出一个子块, 弹出最顶层命名空间
    /// 如果无物可弹说明逻辑出现了问题, 所以内部处理为unwrap
    /// 一个enter对应一个exit
    pub fn block_exit(&mut self) -> HashMap<Var, (Vec<Var>, Value)> {
        // this is poped block
        let (leaks, mut res)
            = self.const_var_namespace.pop().unwrap();

        // do leak
        for leak_const_name in leaks {
            let value
                = res.remove(&leak_const_name).unwrap();

            // insert to prev block
            self.const_var_namespace
                .last_mut()
                .unwrap()
                .1
                .insert(leak_const_name, value);
        }
        res
    }

    /// 添加一个需泄露的const
    pub fn add_const_value_leak(&mut self, name: Var) {
        self.const_var_namespace
            .last_mut()
            .unwrap()
            .0
            .push(name)
    }

    /// 获取一个常量到值的使用次数与映射与其内部标记的引用,
    /// 从当前作用域往顶层作用域一层层找, 都没找到就返回空
    pub fn get_const_value(&self, name: &Var) -> Option<&(Vec<Var>, Value)> {
        self.const_var_namespace
            .iter()
            .rev()
            .find_map(|(_, namespace)| {
                namespace.get(name)
            })
    }

    /// 获取一个常量到值的使用次数与映射与其内部标记的可变引用,
    /// 从当前作用域往顶层作用域一层层找, 都没找到就返回空
    pub fn get_const_value_mut(&mut self, name: &Var)
    -> Option<&mut (Vec<Var>, Value)> {
        self.const_var_namespace
            .iter_mut()
            .rev()
            .find_map(|(_, namespace)| {
                namespace.get_mut(name)
            })
    }

    /// 新增一个常量到值的映射, 如果当前作用域已有此映射则返回旧的值并插入新值
    pub fn add_const_value(&mut self, Const(var, mut value, mut labels): Const)
    -> Option<(Vec<Var>, Value)> {
        if let Some(var_1) = value.as_var() {
            // 如果const的映射目标值是一个var
            if let Some((labels_1, value_1))
                = self.get_const_value(var_1).cloned() {
                    // 且它是一个常量, 则将它直接克隆过来.
                    // 防止直接映射到另一常量,
                    // 但是另一常量被覆盖导致这在覆盖之前映射的常量结果也发生改变
                    (labels, value) = (labels_1, value_1)
                }
        }

        // 当后方为原始值时, 直接转换为普通Var
        // 因为常量替换之后进行一次, 并不是之前的链式替换
        // 所以替换过去一个原始值没有意义
        // 所以常量定义时会去除后方的原始值
        //
        // 去除了原始值后, 我们也可以将其用在DExp返回句柄处防止被替换了
        // 因为常量替换只进行一次
        // 而我们已经进行了一次了
        // 例如
        // ```
        // const X = `X`;
        // print (X:);
        // ```
        value = match value {
            Value::ReprVar(value) => value.into(),
            value => value,
        };

        self.const_var_namespace
            .last_mut()
            .unwrap()
            .1
            .insert(var, (labels, value))
    }

    /// 新增一层DExp, 并且传入它使用的返回句柄
    pub fn push_dexp_handle(&mut self, handle: Var) {
        self.dexp_result_handles.push(handle)
    }

    /// 如果弹无可弹, 说明逻辑出现了问题
    pub fn pop_dexp_handle(&mut self) -> Var {
        self.dexp_result_handles.pop().unwrap()
    }

    pub fn debug_tag_codes(&self) -> Vec<String> {
        self.tag_codes().lines().iter()
            .map(ToString::to_string)
            .collect()
    }

    pub fn debug_tags_map(&self) -> Vec<String> {
        let mut tags_map: Vec<_> = self
            .tags_map()
            .iter()
            .collect();
        tags_map.sort_unstable_by_key(|(_, &k)| k);
        tags_map
            .into_iter()
            .map(|(tag, id)| format!("{} \t-> {}", id, tag))
            .collect()
    }

    /// 获取当前DExp返回句柄
    pub fn dexp_handle(&self) -> &Var {
        self.dexp_result_handles
            .last()
            .unwrap_or_else(
                || self.do_out_of_dexp_err("`DExpHandle` (`$`)"))
    }

    /// 将当前DExp返回句柄替换为新的
    /// 并将旧的句柄返回
    pub fn set_dexp_handle(&mut self, new_dexp_handle: Var) -> Var {
        if let Some(ref_) = self.dexp_result_handles.last_mut() {
            replace(ref_, new_dexp_handle)
        } else {
            self.do_out_of_dexp_err("`setres`")
        }
    }

    /// 对在外部使用`DExpHandle`进行报错
    fn do_out_of_dexp_err(&self, value: &str) -> ! {
        err!(
            "{}\n尝试在`DExp`的外部使用{}",
            self.err_info().join("\n"),
            value,
        );
        exit(6)
    }

    /// 对于一个标记(Label), 进行寻找, 如果是在展开宏中, 则进行替换
    /// 一层层往上找, 如果没找到返回本身
    pub fn get_in_const_label(&self, name: Var) -> Var {
        self.const_expand_tag_name_map.iter().rev()
            .find_map(|map| map.get(&name).cloned())
            .unwrap_or(name)
    }

    /// 进入一层宏展开环境, 并且返回其值
    /// 这个函数会直接调用获取函数将标记映射完毕, 然后返回其值
    /// 如果不是一个宏则直接返回None, 也不会进入无需清理
    pub fn const_expand_enter(&mut self, name: &Var) -> Option<Value> {
        let label_count = self.get_const_value(name)?.0.len();
        let mut tmp_tags = Vec::with_capacity(label_count);
        tmp_tags.extend(repeat_with(|| self.get_tmp_tag())
                        .take(label_count));

        let (labels, value)
            = self.get_const_value(name).unwrap();
        let mut labels_map = HashMap::with_capacity(labels.len());
        for (tmp_tag, label) in zip(tmp_tags, labels.iter().cloned()) {
            let maped_label = format!(
                "{}_const_{}_{}",
                tmp_tag,
                &name,
                &label
            );
            labels_map.insert(label, maped_label);
        }
        let res = value.clone();
        self.const_expand_tag_name_map.push(labels_map);
        res.into()
    }

    pub fn const_expand_exit(&mut self) -> HashMap<Var, Var> {
        self.const_expand_tag_name_map.pop().unwrap()
    }

    pub fn err_info(&self) -> Vec<String> {
        let mut res = Vec::new();

        let mut tags_map = self.debug_tags_map();
        let mut tag_lines = self.debug_tag_codes();
        line_first_add(&mut tags_map, "\t");
        line_first_add(&mut tag_lines, "\t");

        res.push("Id映射Tag:".into());
        res.extend(tags_map);
        res.push("已生成代码:".into());
        res.extend(tag_lines);
        res
    }
}

pub fn line_first_add(lines: &mut Vec<String>, insert: &str) {
    for line in lines {
        let s = format!("{}{}", insert, line);
        *line = s;
    }
}

pub enum OpExprInfo {
    Value(Value),
    Op(Op),
    IfElse {
        child_result: Var,
        cmp: CmpTree,
        true_line: LogicLine,
        false_line: LogicLine,
    },
}
impl OpExprInfo {
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
                    Take(child_result, Value::ResultHandle).into(),
                    Goto(true_lab.clone(), cmp).into(),
                    false_line,
                    Goto(skip_lab.clone(), JumpCmp::Always.into()).into(),
                    LogicLine::Label(true_lab),
                    true_line,
                    LogicLine::Label(skip_lab),
                ].into()).into()
            },
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
                    Take(child_result, result).into(),
                    Goto(true_lab.clone(), cmp).into(),
                    false_line,
                    Goto(skip_lab.clone(), JumpCmp::Always.into()).into(),
                    LogicLine::Label(true_lab),
                    true_line,
                    LogicLine::Label(skip_lab),
                ]).into()
            },
        }
    }
}
impl_enum_froms!(impl From for OpExprInfo {
    Value => Value;
    Op => Op;
});

/// 构建一个op运算
pub fn op_expr_build_op<F>(f: F) -> OpExprInfo
where F: FnOnce() -> Op
{
    f().into()
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;
    use crate::syntax_def::*;

    /// 快捷的创建一个新的`Meta`并且`parse`
    macro_rules! parse {
        ( $parser:expr, $src:expr ) => {
            ($parser).parse(&mut Meta::new(), $src)
        };
    }

    #[test]
    fn var_test() {
        let parser = VarParser::new();

        assert_eq!(parse!(parser, "_abc").unwrap(), "_abc");
        assert_eq!(parse!(parser, "'ab-cd'").unwrap(), "ab-cd");
        assert_eq!(parse!(parser, "'ab.cd'").unwrap(), "ab.cd");
        assert_eq!(parse!(parser, "0x1_b").unwrap(), "0x1b");
        assert_eq!(parse!(parser, "-4_3_.7_29").unwrap(), "-43.729");
        assert_eq!(parse!(parser, "0b-00_10").unwrap(), "0b-0010");
        assert_eq!(parse!(parser, "@abc-def").unwrap(), "@abc-def");
        assert_eq!(parse!(parser, "@abc-def_30").unwrap(), "@abc-def_30");
        assert_eq!(parse!(parser, "@abc-def-34").unwrap(), "@abc-def-34");
        assert_eq!(parse!(parser, r#"'abc"def'"#).unwrap(), "abc'def"); // 双引号被替换为单引号

        assert!(parse!(parser, "'ab cd'").is_err());
        assert!(parse!(parser, "ab-cd").is_err());
        assert!(parse!(parser, "0o25").is_err()); // 不支持8进制, 懒得弄转换
        assert!(parse!(parser, r"@ab\c").is_err());
        assert!(parse!(parser, "-_2").is_err());
        assert!(parse!(parser, "-0._3").is_err());
        assert!(parse!(parser, "0x_2").is_err());
    }

    #[test]
    fn expand_test() {
        let parser = TopLevelParser::new();
        let lines = parse!(parser, r#"
        op + a a 1;
        op - a a 1;
        op a a * 2;
        "#).unwrap();
        let mut iter = lines.iter();
        assert_eq!(iter.next().unwrap(), &Op::Add("a".into(), "a".into(), "1".into()).into());
        assert_eq!(iter.next().unwrap(), &Op::Sub("a".into(), "a".into(), "1".into()).into());
        assert_eq!(iter.next().unwrap(), &Op::Mul("a".into(), "a".into(), "2".into()).into());
        assert!(iter.next().is_none());

        assert_eq!(parse!(parser, "op x sin y 0;").unwrap()[0], Op::Sin("x".into(), "y".into()).into());
        assert_eq!(
            parse!(
                parser,
                "op res (op $ 1 + 2; op $ $ * 2;) / (x: op $ 2 * 3;);"
            ).unwrap()[0],
            Op::Div(
                "res".into(),
                DExp::new_nores(
                    vec![
                        Op::Add(
                            Value::ResultHandle,
                            "1".into(),
                            "2".into()
                        ).into(),
                        Op::Mul(
                            Value::ResultHandle,
                            Value::ResultHandle,
                            "2".into()
                        ).into()
                    ].into()).into(),
                DExp::new(
                    "x".into(),
                    vec![
                        Op::Mul(Value::ResultHandle, "2".into(), "3".into()).into()
                    ].into(),
                ).into()
            ).into()
        );
        assert_eq!(
            parse!(
                parser,
                "op res (op $ 1 + 2; op $ $ * 2;) / (op $ 2 * 3;);"
            ).unwrap()[0],
            Op::Div(
                "res".into(),
                DExp::new_nores(
                    vec![
                        Op::Add(
                            Value::ResultHandle,
                            "1".into(),
                            "2".into()
                        ).into(),
                        Op::Mul(
                            Value::ResultHandle,
                            Value::ResultHandle,
                            "2".into()
                        ).into()
                    ].into()).into(),
                DExp::new_nores(
                    vec![
                        Op::Mul(Value::ResultHandle, "2".into(), "3".into()).into()
                    ].into(),
                ).into()
            ).into()
        );
    }

    #[test]
    fn goto_test() {
        let parser = TopLevelParser::new();
        assert_eq!(parse!(parser, "goto :a 1 <= 2; :a").unwrap(), vec![
            Goto("a".into(), JumpCmp::LessThanEq("1".into(), "2".into()).into()).into(),
            LogicLine::Label("a".into()),
        ].into());
    }

    #[test]
    fn control_test() {
        let parser = LogicLineParser::new();
        assert_eq!(
            parse!(parser, r#"skip 1 < 2 print "hello";"#).unwrap(),
            Expand(vec![
                Goto("___0".into(), JumpCmp::LessThan("1".into(), "2".into()).into()).into(),
                LogicLine::Other(vec![Value::ReprVar("print".into()), r#""hello""#.into()]),
                LogicLine::Label("___0".into()),
            ]).into()
        );

        assert_eq!(
            parse!(parser, r#"
            if 2 < 3 {
                print 1;
            } elif 3 < 4 {
                print 2;
            } elif 4 < 5 {
                print 3;
            } else print 4;
            "#).unwrap(),
            parse!(parser, r#"
            {
                goto :___1 2 < 3;
                goto :___2 3 < 4;
                goto :___3 4 < 5;
                print 4;
                goto :___0 _;
                :___2 {
                    print 2;
                }
                goto :___0 _;
                :___3 {
                    print 3;
                }
                goto :___0 _;
                :___1 {
                    print 1;
                }
                :___0
            }
            "#).unwrap(),
        );

        assert_eq!(
            parse!(parser, r#"
            if 2 < 3 { # 对于没有elif与else的if, 会将条件反转并构建为skip
                print 1;
            }
            "#).unwrap(),
            parse!(parser, r#"
            skip ! 2 < 3 {
                print 1;
            }
            "#).unwrap(),
        );

        assert_eq!(
            parse!(parser, r#"
            while a < b
                print 3;
            "#).unwrap(),
            parse!(parser, r#"
            {
                goto :___0 a >= b;
                :___1
                print 3;
                goto :___1 a < b;
                :___0
            }
            "#).unwrap(),
        );

        assert_eq!(
            parse!(parser, r#"
            do {
                print 1;
            } while a < b;
            "#).unwrap(),
            parse!(parser, r#"
            {
                :___0 {
                    print 1;
                }
                goto :___0 a < b;
            }
            "#).unwrap(),
        );

        assert_eq!(
            parse!(parser, r#"
            gwhile a < b {
                print 1;
            }
            "#).unwrap(),
            parse!(parser, r#"
            {
                goto :___0 _;
                :___1 {
                    print 1;
                }
                :___0
                goto :___1 a < b;
            }
            "#).unwrap(),
        );
    }

    #[test]
    fn reverse_test() {
        let parser = LogicLineParser::new();

        let datas = vec![
            [r#"goto :a x === y;"#, r#"goto :a x !== y;"#],
            [r#"goto :a x == y;"#, r#"goto :a x != y;"#],
            [r#"goto :a x != y;"#, r#"goto :a x == y;"#],
            [r#"goto :a x < y;"#, r#"goto :a x >= y;"#],
            [r#"goto :a x > y;"#, r#"goto :a x <= y;"#],
            [r#"goto :a x <= y;"#, r#"goto :a x > y;"#],
            [r#"goto :a x >= y;"#, r#"goto :a x < y;"#],
            [r#"goto :a x;"#, r#"goto :a x == `false`;"#],
            [r#"goto :a _;"#, r#"goto :a !_;"#],
        ];
        for [src, dst] in datas {
            assert_eq!(
                parse!(parser, src).unwrap().as_goto().unwrap().1.clone().reverse(),
                parse!(parser, dst).unwrap().as_goto().unwrap().1,
            );
        }

        // 手动转换
        let datas = vec![
            [r#"goto :a ! x === y;"#, r#"goto :a x !== y;"#],
            [r#"goto :a ! x == y;"#, r#"goto :a x != y;"#],
            [r#"goto :a ! x != y;"#, r#"goto :a x == y;"#],
            [r#"goto :a ! x < y;"#, r#"goto :a x >= y;"#],
            [r#"goto :a ! x > y;"#, r#"goto :a x <= y;"#],
            [r#"goto :a ! x <= y;"#, r#"goto :a x > y;"#],
            [r#"goto :a ! x >= y;"#, r#"goto :a x < y;"#],
            [r#"goto :a ! x;"#, r#"goto :a x == `false`;"#],
            // 多次取反
            [r#"goto :a !!! x == y;"#, r#"goto :a x != y;"#],
            [r#"goto :a !!! x != y;"#, r#"goto :a x == y;"#],
            [r#"goto :a !!! x < y;"#, r#"goto :a x >= y;"#],
            [r#"goto :a !!! x > y;"#, r#"goto :a x <= y;"#],
            [r#"goto :a !!! x <= y;"#, r#"goto :a x > y;"#],
            [r#"goto :a !!! x >= y;"#, r#"goto :a x < y;"#],
            [r#"goto :a !!! x;"#, r#"goto :a x == `false`;"#],
            [r#"goto :a !!! x === y;"#, r#"goto :a x !== y;"#],
            [r#"goto :a !!! _;"#, r#"goto :a !_;"#],
        ];
        for [src, dst] in datas {
            assert_eq!(
                parse!(parser, src).unwrap().as_goto().unwrap().1,
                parse!(parser, dst).unwrap().as_goto().unwrap().1,
            );
        }
    }

    #[test]
    fn goto_compile_test() {
        let parser = TopLevelParser::new();

        let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
        goto :x _;
        :x
        end;
        "#).unwrap()).compile().unwrap();
        assert_eq!(logic_lines, vec![
                   "jump 1 always 0 0",
                   "end",
        ]);

        let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
        const 0 = 1;
        goto :x _;
        :x
        end;
        "#).unwrap()).compile().unwrap();
        assert_eq!(logic_lines, vec![
                   "jump 1 always 0 0",
                   "end",
        ]);

        let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
        const 0 = 1;
        goto :x !_;
        :x
        end;
        "#).unwrap()).compile().unwrap();
        assert_eq!(logic_lines, vec![
                   "jump 1 notEqual 0 0",
                   "end",
        ]);

        let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
        const 0 = 1;
        const false = true;
        goto :x a === b;
        :x
        end;
        "#).unwrap()).compile().unwrap();
        assert_eq!(logic_lines, vec![
                   "jump 1 strictEqual a b",
                   "end",
        ]);

        let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
        const 0 = 1;
        const false = true;
        goto :x !!a === b;
        :x
        end;
        "#).unwrap()).compile().unwrap();
        assert_eq!(logic_lines, vec![
                   "jump 1 strictEqual a b",
                   "end",
        ]);

        let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
        const 0 = 1;
        const false = true;
        goto :x !a === b;
        :x
        end;
        "#).unwrap()).compile().unwrap();
        assert_eq!(logic_lines, vec![
                   "op strictEqual __0 a b",
                   "jump 2 equal __0 false",
                   "end",
        ]);

        let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
        const 0 = 1;
        const false = true;
        goto :x a !== b;
        :x
        end;
        "#).unwrap()).compile().unwrap();
        assert_eq!(logic_lines, vec![
                   "op strictEqual __0 a b",
                   "jump 2 equal __0 false",
                   "end",
        ]);

    }

    #[test]
    fn line_test() {
        let parser = LogicLineParser::new();
        assert_eq!(parse!(parser, "noop;").unwrap(), LogicLine::NoOp);
    }

    #[test]
    fn literal_uint_test() {
        let parser = LiteralUIntParser::new();
        assert!(parse!(parser, "1.5").is_err());

        assert_eq!(parse!(parser, "23").unwrap(), 23);
        assert_eq!(parse!(parser, "0x1b").unwrap(), 0x1b);
        assert_eq!(parse!(parser, "0b10_1001").unwrap(), 0b10_1001);
    }

    #[test]
    fn switch_test() {
        let parser = LogicLineParser::new();

        let ast = parse!(parser, r#"
            switch 2 {
            case 1:
                print 1;
            case 2 4:
                print 2;
                print 4;
            case 5:
                :a
                :b
                print 5;
            }
        "#).unwrap();
        assert_eq!(
            ast,
            Select(
                "2".into(),
                Expand(vec![
                    LogicLine::Ignore,
                    Expand(vec![LogicLine::Other(vec![Value::ReprVar("print".into()), "1".into()])]).into(),
                    Expand(vec![
                        LogicLine::Other(vec![Value::ReprVar("print".into()), "2".into()]),
                        LogicLine::Other(vec![Value::ReprVar("print".into()), "4".into()]),
                    ]).into(),
                    LogicLine::Ignore,
                    Expand(vec![
                        LogicLine::Other(vec![Value::ReprVar("print".into()), "2".into()]),
                        LogicLine::Other(vec![Value::ReprVar("print".into()), "4".into()]),
                    ]).into(),
                    Expand(vec![
                        LogicLine::Label("a".into()),
                        LogicLine::Label("b".into()),
                        LogicLine::Other(vec![Value::ReprVar("print".into()), "5".into()]),
                    ]).into(),
                ])
            ).into()
        );
        let mut tag_codes = CompileMeta::new()
            .compile(Expand(vec![ast]).into());
        let lines = tag_codes
            .compile()
            .unwrap();
        assert_eq!(lines, [
            "op mul __0 2 2",
            "op add @counter @counter __0",
            "noop",
            "noop",
            "print 1",
            "noop",
            "print 2",
            "print 4",
            "noop",
            "noop",
            "print 2",
            "print 4",
            "print 5",
            "noop",
        ]);

        let ast = parse!(parser, r#"
            switch 1 {
            print end;
            case 0: print 0;
            case 1: print 1;
            }
        "#).unwrap();
        assert_eq!(
            ast,
            Select(
                "1".into(),
                Expand(vec![
                    Expand(vec![
                            LogicLine::Other(vec![Value::ReprVar("print".into()), "0".into()]),
                            LogicLine::Other(vec![Value::ReprVar("print".into()), "end".into()]),
                    ]).into(),
                    Expand(vec![
                            LogicLine::Other(vec![Value::ReprVar("print".into()), "1".into()]),
                            LogicLine::Other(vec![Value::ReprVar("print".into()), "end".into()]),
                    ]).into(),
                ])
            ).into()
        );

        // 测试追加对于填充的效用
        let ast = parse!(parser, r#"
            switch 1 {
            print end;
            case 1: print 1;
            }
        "#).unwrap();
        assert_eq!(
            ast,
            Select(
                "1".into(),
                Expand(vec![
                    Expand(vec![
                            LogicLine::Other(vec![Value::ReprVar("print".into()), "end".into()]),
                    ]).into(),
                    Expand(vec![
                            LogicLine::Other(vec![Value::ReprVar("print".into()), "1".into()]),
                            LogicLine::Other(vec![Value::ReprVar("print".into()), "end".into()]),
                    ]).into(),
                ])
            ).into()
        );
    }

    #[test]
    fn comments_test() {
        let parser = LogicLineParser::new();
        assert_eq!(
            parse!(parser, r#"
            # inline comment
            #comment1
            #* this is a long comments
             * ...
             * gogogo
             *#
            #***x*s;;@****\*\*#
            #*##xs*** #** *#
            #*r*#
            #
            #*一行内的长注释*#
            #*语句前面的长注释*#noop;#语句后注释
            #注释
            "#
            ).unwrap(),
            LogicLine::NoOp
        );
    }

    #[test]
    fn op_generate_test() {
        assert_eq!(
            Op::Add("x".into(), "y".into(), "z".into()).generate_args(&mut Default::default()),
            vec!["op", "add", "x", "y", "z"],
        );
    }

    #[test]
    fn compile_test() {
        let parser = TopLevelParser::new();
        let src = r#"
        op x 1 + 2;
        op y (op $ x + 3;) * (op $ x * 2;);
        if (op tmp y & 1; op $ tmp + 1;) == 1 {
            print "a ";
        } else {
            print "b ";
        }
        print (op $ y + 3;);
        "#;
        let ast = parse!(parser, src).unwrap();
        let meta = CompileMeta::new();
        let mut tag_codes = meta.compile(ast);
        let logic_lines = tag_codes.compile().unwrap();
        assert_eq!(logic_lines, [
            r#"op add x 1 2"#,
            r#"op add __0 x 3"#,
            r#"op mul __1 x 2"#,
            r#"op mul y __0 __1"#,
            r#"op and tmp y 1"#,
            r#"op add __2 tmp 1"#,
            r#"jump 9 equal __2 1"#,
            r#"print "b ""#,
            r#"jump 10 always 0 0"#,
            r#"print "a ""#,
            r#"op add __3 y 3"#,
            r#"print __3"#,
        ])
    }

    #[test]
    fn compile_take_test() {
        let parser = LogicLineParser::new();
        let ast = parse!(parser, "op x (op $ 1 + 2;) + 3;").unwrap();
        let mut meta = CompileMeta::new();
        meta.push(TagLine::Line("noop".to_string().into()));
        assert_eq!(
            ast.compile_take(&mut meta),
            vec![
                TagLine::Line("op add __0 1 2".to_string().into()),
                TagLine::Line("op add x __0 3".to_string().into()),
            ]
        );
        assert_eq!(meta.tag_codes.len(), 1);
        assert_eq!(meta.tag_codes.lines(), &vec![TagLine::Line("noop".to_string().into())]);
    }

    #[test]
    fn const_value_test() {
        let parser = TopLevelParser::new();

        let src = r#"
        x = C;
        const C = (read $ cell1 0;);
        y = C;
        "#;
        let ast = parse!(parser, src).unwrap();
        let meta = CompileMeta::new();
        let mut tag_codes = meta.compile(ast);
        let logic_lines = tag_codes.compile().unwrap();
        assert_eq!(logic_lines, vec![
                   "set x C",
                   "read __0 cell1 0",
                   "set y __0",
        ]);

        let src = r#"
        x = C;
        const C = (k: read k cell1 0;);
        y = C;
        "#;
        let ast = parse!(parser, src).unwrap();
        let meta = CompileMeta::new();
        let mut tag_codes = meta.compile(ast);
        let logic_lines = tag_codes.compile().unwrap();
        assert_eq!(logic_lines, vec![
                   "set x C",
                   "read k cell1 0",
                   "set y k",
        ]);

        let src = r#"
        x = C;
        const C = (read $ cell1 0;);
        foo a b C d C;
        "#;
        let ast = parse!(parser, src).unwrap();
        let meta = CompileMeta::new();
        let mut tag_codes = meta.compile(ast);
        let logic_lines = tag_codes.compile().unwrap();
        assert_eq!(logic_lines, vec![
                   "set x C",
                   "read __0 cell1 0",
                   "read __1 cell1 0",
                   "foo a b __0 d __1",
        ]);

        let src = r#"
        const C = (m: read $ cell1 0;);
        x = C;
        "#;
        let ast = parse!(parser, src).unwrap();
        let meta = CompileMeta::new();
        let mut tag_codes = meta.compile(ast);
        let logic_lines = tag_codes.compile().unwrap();
        assert_eq!(logic_lines, vec![
                   "read m cell1 0",
                   "set x m",
        ]);

        let src = r#"
        const C = (read $ cell1 (i: read $ cell2 0;););
        print C;
        "#;
        let ast = parse!(parser, src).unwrap();
        let meta = CompileMeta::new();
        let mut tag_codes = meta.compile(ast);
        let logic_lines = tag_codes.compile().unwrap();
        assert_eq!(logic_lines, vec![
                   "read i cell2 0",
                   "read __0 cell1 i",
                   "print __0",
        ]);
    }

    #[test]
    fn const_value_block_range_test() {
        let parser = TopLevelParser::new();

        let src = r#"
        {
            x = C;
            const C = (read $ cell1 0;);
            const C = (read $ cell2 0;); # 常量覆盖
            {
                const C = (read $ cell3 0;); # 子块常量
                m = C;
            }
            y = C;
            foo C C;
        }
        z = C;
        "#;
        let ast = parse!(parser, src).unwrap();
        let meta = CompileMeta::new();
        let mut tag_codes = meta.compile(ast);
        let logic_lines = tag_codes.compile().unwrap();
        assert_eq!(logic_lines, vec![
                   "set x C",
                   "read __0 cell3 0",
                   "set m __0",
                   "read __1 cell2 0",
                   "set y __1",
                   "read __2 cell2 0",
                   "read __3 cell2 0",
                   "foo __2 __3",
                   "set z C",
        ]);
    }

    #[test]
    fn take_test() {
        let parser = TopLevelParser::new();

        let src = r#"
        print start;
        const F = (read $ cell1 0;);
        take V = F; # 求值并映射
        print V;
        print V; # 再来一次
        foo V V;
        take V1 = F; # 再求值并映射
        print V1;
        "#;
        let ast = parse!(parser, src).unwrap();
        let meta = CompileMeta::new();
        let mut tag_codes = meta.compile(ast);
        let logic_lines = tag_codes.compile().unwrap();
        assert_eq!(logic_lines, vec![
                   "print start",
                   "read __0 cell1 0",
                   "print __0",
                   "print __0",
                   "foo __0 __0",
                   "read __1 cell1 0",
                   "print __1",
        ]);

        let src = r#"
        const F = (m: read $ cell1 0;);
        take V = F; # 求值并映射
        print V;
        "#;
        let ast = parse!(parser, src).unwrap();
        let meta = CompileMeta::new();
        let mut tag_codes = meta.compile(ast);
        let logic_lines = tag_codes.compile().unwrap();
        assert_eq!(logic_lines, vec![
                   "read m cell1 0",
                   "print m",
        ]);
    }

    #[test]
    fn print_test() {
        let parser = TopLevelParser::new();

        let src = r#"
        print "abc" "def" "ghi" j 123 @counter;
        "#;
        let ast = parse!(parser, src).unwrap();
        let meta = CompileMeta::new();
        let mut tag_codes = meta.compile(ast);
        let logic_lines = tag_codes.compile().unwrap();
        assert_eq!(logic_lines, vec![
                   r#"print "abc""#,
                   r#"print "def""#,
                   r#"print "ghi""#,
                   r#"print j"#,
                   r#"print 123"#,
                   r#"print @counter"#,
        ]);

    }

    #[test]
    fn in_const_label_test() {
        let parser = TopLevelParser::new();
        let mut meta = Meta::new();
        let ast = parser.parse(&mut meta, r#"
        :start
        const X = (
            :in_const
            print "hi";
        );
        "#).unwrap();
        let mut iter = ast.0.into_iter();
        assert_eq!(iter.next().unwrap(), LogicLine::Label("start".into()));
        assert_eq!(
            iter.next().unwrap(),
            Const(
                "X".into(),
                DExp::new_nores(
                    vec![
                        LogicLine::Label("in_const".into()),
                        LogicLine::Other(vec![Value::ReprVar("print".into()), "\"hi\"".into()])
                    ].into()
                ).into(),
                vec!["in_const".into()]
            ).into()
        );
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn const_expand_label_rename_test() {
        let parser = TopLevelParser::new();

        let mut meta = Meta::new();
        let ast = parser.parse(&mut meta, r#"
            :start
            const X = (
                if num < 2 {
                    print "num < 2";
                } else
                    print "num >= 2";
                goto :start _;
            );
            take __ = X;
            take __ = X;
        "#).unwrap();
        let compile_meta = CompileMeta::new();
        let mut tag_codes = compile_meta.compile(ast);
        let logic_lines = tag_codes.compile().unwrap();
        assert_eq!(
            logic_lines,
            vec![
                r#"jump 3 lessThan num 2"#,
                r#"print "num >= 2""#,
                r#"jump 0 always 0 0"#,
                r#"print "num < 2""#,
                r#"jump 0 always 0 0"#,
                r#"jump 8 lessThan num 2"#,
                r#"print "num >= 2""#,
                r#"jump 0 always 0 0"#,
                r#"print "num < 2""#,
                r#"jump 0 always 0 0"#,
            ]
        );

        let mut meta = Meta::new();
        let ast = parser.parse(&mut meta, r#"
            # 这里是__0以此类推, 所以接下来的使用C的句柄为__2, 测试数据解释
            const A = (
                const B = (
                    i = C;
                    goto :next _; # 测试往外跳
                );
                const C = (op $ 1 + 1;);
                take __ = B;
                print "skiped";
                :next
                do {
                    print "in a";
                    op i i + 1;
                } while i < 5;
            );
            take __ = A;
        "#).unwrap();
        let compile_meta = CompileMeta::new();
        let mut tag_codes = compile_meta.compile(ast);
        let logic_lines = tag_codes.compile().unwrap();
        assert_eq!(
            logic_lines,
            vec![
                r#"op add __2 1 1"#,
                r#"set i __2"#,
                r#"jump 4 always 0 0"#,
                r#"print "skiped""#,
                r#"print "in a""#,
                r#"op add i i 1"#,
                r#"jump 4 lessThan i 5"#,
            ]
        );
    }

    #[test]
    fn dexp_result_handle_use_const_test() {
        let parser = TopLevelParser::new();

        let ast = parse!(parser, r#"
        {
            print (R: $ = 2;);
            const R = x;
            print (R: $ = 2;);
        }
        print (R: $ = 2;);
        "#).unwrap();
        let meta = CompileMeta::new();
        let mut tag_codes = meta.compile(ast);
        let logic_lines = tag_codes.compile().unwrap();
        assert_eq!(logic_lines, vec![
                   "set R 2",
                   "print R",
                   "set x 2",
                   "print x",
                   "set R 2",
                   "print R",
        ]);
    }

    #[test]
    fn in_const_const_label_rename_test() {
        let parser = TopLevelParser::new();

        let ast = parse!(parser, r#"
        const X = (
            const X = (
                i = 0;
                do {
                    op i i + 1;
                } while i < 10;
            );
            take __ = X;
            take __ = X;
        );
        take __ = X;
        take __ = X;
        "#).unwrap();
        let meta = CompileMeta::new();
        let mut tag_codes = meta.compile(ast);
        let _logic_lines = tag_codes.compile().unwrap();
    }

    #[test]
    fn take_default_result_test() {
        let parser = LogicLineParser::new();

        let ast = parse!(parser, "take 2;").unwrap();
        assert_eq!(ast, Take("__".into(), "2".into()).into());
    }

    #[test]
    fn const_value_leak_test() {
        let ast: Expand = vec![
            Expand(vec![
                LogicLine::Other(vec!["print".into(), "N".into()]),
                Const("N".into(), "2".into(), Vec::new()).into(),
                LogicLine::Other(vec!["print".into(), "N".into()]),
            ]).into(),
            LogicLine::Other(vec!["print".into(), "N".into()]),
        ].into();
        let meta = CompileMeta::new();
        let mut tag_codes = meta.compile(ast);
        let logic_lines = tag_codes.compile().unwrap();
        assert_eq!(logic_lines, vec![
                   "print N",
                   "print 2",
                   "print N",
        ]);

        let ast: Expand = vec![
            Expand(vec![
                LogicLine::Other(vec!["print".into(), "N".into()]),
                Const("N".into(), "2".into(), Vec::new()).into(),
                LogicLine::Other(vec!["print".into(), "N".into()]),
                LogicLine::ConstLeak("N".into()),
            ]).into(),
            LogicLine::Other(vec!["print".into(), "N".into()]),
        ].into();
        let meta = CompileMeta::new();
        let mut tag_codes = meta.compile(ast);
        let logic_lines = tag_codes.compile().unwrap();
        assert_eq!(logic_lines, vec![
                   "print N",
                   "print 2",
                   "print 2",
        ]);
    }

    #[test]
    fn take_test2() {
        let parser = LogicLineParser::new();

        let ast = parse!(parser, "take X;").unwrap();
        assert_eq!(ast, Take("__".into(), "X".into()).into());

        let ast = parse!(parser, "take R = X;").unwrap();
        assert_eq!(ast, Take("R".into(), "X".into()).into());

        let ast = parse!(parser, "take[] X;").unwrap();
        assert_eq!(ast, Take("__".into(), "X".into()).into());

        let ast = parse!(parser, "take[] R = X;").unwrap();
        assert_eq!(ast, Take("R".into(), "X".into()).into());

        let ast = parse!(parser, "take[1 2] R = X;").unwrap();
        assert_eq!(ast, Expand(vec![
                Const::new("_0".into(), "1".into()).into(),
                Const::new("_1".into(), "2".into()).into(),
                Take("R".into(), "X".into()).into(),
                LogicLine::ConstLeak("R".into()),
        ]).into());

        let ast = parse!(parser, "take[1 2] X;").unwrap();
        assert_eq!(ast, Expand(vec![
                Const::new("_0".into(), "1".into()).into(),
                Const::new("_1".into(), "2".into()).into(),
                Take("__".into(), "X".into()).into(),
        ]).into());
    }

    #[test]
    fn take_args_test() {
        let parser = TopLevelParser::new();

        let ast = parse!(parser, r#"
        const M = (
            print _0 _1 _2;
            set $ 3;
        );
        take[1 2 3] M;
        take[4 5 6] R = M;
        print R;
        "#).unwrap();
        let meta = CompileMeta::new();
        let mut tag_codes = meta.compile(ast);
        let logic_lines = tag_codes.compile().unwrap();
        assert_eq!(logic_lines, vec![
                   "print 1",
                   "print 2",
                   "print 3",
                   "set __0 3",
                   "print 4",
                   "print 5",
                   "print 6",
                   "set __1 3",
                   "print __1",
        ]);

        let ast = parse!(parser, r#"
        const DO = (
            print _0 "start";
            take _1;
            print _0 "start*2";
            take _1;
            printflush message1;
        );
        # 这里赋给一个常量再使用, 因为直接使用不会记录label, 无法重复被使用
        # 而DO中, 会使用两次传入的参数1
        const F = (
            i = 0;
            while i < 10 {
                print i;
                op i i + 1;
            }
        );
        take["loop" F] DO;
        "#).unwrap();
        let meta = CompileMeta::new();
        let mut tag_codes = meta.compile(ast);
        let logic_lines = tag_codes.compile().unwrap();
        assert_eq!(logic_lines, vec![
                   r#"print "loop""#,
                   r#"print "start""#,
                   r#"set i 0"#,
                   r#"jump 7 greaterThanEq i 10"#,
                   r#"print i"#,
                   r#"op add i i 1"#,
                   r#"jump 4 lessThan i 10"#,
                   r#"print "loop""#,
                   r#"print "start*2""#,
                   r#"set i 0"#,
                   r#"jump 14 greaterThanEq i 10"#,
                   r#"print i"#,
                   r#"op add i i 1"#,
                   r#"jump 11 lessThan i 10"#,
                   r#"printflush message1"#,
        ]);
    }

    #[test]
    fn sets_test() {
        let parser = TopLevelParser::new();

        let ast = parse!(parser, r#"
        a b c = 1 2 (op $ 2 + 1;);
        "#).unwrap();
        let meta = CompileMeta::new();
        let mut tag_codes = meta.compile(ast);
        let logic_lines = tag_codes.compile().unwrap();
        assert_eq!(logic_lines, vec![
                   "set a 1",
                   "set b 2",
                   "op add __0 2 1",
                   "set c __0",
        ]);

        assert!(parse!(parser, r#"
        a b c = 1 2;
        "#).is_err());

        assert!(parse!(parser, r#"
        a = 1 2;
        "#).is_err());

        assert!(parse!(parser, r#"
         = 1 2;
        "#).is_err());
    }

    #[test]
    fn const_value_clone_test() {
        let parser = TopLevelParser::new();

        let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
        const A = 1;
        const B = A;
        const A = 2;
        print A B;
        "#).unwrap()).compile().unwrap();
        assert_eq!(logic_lines, vec![
                   "print 2",
                   "print 1",
        ]);

        let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
        const A = 1;
        const B = A;
        const A = 2;
        const C = B;
        const B = 3;
        const B = B;
        print A B C;
        "#).unwrap()).compile().unwrap();
        assert_eq!(logic_lines, vec![
                   "print 2",
                   "print 3",
                   "print 1",
        ]);

        let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
        const A = B;
        const B = 2;
        print A;
        "#).unwrap()).compile().unwrap();
        assert_eq!(logic_lines, vec![
                   "print B",
        ]);

        let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
        const A = B;
        const B = 2;
        const A = A;
        print A;
        "#).unwrap()).compile().unwrap();
        assert_eq!(logic_lines, vec![
                   "print B",
        ]);

        let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
        const A = B;
        const B = 2;
        {
            const A = A;
            print A;
        }
        "#).unwrap()).compile().unwrap();
        assert_eq!(logic_lines, vec![
                   "print B",
        ]);

        let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
        const A = B;
        {
            const B = 2;
            const A = A;
            print A;
        }
        "#).unwrap()).compile().unwrap();
        assert_eq!(logic_lines, vec![
                   "print B",
        ]);

        let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
        const A = B;
        {
            const B = 2;
            print A;
        }
        "#).unwrap()).compile().unwrap();
        assert_eq!(logic_lines, vec![
                   "print B",
        ]);

        let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
        const A = B;
        const B = C;
        const C = A;
        print C;
        "#).unwrap()).compile().unwrap();
        assert_eq!(logic_lines, vec![
                   "print B",
        ]);

        let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
        const A = C;
        const C = 2;
        const B = A;
        const A = 3;
        const C = B;
        print C;
        "#).unwrap()).compile().unwrap();
        assert_eq!(logic_lines, vec![
                   "print C",
        ]);
    }

    #[test]
    fn cmptree_test() {
        let parser = TopLevelParser::new();

        let ast = parse!(parser, r#"
        goto :end a && b && c;
        foo;
        :end
        end;
        "#).unwrap();
        assert_eq!(
            ast[0].as_goto().unwrap().1,
            CmpTree::And(
                CmpTree::And(
                    Box::new(JumpCmp::NotEqual("a".into(), Value::ReprVar(FALSE_VAR.into()).into()).into()),
                    Box::new(JumpCmp::NotEqual("b".into(), Value::ReprVar(FALSE_VAR.into()).into()).into()),
                ).into(),
                Box::new(JumpCmp::NotEqual("c".into(), Value::ReprVar(FALSE_VAR.into()).into()).into()),
            ).into()
        );

        let ast = parse!(parser, r#"
        goto :end a || b || c;
        foo;
        :end
        end;
        "#).unwrap();
        assert_eq!(
            ast[0].as_goto().unwrap().1,
            CmpTree::Or(
                CmpTree::Or(
                    Box::new(JumpCmp::NotEqual("a".into(), Value::ReprVar(FALSE_VAR.into()).into()).into()),
                    Box::new(JumpCmp::NotEqual("b".into(), Value::ReprVar(FALSE_VAR.into()).into()).into()),
                ).into(),
                Box::new(JumpCmp::NotEqual("c".into(), Value::ReprVar(FALSE_VAR.into()).into()).into()),
            ).into()
        );

        let ast = parse!(parser, r#"
        goto :end a && b || c && d;
        foo;
        :end
        end;
        "#).unwrap();
        assert_eq!(
            ast[0].as_goto().unwrap().1,
            CmpTree::Or(
                CmpTree::And(
                    Box::new(JumpCmp::NotEqual("a".into(), Value::ReprVar(FALSE_VAR.into()).into()).into()),
                    Box::new(JumpCmp::NotEqual("b".into(), Value::ReprVar(FALSE_VAR.into()).into()).into()),
                ).into(),
                CmpTree::And(
                    Box::new(JumpCmp::NotEqual("c".into(), Value::ReprVar(FALSE_VAR.into()).into()).into()),
                    Box::new(JumpCmp::NotEqual("d".into(), Value::ReprVar(FALSE_VAR.into()).into()).into()),
                ).into(),
            ).into()
        );

        let ast = parse!(parser, r#"
        goto :end a && (b || c) && d;
        foo;
        :end
        end;
        "#).unwrap();
        assert_eq!(
            ast[0].as_goto().unwrap().1,
            CmpTree::And(
                CmpTree::And(
                    Box::new(JumpCmp::NotEqual("a".into(), Value::ReprVar(FALSE_VAR.into()).into()).into()),
                    CmpTree::Or(
                        Box::new(JumpCmp::NotEqual("b".into(), Value::ReprVar(FALSE_VAR.into()).into()).into()),
                        Box::new(JumpCmp::NotEqual("c".into(), Value::ReprVar(FALSE_VAR.into()).into()).into()),
                    ).into(),
                ).into(),
                Box::new(JumpCmp::NotEqual("d".into(), Value::ReprVar(FALSE_VAR.into()).into()).into()),
            ).into()
        );

        let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
        goto :end a && b;
        foo;
        :end
        end;
        "#).unwrap()).compile().unwrap();
        assert_eq!(logic_lines, vec![
                   "jump 2 equal a false",
                   "jump 3 notEqual b false",
                   "foo",
                   "end",
        ]);

        let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
        goto :end (a || b) && c;
        foo;
        :end
        end;
        "#).unwrap()).compile().unwrap();
        assert_eq!(logic_lines, vec![
                   "jump 2 notEqual a false",
                   "jump 3 equal b false",
                   "jump 4 notEqual c false",
                   "foo",
                   "end",
        ]);

        let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
        goto :end (a || b) && (c || d);
        foo;
        :end
        end;
        "#).unwrap()).compile().unwrap();
        assert_eq!(logic_lines, vec![
                   "jump 2 notEqual a false",
                   "jump 4 equal b false",
                   "jump 5 notEqual c false",
                   "jump 5 notEqual d false",
                   "foo",
                   "end",
        ]);

        let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
        goto :end a || b || c || d || e;
        foo;
        :end
        end;
        "#).unwrap()).compile().unwrap();
        assert_eq!(logic_lines, vec![
                   "jump 6 notEqual a false",
                   "jump 6 notEqual b false",
                   "jump 6 notEqual c false",
                   "jump 6 notEqual d false",
                   "jump 6 notEqual e false",
                   "foo",
                   "end",
        ]);

        let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
        goto :end a && b && c && d && e;
        foo;
        :end
        end;
        "#).unwrap()).compile().unwrap();
        assert_eq!(logic_lines, vec![
                   "jump 5 equal a false",
                   "jump 5 equal b false",
                   "jump 5 equal c false",
                   "jump 5 equal d false",
                   "jump 6 notEqual e false",
                   "foo",
                   "end",
        ]);

        let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
        goto :end (a && b && c) && d && e;
        foo;
        :end
        end;
        "#).unwrap()).compile().unwrap();
        assert_eq!(logic_lines, vec![
                   "jump 5 equal a false",
                   "jump 5 equal b false",
                   "jump 5 equal c false",
                   "jump 5 equal d false",
                   "jump 6 notEqual e false",
                   "foo",
                   "end",
        ]);

        let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
        goto :end a && b && (c && d && e);
        foo;
        :end
        end;
        "#).unwrap()).compile().unwrap();
        assert_eq!(logic_lines, vec![
                   "jump 5 equal a false",
                   "jump 5 equal b false",
                   "jump 5 equal c false",
                   "jump 5 equal d false",
                   "jump 6 notEqual e false",
                   "foo",
                   "end",
        ]);

        let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
        goto :end a && (op $ b && c;);
        foo;
        :end
        end;
        "#).unwrap()).compile().unwrap();
        assert_eq!(logic_lines, vec![
                   "jump 3 equal a false",
                   "op land __0 b c",
                   "jump 4 notEqual __0 false",
                   "foo",
                   "end",
        ]);

        let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
        goto :end a && b || c && d;
        foo;
        :end
        end;
        "#).unwrap()).compile().unwrap();
        assert_eq!(logic_lines, vec![
                   "jump 2 equal a false",
                   "jump 5 notEqual b false",
                   "jump 4 equal c false",
                   "jump 5 notEqual d false",
                   "foo",
                   "end",
        ]);

        let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
        goto :end !a && b || c && d;
        foo;
        :end
        end;
        "#).unwrap()).compile().unwrap();
        assert_eq!(logic_lines, vec![
                   "jump 2 notEqual a false",
                   "jump 5 notEqual b false",
                   "jump 4 equal c false",
                   "jump 5 notEqual d false",
                   "foo",
                   "end",
        ]);

        let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
        goto :end (a && b) || !(c && d);
        foo;
        :end
        end;
        "#).unwrap()).compile().unwrap();
        assert_eq!(logic_lines, vec![
                   "jump 2 equal a false",
                   "jump 5 notEqual b false",
                   "jump 5 equal c false",
                   "jump 5 equal d false",
                   "foo",
                   "end",
        ]);

        let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
        goto :end (a && b && c) || (d && e);
        foo;
        :end
        end;
        "#).unwrap()).compile().unwrap();
        assert_eq!(logic_lines, vec![
                   "jump 3 equal a false",
                   "jump 3 equal b false",
                   "jump 6 notEqual c false",
                   "jump 5 equal d false",
                   "jump 6 notEqual e false",
                   "foo",
                   "end",
        ]);

        let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
        goto :end (a && b || c) || (d && e);
        foo;
        :end
        end;
        "#).unwrap()).compile().unwrap();
        assert_eq!(logic_lines, vec![
                   "jump 2 equal a false",
                   "jump 6 notEqual b false",
                   "jump 6 notEqual c false",
                   "jump 5 equal d false",
                   "jump 6 notEqual e false",
                   "foo",
                   "end",
        ]);

        let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
        goto :end ((a && b) || c) || (d && e);
        foo;
        :end
        end;
        "#).unwrap()).compile().unwrap();
        assert_eq!(logic_lines, vec![
                   "jump 2 equal a false",
                   "jump 6 notEqual b false",
                   "jump 6 notEqual c false",
                   "jump 5 equal d false",
                   "jump 6 notEqual e false",
                   "foo",
                   "end",
        ]);

        let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
        goto :end (a && (b || c)) || (d && e);
        foo;
        :end
        end;
        "#).unwrap()).compile().unwrap();
        assert_eq!(logic_lines, vec![
                   "jump 3 equal a false",
                   "jump 6 notEqual b false",
                   "jump 6 notEqual c false",
                   "jump 5 equal d false",
                   "jump 6 notEqual e false",
                   "foo",
                   "end",
        ]);

        let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
        goto :end (op $ a + 2;) && (op $ b + 2;);
        foo;
        :end
        end;
        "#).unwrap()).compile().unwrap();
        assert_eq!(logic_lines, vec![
                   "op add __0 a 2",
                   "jump 4 equal __0 false",
                   "op add __1 b 2",
                   "jump 5 notEqual __1 false",
                   "foo",
                   "end",
        ]);
    }

    #[test]
    fn set_res_test() {
        let parser = TopLevelParser::new();

        let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
        print (setres (x: op $ 1 + 2;););
        "#).unwrap()).compile().unwrap();
        assert_eq!(logic_lines, vec![
                   "op add x 1 2",
                   "print x",
        ]);

        let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
        print (setres m;);
        "#).unwrap()).compile().unwrap();
        assert_eq!(logic_lines, vec![
                   "print m",
        ]);
    }

    #[test]
    fn repr_var_test() {
        let parser = TopLevelParser::new();

        let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
        print a;
        print `a`;
        const a = b;
        print a;
        print `a`;
        "#).unwrap()).compile().unwrap();
        assert_eq!(logic_lines, vec![
                   "print a",
                   "print a",
                   "print b",
                   "print a",
        ]);
    }

    #[test]
    fn select_test() {
        let parser = TopLevelParser::new();

        let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
        select 1 {
            print 0;
            print 1 " is one!";
            print 2;
        }
        "#).unwrap()).compile().unwrap();
        assert_eq!(logic_lines, vec![
                   "op mul __0 1 2",
                   "op add @counter @counter __0",
                   "print 0",
                   "noop",
                   "print 1",
                   "print \" is one!\"",
                   "print 2",
                   "noop",
        ]);

        let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
        select x {
            print 0;
            print 1;
            print 2;
        }
        "#).unwrap()).compile().unwrap();
        assert_eq!(logic_lines, vec![
                   "op add @counter @counter x",
                   "print 0",
                   "print 1",
                   "print 2",
        ]);

        let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
        select (y: op $ x + 2;) {}
        "#).unwrap()).compile().unwrap();
        assert_eq!(logic_lines, vec![
                   "op add y x 2",
        ]);

        let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
        select x {}
        "#).unwrap()).compile().unwrap();
        assert_eq!(logic_lines, Vec::<&str>::new());

    }

    #[test]
    fn switch_catch_test() {
        let parser = TopLevelParser::new();

        let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
        switch (op $ x + 2;) {
            end;
        case <:
            print "Underflow";
            stop;
        case ! e:
            print "Misses: " e;
            stop;
        case > n:
            print "Overflow: " n;
            stop;
        case 1:
            print 1;
        case 3:
            print 3 "!";
        }
        "#).unwrap()).compile().unwrap();
        // 这里由于是比较生成后的代码而不是语法树, 所以可以不用那么严谨
        assert_eq!(logic_lines, CompileMeta::new().compile(parse!(parser, r#"
        take tmp = (op $ x + 2;);
        skip tmp >= 0 {
            print "Underflow";
            stop;
        }
        skip _ {
            :mis
            const e = tmp;
            print "Misses: " e;
            stop;
        }
        skip tmp <= 3 {
            const n = tmp;
            print "Overflow: " n;
            stop;
        }
        select tmp {
            goto :mis _;
            {
                print 1;
                end;
            }
            goto :mis _;
            {
                print 3 "!";
                end;
            }
        }
        "#).unwrap()).compile().unwrap());

        let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
        switch (op $ x + 2;) {
            end;
        case <!>:
            stop;
        case 1:
            print 1;
        case 3:
            print 3 "!";
        }
        "#).unwrap()).compile().unwrap();
        // 这里由于是比较生成后的代码而不是语法树, 所以可以不用那么严谨
        assert_eq!(logic_lines, CompileMeta::new().compile(parse!(parser, r#"
        take tmp = (op $ x + 2;);
        skip tmp >= 0 && tmp <= 3 {
            :mis
            stop;
        }
        select tmp {
            goto :mis _;
            {
                print 1;
                end;
            }
            goto :mis _;
            {
                print 3 "!";
                end;
            }
        }
        "#).unwrap()).compile().unwrap());

        let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
        switch (op $ x + 2;) {
            end;
        case <!>:
            stop;
        case (a < b):
            foo;
        case 1:
            print 1;
        case 3:
            print 3 "!";
        }
        "#).unwrap()).compile().unwrap();
        assert_eq!(logic_lines, CompileMeta::new().compile(parse!(parser, r#"
        take tmp = (op $ x + 2;);
        skip tmp >= 0 && tmp <= 3 {
            :mis
            stop;
        }
        skip !a < b {
            foo;
        }
        select tmp {
            goto :mis _;
            {
                print 1;
                end;
            }
            goto :mis _;
            {
                print 3 "!";
                end;
            }
        }
        "#).unwrap()).compile().unwrap());

        let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
        switch (op $ x + 2;) {
            end;
        case !!!: # 捕获多个未命中也可以哦, 当然只有最后一个生效
            stop;
        case 1:
            print 1;
        case 3:
            print 3 "!";
        }
        "#).unwrap()).compile().unwrap();
        assert_eq!(logic_lines, CompileMeta::new().compile(parse!(parser, r#"
        take tmp = (op $ x + 2;);
        skip _ {
            :mis
            stop;
        }
        select tmp {
            goto :mis _;
            {
                print 1;
                end;
            }
            goto :mis _;
            {
                print 3 "!";
                end;
            }
        }
        "#).unwrap()).compile().unwrap());

        let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
        switch (op $ x + 2;) {
            end;
        case !!!: # 捕获多个未命中也可以哦, 当然只有最后一个生效
            stop;
        case !:
            foo; # 最后一个
        case 1:
            print 1;
        case 3:
            print 3 "!";
        }
        "#).unwrap()).compile().unwrap();
        assert_eq!(logic_lines, CompileMeta::new().compile(parse!(parser, r#"
        take tmp = (op $ x + 2;);
        skip _ {
            # 可以看出, 这个是一个没用的捕获, 也不会被跳转
            # 所以不要这么玩, 浪费跳转和行数
            :mis
            stop;
        }
        skip _ {
            :mis1
            foo;
        }
        select tmp {
            goto :mis1 _;
            {
                print 1;
                end;
            }
            goto :mis1 _;
            {
                print 3 "!";
                end;
            }
        }
        "#).unwrap()).compile().unwrap());

        let ast = parse!(parser, r#"
        switch (op $ x + 2;) {
            end;
        case <!> e:
            `e` = e;
            stop;
        case (e < x) e:
            foo;
        case 1:
            print 1;
        case 3:
            print 3;
        }
        "#).unwrap();
        assert_eq!(ast, parse!(parser, r#"
        {
            take ___0 = (op $ x + 2;);
            {
                {
                    const e = ___0;
                    goto :___1 ___0 >= `0` && ___0 <= `3`;
                    :___0
                    {
                        `e` = e;
                        stop;
                    }
                    :___1
                }
                {
                    const e = ___0;
                    goto :___2 ! e < x;
                    {
                        foo;
                    }
                    :___2
                }
            }
            select ___0 {
                goto :___0 _;
                {
                    print 1;
                    end;
                }
                goto :___0 _;
                {
                    print 3;
                    end;
                }
            }
        }
        "#).unwrap());

        let ast = parse!(parser, r#"
        switch (op $ x + 2;) {
            end;
        case <> e:
            `e` = e;
            stop;
        case (e < x) e:
            foo;
        case 1:
            print 1;
        case 3:
            print 3;
        }
        "#).unwrap();
        assert_eq!(ast, parse!(parser, r#"
        {
            take ___0 = (op $ x + 2;);
            {
                {
                    const e = ___0;
                    goto :___0 ___0 >= `0` && ___0 <= `3`;
                    {
                        `e` = e;
                        stop;
                    }
                    :___0
                }
                {
                    const e = ___0;
                    goto :___1 ! e < x;
                    {
                        foo;
                    }
                    :___1
                }
            }
            select ___0 {
                { end; }
                {
                    print 1;
                    end;
                }
                { end; }
                {
                    print 3;
                    end;
                }
            }
        }
        "#).unwrap());

        let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
        switch (op $ x + 2;) {
        case !:
            stop;
        case 1:
        case 3:
        }
        "#).unwrap()).compile().unwrap();
        assert_eq!(logic_lines, CompileMeta::new().compile(parse!(parser, r#"
        take tmp = (op $ x + 2;);
        skip _ {
            :mis
            stop;
        }
        select tmp {
            goto :mis _;
            noop;
            goto :mis _;
            noop;
        }
        "#).unwrap()).compile().unwrap());
    }

    #[test]
    fn display_source_test() {
        let mut meta = Default::default();
        assert_eq!(
            parse!(
                LogicLineParser::new(),
                r#"'abc' 'abc"def' "str" "str'str" 'no_str' '2';"#
            )
                .unwrap()
                .display_source_and_get(&mut meta),
            r#"'abc' 'abc"def' "str" "str'str" 'no_str' '2';"#
        );
        assert_eq!(
            JumpCmp::GreaterThan("a".into(), "1".into())
                .display_source_and_get(&mut meta),
            "'a' > '1'"
        );
        assert_eq!(
            parse!(JumpCmpParser::new(), "a < b && c < d && e < f")
                .unwrap()
                .display_source_and_get(&mut meta),
            "(('a' < 'b' && 'c' < 'd') && 'e' < 'f')"
        );
        assert_eq!(
            parse!(LogicLineParser::new(), "{foo;}")
                .unwrap()
                .display_source_and_get(&mut meta),
            "{\n    'foo';\n}"
        );
        assert_eq!(
            parse!(LogicLineParser::new(), "print ($ = x;);")
                .unwrap()
                .display_source_and_get(&mut meta),
            "`'print'` (`'set'` $ 'x';);"
        );
        assert_eq!(
            parse!(LogicLineParser::new(), "print (res: $ = x;);")
                .unwrap()
                .display_source_and_get(&mut meta),
            "`'print'` ('res': `'set'` $ 'x';);"
        );
        assert_eq!(
            parse!(LogicLineParser::new(), "print (noop;$ = x;);")
                .unwrap()
                .display_source_and_get(&mut meta),
            "`'print'` (\n    noop;\n    `'set'` $ 'x';\n);"
        );
        assert_eq!(
            parse!(LogicLineParser::new(), "print (res: noop;$ = x;);")
                .unwrap()
                .display_source_and_get(&mut meta),
            "`'print'` ('res':\n    noop;\n    `'set'` $ 'x';\n);"
        );
        assert_eq!(
            parse!(LogicLineParser::new(), "print a.b.c;")
                .unwrap()
                .display_source_and_get(&mut meta),
            "`'print'` 'a'.'b'.'c';"
        );
        assert_eq!(
            parse!(LogicLineParser::new(), "op add a b c;")
                .unwrap()
                .display_source_and_get(&mut meta),
            "op 'a' 'b' + 'c';"
        );
        assert_eq!(
            parse!(LogicLineParser::new(), "op x noise a b;")
                .unwrap()
                .display_source_and_get(&mut meta),
            "op 'x' noise 'a' 'b';"
        );
    }

    #[test]
    fn quick_dexp_take_test() {
        let parser = TopLevelParser::new();

        assert_eq!(
            parse!(parser, r#"
                print Foo[1 2];
            "#).unwrap(),
            parse!(parser, r#"
                print (__:
                    const _0 = 1;
                    const _1 = 2;
                    setres Foo;
                );
            "#).unwrap(),
        );


        let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
        const Add = (
            take A = _0;
            take B = _1;
            op $ A + B;
        );
        print Add[1 2];
        "#).unwrap()).compile().unwrap();
        assert_eq!(logic_lines, vec![
                   "op add __0 1 2",
                   "print __0",
        ]);

        let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
        const Add = (
            take A = _0;
            take B = _1;
            op $ A + B;
        );
        const Do = (_unused:
            const Fun = _0;

            print enter Fun;
        );
        take[Add[1 2]] Do;
        "#).unwrap()).compile().unwrap();
        assert_eq!(logic_lines, vec![
                   "print enter",
                   "op add __0 1 2",
                   "print __0",
        ]);

    }

    #[test]
    fn value_bind_test() {
        let parser = TopLevelParser::new();

        let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
        const Jack = jack;
        Jack Jack.age = "jack" 18;
        print Jack Jack.age;
        "#).unwrap()).compile().unwrap();
        assert_eq!(logic_lines, vec![
                   "set jack \"jack\"",
                   "set __jack__bind__age 18",
                   "print jack",
                   "print __jack__bind__age",
        ]);

        let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
        print a.b.c;
        "#).unwrap()).compile().unwrap();
        assert_eq!(logic_lines, vec![
                   "print ____a__bind__b__bind__c",
        ]);

    }

    #[test]
    fn no_string_var_test() {
        let parser = NoStringVarParser::new();

        assert!(parse!(parser, r#"1"#).is_ok());
        assert!(parse!(parser, r#"1.5"#).is_ok());
        assert!(parse!(parser, r#"sbosb"#).is_ok());
        assert!(parse!(parser, r#"0x1b"#).is_ok());
        assert!(parse!(parser, r#"@abc"#).is_ok());
        assert!(parse!(parser, r#"'My_name"s'"#).is_ok());
        assert!(parse!(parser, r#"'"no_str"'"#).is_ok());

        assert!(parse!(parser, r#""abc""#).is_err());
        assert!(parse!(parser, r#""""#).is_err());
    }

    #[test]
    fn jumpcmp_from_str_test() {
        let datas = [
            ("always", Err(JumpCmpRParseError::ArgsCountError(
                vec!["always".into()]
            ).into())),
            ("always 0", Err(JumpCmpRParseError::ArgsCountError(
                vec!["always".into(), "0".into()]
            ).into())),
            ("add 1 2", Err(JumpCmpRParseError::UnknownComparer(
                "add".into(),
                ["1".into(), "2".into()]
            ).into())),
            ("equal a b", Ok(JumpCmp::Equal("a".into(), "b".into()))),
            ("lessThan a b", Ok(JumpCmp::LessThan("a".into(), "b".into()))),
            ("always 0 0", Ok(JumpCmp::Always)),
        ];

        for (src, expect) in datas {
            assert_eq!(JumpCmp::from_mdt_args(&mdt_logic_split(src).unwrap()), expect)
        }
    }

    #[test]
    fn logic_line_from() {
        type Error = (usize, LogicLineFromTagError);
        let datas: [(&str, Result<Vec<LogicLine>, Error>); 2] = [
            (
                "op add i i 1",
                Ok(vec![
                   Op::Add("i".into(), "i".into(), "1".into()).into(),
                ])
            ),
            (
                "op add i i 1\njump 0 lessThan i 10",
                Ok(vec![
                   LogicLine::Label("0".into()).into(),
                   Op::Add("i".into(), "i".into(), "1".into()).into(),
                   Goto("0".into(), JumpCmp::LessThan("i".into(), "10".into()).into()).into(),
                ])
            ),
        ];
        for (src, lines2) in datas {
            let mut tag_codes = TagCodes::from_str(src).unwrap();
            tag_codes.build_tagdown().unwrap();
            tag_codes.tag_up();
            assert_eq!(
                (&tag_codes).try_into(),
                lines2.map(Expand)
            );
        }
    }

    #[test]
    fn op_expr_test() {
        let parser = TopLevelParser::new();

        assert_eq!(
            parse!(parser, r#"
            x = max(1, 2);
            y = max(max(1, 2), max(3, max(4, 5)));
            "#).unwrap(),
            parse!(parser, r#"
            op x max 1 2;
            op y max (op $ max 1 2;) (op $ max 3 (op $ max 4 5;););
            "#).unwrap(),
        );

        assert_eq!(
            parse!(parser, r#"
            x = 1+2*3;
            y = (1+2)*3;
            z = 1+2+3;
            "#).unwrap(),
            parse!(parser, r#"
            op x 1 + (op $ 2 * 3;);
            op y (op $ 1 + 2;) * 3;
            op z (op $ 1 + 2;) + 3;
            "#).unwrap(),
        );

        assert_eq!(
            parse!(parser, r#"
            x = 1*max(2, 3);
            y = a & b | c & d & e | f;
            "#).unwrap(),
            parse!(parser, r#"
            op x 1 * (op $ max 2 3;);
            op y (op $ (op $ a & b;) | (op $ (op $ c & d;) & e;);) | f;
            "#).unwrap(),
        );

        assert_eq!(
            parse!(parser, r#"
            x = a**b**c; # pow的右结合
            y = -x;
            z = ~y;
            e = a !== b;
            "#).unwrap(),
            parse!(parser, r#"
            op x a ** (op $ b ** c;);
            op y `0` - x;
            op z ~y;
            op e (op $ a === b;) == `false`;
            "#).unwrap(),
        );

        assert_eq!(
            parse!(parser, r#"
            a, b, c = x, -y, z+2*3;
            "#).unwrap(),
            parse!(parser, r#"
            {
                a = x;
                b = -y;
                c = z+2*3;
            }
            "#).unwrap(),
        );

    }

    #[test]
    fn op_test() {
        let parser = TopLevelParser::new();

        assert_eq!(
            parse!(parser, r#"
            op x a !== b;
            "#).unwrap(),
            parse!(parser, r#"
            op x (op $ a === b;) == `false`;
            "#).unwrap(),
        );

    }

    #[test]
    fn inline_block_test() {
        let parser = TopLevelParser::new();

        assert_eq!(
            parse!(parser, r#"
            inline {
                foo;
            }
            "#).unwrap(),
            Expand(vec![
                InlineBlock(vec![
                    LogicLine::Other(vec!["foo".into()])
                ]).into()
            ]).into()
        );

        let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
        print A;
        inline {
            const A = 2;
            print A;
        }
        print A;
        "#).unwrap()).compile().unwrap();
        assert_eq!(logic_lines, vec![
                   "print A",
                   "print 2",
                   "print 2",
        ]);
    }

    #[test]
    fn consted_dexp() {
        let parser = TopLevelParser::new();

        assert_eq!(
            parse!(parser, r#"
            foo const(:x bar;);
            "#).unwrap(),
            Expand(vec![
                LogicLine::Other(vec![
                    "foo".into(),
                    DExp::new(
                        "__".into(),
                        vec![
                            Const(
                                "___0".into(),
                                DExp::new_nores(vec![
                                    LogicLine::Label("x".into()),
                                    LogicLine::Other(vec!["bar".into()])
                                ].into()).into(),
                                vec!["x".into()],
                            ).into(),
                            LogicLine::SetResultHandle("___0".into()),
                        ].into()
                    ).into()
                ]),
            ]).into()
        );

        let logic_lines = CompileMeta::new().compile(parse!(parser, r#"
        const Do2 = (
            const F = _0;
            take F;
            take F;
        );
        take[
            const(
                if a < b {
                    print 1;
                } else {
                    print 2;
                }
            )
        ] Do2;
        "#).unwrap()).compile().unwrap();
        assert_eq!(logic_lines, vec![
                   "jump 3 lessThan a b",
                   "print 2",
                   "jump 4 always 0 0",
                   "print 1",
                   "jump 7 lessThan a b",
                   "print 2",
                   "jump 0 always 0 0",
                   "print 1",
        ]);

        assert!(CompileMeta::new().compile(parse!(parser, r#"
        const Do2 = (
            const F = _0;
            take F;
            take F;
        );
        take[
            (
                if a < b {
                    print 1;
                } else {
                    print 2;
                }
            )
        ] Do2;
        "#).unwrap()).compile().is_err());
    }

    #[test]
    fn op_into_cmp_test() {
        assert_eq!(
            Op::Add("a".into(), "b".into(), "c".into()).try_into_cmp(),
            None,
        );
        assert_eq!(
            Op::Add(Value::ResultHandle, "b".into(), "c".into()).try_into_cmp(),
            None,
        );
        assert_eq!(
            Op::Land("a".into(), "b".into(), "c".into()).try_into_cmp(),
            None,
        );
        assert_eq!(
            Op::Land(Value::ResultHandle, "b".into(), "c".into()).try_into_cmp(),
            None,
        );
        assert_eq!(
            Op::LessThan("a".into(), "b".into(), "c".into()).try_into_cmp(),
            None,
        );
        assert_eq!(
            Op::LessThan(Value::ResultHandle, "b".into(), "c".into()).try_into_cmp(),
            Some(JumpCmp::LessThan("b".into(), "c".into())),
        );
        assert_eq!(
            Op::StrictEqual(Value::ResultHandle, "b".into(), "c".into()).try_into_cmp(),
            Some(JumpCmp::StrictEqual("b".into(), "c".into())),
        );
    }

    #[test]
    fn inline_cmp_op_test() {
        let parser = TopLevelParser::new();

        assert_eq!(
            CompileMeta::new().compile(parse!(parser, r#"
            :0 goto :0 a < b;
            "#).unwrap()).compile().unwrap(),
            vec![
                "jump 0 lessThan a b"
            ]
        );

        assert_eq!(
            CompileMeta::new().compile(parse!(parser, r#"
            :0 goto :0 a < b;
            "#).unwrap()).compile().unwrap(),
            CompileMeta::new().compile(parse!(parser, r#"
            :0 goto :0 a < b;
            "#).unwrap()).compile().unwrap(),
        );

        assert_eq!(
            CompileMeta::new().compile(parse!(parser, r#"
            :0 goto :0 (op $ a < b;);
            "#).unwrap()).compile().unwrap(),
            CompileMeta::new().compile(parse!(parser, r#"
            :0 goto :0 a < b;
            "#).unwrap()).compile().unwrap(),
        );

        assert_eq!(
            CompileMeta::new().compile(parse!(parser, r#"
            :0 goto :0 (op $ a === b;);
            "#).unwrap()).compile().unwrap(),
            CompileMeta::new().compile(parse!(parser, r#"
            :0 goto :0 a === b;
            "#).unwrap()).compile().unwrap(),
        );

        assert_eq!(
            CompileMeta::new().compile(parse!(parser, r#"
            :0 goto :0 (x: op x a === b;);
            "#).unwrap()).compile().unwrap(),
            CompileMeta::new().compile(parse!(parser, r#"
            :0 goto :0 (x: op x a === b;);
            "#).unwrap()).compile().unwrap(),
        );

        assert_eq!(
            CompileMeta::new().compile(parse!(parser, r#"
            :0 goto :0 (op x a === b;);
            "#).unwrap()).compile().unwrap(),
            CompileMeta::new().compile(parse!(parser, r#"
            :0 goto :0 (op x a === b;);
            "#).unwrap()).compile().unwrap(),
        );

        assert_eq!(
            CompileMeta::new().compile(parse!(parser, r#"
            :0 goto :0 (x: op $ a === b;);
            "#).unwrap()).compile().unwrap(),
            CompileMeta::new().compile(parse!(parser, r#"
            :0 goto :0 (x: op $ a === b;);
            "#).unwrap()).compile().unwrap(),
        );

        assert_eq!(
            CompileMeta::new().compile(parse!(parser, r#"
            :0 goto :0 !(op $ a < b;);
            "#).unwrap()).compile().unwrap(),
            CompileMeta::new().compile(parse!(parser, r#"
            :0 goto :0 !a < b;
            "#).unwrap()).compile().unwrap(),
        );

        assert_eq!(
            CompileMeta::new().compile(parse!(parser, r#"
            :0 goto :0 !!!(op $ a < b;);
            "#).unwrap()).compile().unwrap(),
            CompileMeta::new().compile(parse!(parser, r#"
            :0 goto :0 !!!a < b;
            "#).unwrap()).compile().unwrap(),
        );

        // 暂未实现直接到StrictNotEqual, 目前这就算了吧, 反正最终编译产物一样
        assert_eq!(
            CompileMeta::new().compile(parse!(parser, r#"
            :0 goto :0 !(op $ a === b;);
            "#).unwrap()).compile().unwrap(),
            CompileMeta::new().compile(parse!(parser, r#"
            :0 goto :0 a !== b;
            "#).unwrap()).compile().unwrap(),
        );

        assert_eq!(
            CompileMeta::new().compile(parse!(parser, r#"
            :0 goto :0 (noop; op $ a < b;);
            "#).unwrap()).compile().unwrap(),
            CompileMeta::new().compile(parse!(parser, r#"
            :0 goto :0 (noop; op $ a < b;);
            "#).unwrap()).compile().unwrap(),
        );

        assert_eq!(
            CompileMeta::new().compile(parse!(parser, r#"
            :0 goto :0 (op $ a < b; noop;);
            "#).unwrap()).compile().unwrap(),
            CompileMeta::new().compile(parse!(parser, r#"
            :0 goto :0 (op $ a < b; noop;);
            "#).unwrap()).compile().unwrap(),
        );

        assert_eq!( // 连续内联的作用
            CompileMeta::new().compile(parse!(parser, r#"
            :0 goto :0 (op $ !(op $ a < b;););
            "#).unwrap()).compile().unwrap(),
            CompileMeta::new().compile(parse!(parser, r#"
            :0 goto :0 !a < b;
            "#).unwrap()).compile().unwrap(),
        );

        assert_eq!( // 连续内联的作用
            CompileMeta::new().compile(parse!(parser, r#"
            :0 goto :0 (op $ !(op $ !(op $ a < b;);););
            "#).unwrap()).compile().unwrap(),
            CompileMeta::new().compile(parse!(parser, r#"
            :0 goto :0 a < b;
            "#).unwrap()).compile().unwrap(),
        );

    }

    #[test]
    fn top_level_break_and_continue_test() {
        let parser = TopLevelParser::new();

        assert_eq!(
            CompileMeta::new().compile(parse!(parser, r#"
            foo;
            continue;
            bar;
            "#).unwrap()).compile().unwrap(),
            [
                "foo",
                "jump 0 always 0 0",
                "bar",
            ]
        );

        assert_eq!(
            CompileMeta::new().compile(parse!(parser, r#"
            foo;
            continue _;
            bar;
            "#).unwrap()).compile().unwrap(),
            [
                "foo",
                "jump 0 always 0 0",
                "bar",
            ]
        );

        assert_eq!(
            CompileMeta::new().compile(parse!(parser, r#"
            foo;
            continue a < b;
            bar;
            "#).unwrap()).compile().unwrap(),
            [
                "foo",
                "jump 0 lessThan a b",
                "bar",
            ]
        );

        assert_eq!(
            CompileMeta::new().compile(parse!(parser, r#"
            foo;
            continue a < b || c < d;
            bar;
            "#).unwrap()).compile().unwrap(),
            [
                "foo",
                "jump 0 lessThan a b",
                "jump 0 lessThan c d",
                "bar",
            ]
        );

        assert_eq!(
            CompileMeta::new().compile(parse!(parser, r#"
            foo;
            continue;
            bar;
            "#).unwrap()).compile().unwrap(),
            [
                "foo",
                "jump 0 always 0 0",
                "bar",
            ]
        );

        assert_eq!(
            CompileMeta::new().compile(parse!(parser, r#"
            foo;
            continue _;
            bar;
            "#).unwrap()).compile().unwrap(),
            [
                "foo",
                "jump 0 always 0 0",
                "bar",
            ]
        );

        assert_eq!(
            CompileMeta::new().compile(parse!(parser, r#"
            foo;
            continue a < b;
            bar;
            "#).unwrap()).compile().unwrap(),
            [
                "foo",
                "jump 0 lessThan a b",
                "bar",
            ]
        );

        assert_eq!(
            CompileMeta::new().compile(parse!(parser, r#"
            foo;
            continue a < b || c < d;
            bar;
            "#).unwrap()).compile().unwrap(),
            [
                "foo",
                "jump 0 lessThan a b",
                "jump 0 lessThan c d",
                "bar",
            ]
        );

    }

    #[test]
    fn control_stmt_break_and_continue_test() {
        let parser = TopLevelParser::new();

        assert_eq!(
            CompileMeta::new().compile(parse!(parser, r#"
            foo;
            while a < b {
                foo1;
                while c < d {
                    foo2;
                    break;
                }
                bar1;
                break;
            }
            bar;
            break;
            "#).unwrap()).compile().unwrap(),
            [
                "foo",
                "jump 10 greaterThanEq a b",
                "foo1",
                "jump 7 greaterThanEq c d",
                "foo2",
                "jump 7 always 0 0",
                "jump 4 lessThan c d",
                "bar1",
                "jump 10 always 0 0",
                "jump 2 lessThan a b",
                "bar",
                "jump 0 always 0 0",
            ]
        );

        assert_eq!(
            CompileMeta::new().compile(parse!(parser, r#"
            foo;
            gwhile a < b {
                foo1;
                gwhile c < d {
                    foo2;
                    break;
                }
                bar1;
                break;
            }
            bar;
            break;
            "#).unwrap()).compile().unwrap(),
            [
                "foo",
                "jump 9 always 0 0",
                "foo1",
                "jump 6 always 0 0",
                "foo2",
                "jump 7 always 0 0",
                "jump 4 lessThan c d",
                "bar1",
                "jump 10 always 0 0",
                "jump 2 lessThan a b",
                "bar",
                "jump 0 always 0 0",
            ]
        );

        assert_eq!(
            CompileMeta::new().compile(parse!(parser, r#"
            foo;
            xxx;
            do {
                foo1;
                xxx;
                do {
                    foo2;
                    break;
                } while c < d;
                bar1;
                break;
            } while a < b;
            bar;
            break;
            "#).unwrap()).compile().unwrap(),
            [
                "foo",
                "xxx",
                "foo1",
                "xxx",
                "foo2",
                "jump 7 always 0 0",
                "jump 4 lessThan c d",
                "bar1",
                "jump 10 always 0 0",
                "jump 2 lessThan a b",
                "bar",
                "jump 0 always 0 0",
            ]
        );

        assert_eq!(
            CompileMeta::new().compile(parse!(parser, r#"
            switch a {
            case 0: foo;
            case 1: break;
            case 2: bar;
            }
            end;
            break;
            "#).unwrap()).compile().unwrap(),
            [
                "op add @counter @counter a",
                "foo",
                "jump 4 always 0 0",
                "bar",
                "end",
                "jump 0 always 0 0",
            ]
        );

        assert_eq!(
            CompileMeta::new().compile(parse!(parser, r#"
            select a {
                foo;
                break;
                bar;
            }
            end;
            break;
            "#).unwrap()).compile().unwrap(),
            [
                "op add @counter @counter a",
                "foo",
                "jump 4 always 0 0",
                "bar",
                "end",
                "jump 0 always 0 0",
            ]
        );

        assert_eq!(
            CompileMeta::new().compile(parse!(parser, r#"
            foo;
            while a < b {
                foo1;
                while c < d {
                    foo2;
                    continue;
                }
                bar1;
                continue;
            }
            bar;
            continue;
            "#).unwrap()).compile().unwrap(),
            [
                "foo",
                "jump 10 greaterThanEq a b",
                "foo1",
                "jump 7 greaterThanEq c d",
                "foo2",
                "jump 6 always 0 0",
                "jump 4 lessThan c d",
                "bar1",
                "jump 9 always 0 0",
                "jump 2 lessThan a b",
                "bar",
                "jump 0 always 0 0",
            ]
        );

        assert_eq!(
            CompileMeta::new().compile(parse!(parser, r#"
            foo;
            while a < b {
                foo1;
                while c < d {
                    continue;
                    foo2;
                }
                bar1;
                continue;
            }
            bar;
            continue;
            "#).unwrap()).compile().unwrap(),
            [
                "foo",
                "jump 10 greaterThanEq a b",
                "foo1",
                "jump 7 greaterThanEq c d",
                "jump 6 always 0 0",
                "foo2",
                "jump 6 lessThan c d", // 4 -> 6
                "bar1",
                "jump 9 always 0 0",
                "jump 2 lessThan a b",
                "bar",
                "jump 0 always 0 0",
            ]
        );

        assert_eq!(
            CompileMeta::new().compile(parse!(parser, r#"
            foo;
            gwhile a < b {
                foo1;
                gwhile c < d {
                    foo2;
                    continue;
                }
                bar1;
                continue;
            }
            bar;
            continue;
            "#).unwrap()).compile().unwrap(),
            [
                "foo",
                "jump 9 always 0 0",
                "foo1",
                "jump 6 always 0 0",
                "foo2",
                "jump 6 always 0 0",
                "jump 4 lessThan c d",
                "bar1",
                "jump 9 always 0 0",
                "jump 2 lessThan a b",
                "bar",
                "jump 0 always 0 0",
            ]
        );

        assert_eq!(
            CompileMeta::new().compile(parse!(parser, r#"
            foo;
            xxx;
            do {
                foo1;
                xxx;
                do {
                    foo2;
                    continue;
                } while c < d;
                bar1;
                continue;
            } while a < b;
            bar;
            continue;
            "#).unwrap()).compile().unwrap(),
            [
                "foo",
                "xxx",
                "foo1",
                "xxx",
                "foo2",
                "jump 6 always 0 0",
                "jump 4 lessThan c d",
                "bar1",
                "jump 9 always 0 0",
                "jump 2 lessThan a b",
                "bar",
                "jump 0 always 0 0",
            ]
        );

        assert_eq!(
            CompileMeta::new().compile(parse!(parser, r#"
            switch a {
            case 0: foo;
            case 1: continue;
            case 2: bar;
            }
            end;
            continue;
            "#).unwrap()).compile().unwrap(),
            [
                "op add @counter @counter a",
                "foo",
                "jump 0 always 0 0",
                "bar",
                "end",
                "jump 0 always 0 0",
            ]
        );

        assert_eq!(
            CompileMeta::new().compile(parse!(parser, r#"
            select a {
                foo;
                continue;
                bar;
            }
            end;
            continue;
            "#).unwrap()).compile().unwrap(),
            [
                "op add @counter @counter a",
                "foo",
                "jump 0 always 0 0",
                "bar",
                "end",
                "jump 0 always 0 0",
            ]
        );

        assert_eq!(
            CompileMeta::new().compile(parse!(parser, r#"
            end;
            switch a {
            case 0: foo;
            case 1: continue;
            case 2: bar;
            }
            end;
            continue;
            "#).unwrap()).compile().unwrap(),
            [
                "end",
                "op add @counter @counter a",
                "foo",
                "jump 1 always 0 0",
                "bar",
                "end",
                "jump 0 always 0 0",
            ]
        );

        assert_eq!(
            CompileMeta::new().compile(parse!(parser, r#"
            end;
            select a {
                foo;
                continue;
                bar;
            }
            end;
            continue;
            "#).unwrap()).compile().unwrap(),
            [
                "end",
                "op add @counter @counter a",
                "foo",
                "jump 1 always 0 0",
                "bar",
                "end",
                "jump 0 always 0 0",
            ]
        );

    }

    #[test]
    fn test_op_expr_if_else() {
        let parser = TopLevelParser::new();

        assert_eq!(
            parse!(parser, r#"
            a = if b < c ? b + 2 : c;
            "#).unwrap(),
            parse!(parser, r#"
            {
                take ___0 = a;
                goto :___0 b < c;
                set ___0 c;
                goto :___1 _;
                :___0
                op ___0 b + 2;
                :___1
            }
            "#).unwrap()
        );

        assert_eq!(
            parse!(parser, r#"
            a = (if b < c ? b + 2 : c);
            "#).unwrap(),
            parse!(parser, r#"
            {
                take ___0 = a;
                goto :___0 b < c;
                set ___0 c;
                goto :___1 _;
                :___0
                op ___0 b + 2;
                :___1
            }
            "#).unwrap()
        );

        assert_eq!(
            parse!(parser, r#"
            a = if b < c ? b + 2 : if d < e ? 8 : c - 2;
            "#).unwrap(),
            parse!(parser, r#"
            {
                take ___1 = a;
                goto :___2 b < c;
                {
                    take ___0 = ___1;
                    goto :___0 d < e;
                    op ___0 c - 2;
                    goto :___1 _;
                    :___0
                    set ___0 8;
                    :___1
                }
                goto :___3 _;
                :___2
                op ___1 b + 2;
                :___3
            }
            "#).unwrap()
        );

        assert_eq!(
            parse!(parser, r#"
            a = 1 + (if b ? c : d);
            "#).unwrap(),
            parse!(parser, r#"
            op a 1 + (
                take ___0 = $;
                goto :___0 b;
                set ___0 d;
                goto :___1 _;
                :___0
                set ___0 c;
                :___1
            );
            "#).unwrap()
        );

    }
}
