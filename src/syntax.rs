use std::{
    ops::Deref,
    num::ParseIntError,
    collections::HashMap,
    iter::{
        zip,
        repeat_with,
    },
    process::exit,
};

use crate::{
    err,
    tag_code::{
        Jump,
        TagCodes,
        TagLine
    },
};

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
}

pub type Var = String;
pub type Location = usize;
pub type Float = f64;

pub const ARG_RES: [&str; 10] = [
    "_0", "_1", "_2", "_3", "_4",
    "_5", "_6", "_7", "_8", "_9",
];
pub const COUNTER: &str = "@counter";


/// 带有一个未使用的返回句柄信息的Var封装
/// 在宏替换Var为Value时很有用
#[derive(Debug, Clone)]
pub struct VarStruct {
    result: Var,
    value: Var,
}
impl PartialEq for VarStruct {
    fn eq(&self, other: &Self) -> bool {
        // 没必要进行result的比较,
        // 因为在Var状态下, result的值是未使用的, 是无意义的
        self.value == other.value
    }
}
impl Deref for VarStruct {
    type Target = Var;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}
impl VarStruct {
    pub fn default_result(&mut self, str: impl FnOnce() -> Var) {
        if self.result.is_empty() {
            self.result = str()
        }
    }
}
impl From<VarStruct> for Var {
    fn from(value: VarStruct) -> Self {
        value.value
    }
}
impl From<&str> for VarStruct {
    fn from(value: &str) -> Self {
        Var::from(value).into()
    }
}
impl From<String> for VarStruct {
    fn from(value: String) -> Self {
        Self {
            result: "".into(),
            value
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum Value {
    Var(VarStruct),
    DExp(DExp),
    /// 编译时被替换为当前DExp返回句柄
    ResultHandle,
}
impl Default for Value {
    fn default() -> Self {
        Self::Var("0".into())
    }
}
impl Value {
    /// 如果是一个[`DExp`]且未指定返回句柄则负责将其设为传入默认值
    pub fn default_result(mut self, str: impl FnOnce() -> Var) -> Self {
        match &mut self {
            Self::Var(ref mut var) => var.default_result(str),
            Self::DExp(ref mut dexp) => dexp.default_result(str),
            Self::ResultHandle => (), // ignore
        }
        self
    }

    /// 编译依赖并返回句柄
    pub fn take(self, meta: &mut CompileMeta) -> Var {
        // TODO
        // 先检查元数据中是否应该将self替换, 例如在编译宏时
        match self {
            Self::Var(var) => {
                if let Some(value) = meta.get_const_value(&var) {
                    value.clone()
                        .default_result(|| var.result)
                        .take(meta)
                } else {
                    var.into()
                }
            },
            Self::DExp(DExp { result, lines }) => {
                #[cfg(debug_assertions)]
                let old_handle = result.clone();
                meta.push_dexp_handle(result);
                lines.compile(meta);
                let result = meta.pop_dexp_handle();
                #[cfg(debug_assertions)]
                assert_eq!(result, old_handle);
                result
            },
            Self::ResultHandle => meta.dexp_handle().clone(),
        }
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
}
impl Deref for Value {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Var(ref s) => &s,
            Self::DExp(DExp { result, .. }) => &result,
            Self::ResultHandle => panic!("未进行AST编译, 而DExp的返回句柄是进行AST编译时已知"),
        }
    }
}
impl_enum_froms!(impl From for Value {
    Var => Var;
    Var => &str;
    DExp => DExp;
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
    pub fn default_result(&mut self, str: impl FnOnce() -> Var) {
        if self.result.is_empty() {
            self.result = str()
        }
    }
}

/// 进行`词法&语法`分析时所依赖的元数据
pub struct Meta {
    tag_number: usize,
    id: usize,
    tmp_var: usize,
}
impl Meta {
    /// use [`Self::default()`]
    pub fn new() -> Self {
        Self::default()
    }

    /// 获取一个标签, 并且进行内部自增以保证不会获取到获取过的
    pub fn get_tag(&mut self) -> String {
        let tag = self.tag_number;
        self.tag_number += 1;
        format!("__{}", tag)
    }

    /// 获取一个临时变量, 不会重复获取
    pub fn get_tmp_var(&mut self) -> Var {
        let id = self.tmp_var;
        self.tmp_var += 1;
        format!("__{}", id)
    }

    /// 获取一个始终不会重复获取的id
    pub fn get_id(&mut self) -> usize {
        let id = self.id;
        self.id += 1;
        id
    }
}
impl Default for Meta {
    fn default() -> Self {
        Self {
            tag_number: 0,
            id: 0,
            tmp_var: 0,
        }
    }
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
    StrictEqual(Value, Value),
    Always,
}
impl JumpCmp {
    /// 创建一个永远为假的变体
    pub fn false_val() -> Self {
        Self::NotEqual("0".into(), "0".into())
    }

    /// 将值转为`bool`来对待
    pub fn bool(val: Value) -> Self {
        Self::NotEqual(val, "false".into())
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
            StrictEqual(a, b) => {
                // 其中一参数转换为`DExp`计算严格相等, 然后取反
                const RES: &str = ARG_RES[0]; // `DExp`返回的目标
                let val = DExp::new(
                    RES.into(),
                    vec![Op::StrictEqual(RES.into(), a, b).into()].into()
                );
                Self::bool(val.into()).reverse()
            },
            Always => Self::false_val(),
        }
    }

    /// 获取两个运算成员, 如果是[`Always`]则返回[`Default`]
    pub fn get_values(self) -> (Value, Value) {
        match self {
            Self::Equal(a, b)
                | Self::NotEqual(a, b)
                | Self::LessThan(a, b)
                | Self::LessThanEq(a, b)
                | Self::GreaterThan(a, b)
                | Self::StrictEqual(a, b)
                | Self::GreaterThanEq(a, b)
                => (a, b),
            Self::Always => Default::default(),
        }
    }

    pub fn cmp_str(&self) -> &'static str {
        macro_rules! build_match {
            ( $( $name:ident $str:literal ),* $(,)? ) => {
                match self {
                    $(
                        Self::$name(..) => $str,
                    )*
                    Self::Always => "always",
                }
            };
        }

        build_match! {
            Equal "equal",
            NotEqual "notEqual",
            LessThan "lessThan",
            LessThanEq "lessThanEq",
            GreaterThan "greaterThan",
            GreaterThanEq "greaterThanEq",
            StrictEqual "strictEqual",
        }
    }

    /// 构建两个值后将句柄送出
    pub fn build_value(self, meta: &mut CompileMeta) -> (Var, Var) {
        let (a, b) = self.get_values();
        (a.take(meta), b.take(meta))
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
    pub fn oper_str(&self) -> &'static str {
        macro_rules! build_match {
            {
                $(
                    $variant:ident $str:literal
                ),* $(,)?
            } => {
                match self {
                    $( Self::$variant(..) => $str ),*
                }
            };
        }
        build_match! {
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
            Not "not",
            Max "max",
            Min "min",
            Angle "angle",
            Len "len",
            Noise "noise",
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
        }
    }

    pub fn generate_args(self, meta: &mut CompileMeta) -> Vec<String> {
        let mut args: Vec<Var> = Vec::with_capacity(5);
        args.push("op".into());
        args.push(self.oper_str().into());
        macro_rules! build_match {
            {
                op1: [ $( $oper1:ident ),* $(,)?  ]
                op2: [ $( $oper2:ident ),* $(,)?  ]
            } => {
                match self {
                    $(
                        Self::$oper1(res, a) => {
                            args.push(res.take(meta).into());
                            args.push(a.take(meta).into());
                            args.push("0".into());
                        },
                    )*
                    $(
                        Self::$oper2(res, a, b) => {
                            args.push(res.take(meta).into());
                            args.push(a.take(meta).into());
                            args.push(b.take(meta).into());
                        },
                    )*
                }
            };
        }
        build_match!(
            op1: [
                Not, Abs, Log, Log10, Floor, Ceil, Sqrt,
                Rand, Sin, Cos, Tan, Asin, Acos, Atan,
            ]
            op2: [
                Add, Sub, Mul, Div, Idiv,
                Mod, Pow, Equal, NotEqual, Land,
                LessThan, LessThanEq, GreaterThan, GreaterThanEq, StrictEqual,
                Shl, Shr, Or, And, Xor,
                Max, Min, Angle, Len, Noise,
            ]
        );
        debug_assert!(args.len() == 5);
        args
    }
}
impl Compile for Op {
    fn compile(self, meta: &mut CompileMeta) {
        let args = self.generate_args(meta);
        meta.tag_codes.push(args.join(" ").into())
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Goto(pub Var, pub JumpCmp);
impl Compile for Goto {
    fn compile(self, meta: &mut CompileMeta) {
        let cmp_str = self.1.cmp_str();
        let (a, b) = self.1.build_value(meta);
        let jump = Jump(
            meta.get_tag(self.0).into(),
            format!("{} {} {}", cmp_str, a, b)
        );
        meta.push(TagLine::Jump(jump.into()))
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
impl_derefs!(impl for Expand => (self: self.0): Vec<LogicLine>);

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
        let max_len = lens.iter().copied().max().unwrap();

        // build head
        let tmp_var = meta.get_tmp_var();
        let mut head = Op::Mul(
            tmp_var.clone().into(),
            self.0,
            max_len.to_string().into()
        ).compile_take(meta);
        let head_1 = Op::Add(
            COUNTER.into(),
            COUNTER.into(),
            tmp_var.into()
        ).compile_take(meta);
        head.extend(head_1);

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

/// 在块作用域将Var常量为后方值, 之后使用Var时都会被替换为后方值
#[derive(Debug, PartialEq, Clone)]
pub struct Const(pub Var, pub Value);
impl Compile for Const {
    fn compile(self, meta: &mut CompileMeta) {
        // 对同作用域定义过的常量形成覆盖
        // 如果要进行警告或者将信息传出则在此处理
        meta.add_const_value(self.0, self.1);
    }
}

/// 在此处计算后方的值, 并将句柄赋给前方值
/// 如果后方不是一个DExp, 而是Var, 那么自然等价于一个常量定义
#[derive(Debug, PartialEq, Clone)]
pub struct Take(pub Var, pub Value);
impl Compile for Take {
    fn compile(self, meta: &mut CompileMeta) {
        Const(self.0, self.1.take(meta).into()).compile(meta)
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum LogicLine {
    Op(Op),
    Label(Var),
    Goto(Goto),
    Other(Vec<Value>),
    Expand(Expand),
    Select(Select),
    End,
    NoOp,
    /// 空语句, 什么也不生成
    Ignore,
    Const(Const),
    Take(Take),
}
impl Compile for LogicLine {
    fn compile(self, meta: &mut CompileMeta) {
        match self {
            Self::End => meta.push("end".into()),
            Self::NoOp => meta.push("noop".into()),
            Self::Label(lab) => {
                let data = TagLine::TagDown(meta.get_tag(lab));
                meta.push(data)
            },
            Self::Other(args) => {
                let handles: Vec<String> = args
                    .into_iter()
                    .map(|val| val.take(meta))
                    .collect();
                meta.push(TagLine::Line(handles.join(" ").into()))
            },
            Self::Select(select) => select.compile(meta),
            Self::Expand(expand) => expand.compile(meta),
            Self::Goto(goto) => goto.compile(meta),
            Self::Op(op) => op.compile(meta),
            Self::Ignore => (),
            Self::Const(r#const) => r#const.compile(meta),
            Self::Take(take) => take.compile(meta),
        }
    }
}
impl Default for LogicLine {
    fn default() -> Self {
        Self::NoOp
    }
}
impl LogicLine {
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
    Select => Select;
    Const => Const;
    Take => Take;
});

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
    const_var_namespace: Vec<HashMap<Var, Value>>,
    /// 每层DExp所使用的句柄, 末尾为当前层
    dexp_result_handles: Vec<Var>,
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
        self.const_var_namespace.push(HashMap::new())
    }

    /// 退出一个子块, 弹出最顶层命名空间
    /// 如果无物可弹说明逻辑出现了问题, 所以内部处理为unwrap
    /// 一个enter对应一个exit
    pub fn block_exit(&mut self) -> HashMap<Var, Value> {
        self.const_var_namespace.pop().unwrap()
    }

    /// 获取一个常量到值的映射, 从当前作用域往顶层作用域一层层找, 都没找到就返回空
    pub fn get_const_value(&self, name: &Var) -> Option<&Value> {
        for namespace in self.const_var_namespace.iter().rev() {
            if let Some(value) = namespace.get(name) {
                return value.into();
            }
        }
        None
    }

    /// 新增一个常量到值的映射, 如果当前作用域已有此映射则返回旧的值并插入新值
    pub fn add_const_value(&mut self, var: Var, value: Value) -> Option<Value> {
        self.const_var_namespace
            .last_mut()
            .unwrap()
            .insert(var, value)
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
        self.tags_map()
            .iter()
            .map(|(tag, id)| format!("{} \t-> {}", tag, id))
            .collect()
    }

    pub fn dexp_handle(&self) -> &Var {
        self.dexp_result_handles.last().unwrap_or_else(|| {
            let mut tags_map = self.debug_tags_map();
            let mut tag_lines = self.debug_tag_codes();
            line_first_add(&mut tags_map, "\t");
            line_first_add(&mut tag_lines, "\t");
            err!(
                concat!(
                    "尝试在`DExp`的外部使用`DExpHandle` (`$`)\n",
                    "tag映射id:\n{}\n",
                    "已经生成的代码:\n{}\n",
                ),
                tags_map.join("\n"),
                tag_lines.join("\n"),
            );
            exit(6)
        })
    }
}

pub fn line_first_add(lines: &mut Vec<String>, insert: &str) {
    for line in lines {
        let s = format!("{}{}", insert, line);
        *line = s;
    }
}

#[cfg(test)]
mod tests {
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
        let parser = ExpandParser::new();
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
                "op res (op _0 1 + 2; op _0 _0 * 2;) / (x: op x 2 * 3;);"
            ).unwrap()[0],
            Op::Div(
                "res".into(),
                DExp::new(
                    "_0".into(),
                    vec![
                        Op::Add(
                            "_0".into(),
                            "1".into(),
                            "2".into()
                        ).into(),
                        Op::Mul(
                            "_0".into(),
                            "_0".into(),
                            "2".into()
                        ).into()
                    ].into()).into(),
                DExp::new(
                    "x".into(),
                    vec![
                        Op::Mul("x".into(), "2".into(), "3".into()).into()
                    ].into(),
                ).into()
            ).into()
        );
        assert_eq!(
            parse!(
                parser,
                "op res (op _0 1 + 2; op _0 _0 * 2;) / (op _1 2 * 3;);"
            ).unwrap()[0],
            Op::Div(
                "res".into(),
                DExp::new(
                    "_0".into(),
                    vec![
                        Op::Add(
                            "_0".into(),
                            "1".into(),
                            "2".into()
                        ).into(),
                        Op::Mul(
                            "_0".into(),
                            "_0".into(),
                            "2".into()
                        ).into()
                    ].into()).into(),
                DExp::new(
                    "_1".into(),
                    vec![
                        Op::Mul("_1".into(), "2".into(), "3".into()).into()
                    ].into(),
                ).into()
            ).into()
        );
    }

    #[test]
    fn goto_test() {
        let parser = ExpandParser::new();
        assert_eq!(parse!(parser, "goto :a 1 <= 2; :a").unwrap(), vec![
            Goto("a".into(), JumpCmp::LessThanEq("1".into(), "2".into())).into(),
            LogicLine::Label("a".into()),
        ].into());
    }

    #[test]
    fn control_test() {
        let parser = LogicLineParser::new();
        assert_eq!(
            parse!(parser, r#"skip 1 < 2 print "hello";"#).unwrap(),
            Expand(vec![
                Goto("__0".into(), JumpCmp::LessThan("1".into(), "2".into())).into(),
                LogicLine::Other(vec!["print".into(), r#""hello""#.into()]),
                LogicLine::Label("__0".into()),
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
                goto :__1 2 < 3;
                goto :__2 3 < 4;
                goto :__3 4 < 5;
                print 4;
                goto :__0 _;
                :__2 {
                    print 2;
                }
                goto :__0 _;
                :__3 {
                    print 3;
                }
                goto :__0 _;
                :__1 {
                    print 1;
                }
                :__0
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
                goto :__0 a >= b;
                :__1
                print 3;
                goto :__1 a < b;
                :__0
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
                :__0 {
                    print 1;
                }
                goto :__0 a < b;
            }
            "#).unwrap(),
        );
    }

    #[test]
    fn reverse_test() {
        let parser = LogicLineParser::new();
        let datas = vec![
            [r#"goto :a x === y;"#, r#"goto :a (op _0 x === y;) == false;"#],
            [r#"goto :a x == y;"#, r#"goto :a x != y;"#],
            [r#"goto :a x != y;"#, r#"goto :a x == y;"#],
            [r#"goto :a x < y;"#, r#"goto :a x >= y;"#],
            [r#"goto :a x > y;"#, r#"goto :a x <= y;"#],
            [r#"goto :a x <= y;"#, r#"goto :a x > y;"#],
            [r#"goto :a x >= y;"#, r#"goto :a x < y;"#],
            [r#"goto :a x;"#, r#"goto :a x == false;"#],
            [r#"goto :a _;"#, r#"goto :a 0 != 0;"#],
        ];
        for [src, dst] in datas {
            assert_eq!(
                parse!(parser, src).unwrap().as_goto().unwrap().1.clone().reverse(),
                parse!(parser, dst).unwrap().as_goto().unwrap().1,
            );
        }
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
                    LogicLine::NoOp,
                    Expand(vec![LogicLine::Other(vec!["print".into(), "1".into()])]).into(),
                    Expand(vec![
                        LogicLine::Other(vec!["print".into(), "2".into()]),
                        LogicLine::Other(vec!["print".into(), "4".into()]),
                    ]).into(),
                    LogicLine::NoOp,
                    Expand(vec![
                        LogicLine::Other(vec!["print".into(), "2".into()]),
                        LogicLine::Other(vec!["print".into(), "4".into()]),
                    ]).into(),
                    Expand(vec![
                        LogicLine::Label("a".into()),
                        LogicLine::Label("b".into()),
                        LogicLine::Other(vec!["print".into(), "5".into()]),
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
        //println!("{}", lines.join("\n"));
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
        let parser = ExpandParser::new();
        let src = r#"
        op x 1 + 2;
        op y (op _0 x + 3;) * (op _1 x * 2;);
        if (op tmp y & 1; op _0 tmp + 1;) == 1 {
            print "a ";
        } else {
            print "b ";
        }
        print (op _0 y + 3;);
        "#;
        //dbg!(&src);
        let ast = parse!(parser, src).unwrap();
        //dbg!(&ast);
        let meta = CompileMeta::new();
        //dbg!(&meta);
        let mut tag_codes = meta.compile(ast);
        //dbg!(&tag_codes);
        let logic_lines = tag_codes.compile().unwrap();
        //dbg!(&logic_lines);
        //println!("{}", logic_lines.join("\n"));
        assert_eq!(logic_lines, [
            r#"op add x 1 2"#,
            r#"op add _0 x 3"#,
            r#"op mul _1 x 2"#,
            r#"op mul y _0 _1"#,
            r#"op and tmp y 1"#,
            r#"op add _0 tmp 1"#,
            r#"jump 9 equal _0 1"#,
            r#"print "b ""#,
            r#"jump 10 always 0 0"#,
            r#"print "a ""#,
            r#"op add _0 y 3"#,
            r#"print _0"#,
        ])
    }

    #[test]
    fn compile_take_test() {
        let parser = LogicLineParser::new();
        let ast = parse!(parser, "op x (op _0 1 + 2;) + 3;").unwrap();
        let mut meta = CompileMeta::new();
        meta.push(TagLine::Line("noop".to_string().into()));
        assert_eq!(
            ast.compile_take(&mut meta),
            vec![
                TagLine::Line("op add _0 1 2".to_string().into()),
                TagLine::Line("op add x _0 3".to_string().into()),
            ]
        );
        assert_eq!(meta.tag_codes.len(), 1);
        assert_eq!(meta.tag_codes.lines(), &vec![TagLine::Line("noop".to_string().into())]);
    }

    #[test]
    fn const_value_test() {
        let parser = ExpandParser::new();

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
                   "read _0 cell1 0",
                   "set y _0",
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
                   "read _2 cell1 0",
                   "read _4 cell1 0",
                   "foo a b _2 d _4",
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
                   "read _0 cell1 i",
                   "print _0",
        ]);
    }

    #[test]
    fn const_value_block_range_test() {
        let parser = ExpandParser::new();

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
                   "read _0 cell3 0",
                   "set m _0",
                   "read _0 cell2 0",
                   "set y _0",
                   "read _0 cell2 0",
                   "read _1 cell2 0",
                   "foo _0 _1",
                   "set z C",
        ]);
    }

    #[test]
    fn take_test() {
        let parser = ExpandParser::new();

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
        let parser = ExpandParser::new();

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
}
