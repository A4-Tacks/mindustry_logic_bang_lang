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
    SetVarNoPatternValue(usize, usize),
}

pub type Var = String;
pub type Location = usize;
pub type Float = f64;

pub const COUNTER: &str = "@counter";


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
        Self::ReprVar("0".into())
    }

    /// 编译依赖并返回句柄
    pub fn take(self, meta: &mut CompileMeta) -> Var {
        // 改为使用空字符串代表空返回字符串
        // 如果空的返回字符串被编译将会被编译为tmp_var
        match self {
            Self::Var(var) => {
                if let Some(value) = meta.const_expand_enter(&var) {
                    // 是一个常量
                    let res = if let Value::Var(var) = value {
                        // 只进行单步常量追溯
                        // 因为常量定义时已经完成了原有的多步
                        var
                    } else {
                        value.take(meta)
                    };
                    meta.const_expand_exit();
                    res
                } else {
                    var
                }
            },
            Self::DExp(DExp { mut result, lines }) => {
                if result.is_empty() {
                    result = meta.get_tmp_var(); /* init tmp_var */
                } else if let
                    Some((_, value))
                        = meta.get_const_value(&result) {
                    // 对返回句柄使用常量值的处理
                    if let Some(dexp) = value.as_dexp() {
                        err!(
                            concat!(
                                "尝试在`DExp`的返回句柄处使用值为`DExp`的const, ",
                                "此处仅允许使用`Var`\n",
                                "DExp: {:?}\n",
                                "名称: {:?}",
                            ),
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
            },
            Self::ResultHandle => meta.dexp_handle().clone(),
            Self::ReprVar(var) => var,
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
            Self::Var(ref s) | Self::ReprVar(ref s) => s,
            Self::DExp(DExp { result, .. }) => &result,
            Self::ResultHandle =>
                panic!("未进行AST编译, 而DExp的返回句柄是进行AST编译时已知"),
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
}

/// 进行`词法&语法`分析时所依赖的元数据
#[derive(Debug)]
pub struct Meta {
    tag_number: usize,
    /// 被跳转的label
    defined_labels: Vec<Vec<Var>>,
}
impl Default for Meta {
    fn default() -> Self {
        Self {
            tag_number: 0,
            defined_labels: vec![Vec::new()],
        }
    }
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

    /// 根据一系列构建一系列常量传参
    pub fn build_arg_consts(&self, values: Vec<Value>, mut f: impl FnMut(Const)) {
        for (i, value) in values.into_iter().enumerate() {
            let name = format!("_{}", i);
            f(Const(name, value, Vec::with_capacity(0)))
        }
    }

    /// 构建一个`sets`, 例如`a b c = 1 2 3;`
    /// 如果只有一个值与被赋值则与之前行为一致
    /// 如果值与被赋值数量不匹配则返回错误
    /// 如果值与被赋值数量匹配且大于一对就返回Expand中多个set
    pub fn build_sets(&self, loc: [Location; 2], mut vars: Vec<Value>, mut values: Vec<Value>)
    -> Result<LogicLine, Error> {
        fn build_set(var: Value, value: Value) -> LogicLine {
            LogicLine::Other(vec![
                    "set".into(),
                    var,
                    value,
            ])
        }
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
        Self::NotEqual(val, Value::ReprVar("false".into()))
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
            Self::Equal(a, b)
                | Self::NotEqual(a, b)
                | Self::LessThan(a, b)
                | Self::LessThanEq(a, b)
                | Self::GreaterThan(a, b)
                | Self::StrictEqual(a, b)
                | Self::GreaterThanEq(a, b)
                | Self::StrictNotEqual(a, b)
                => (a, b),
            Self::Always
                | Self::NotAlways
                // 这里使用default生成无副作用的占位值
                => Default::default(),
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
        (a.take(meta), b.take(meta))
    }

    /// 即将编译时调用, 将自身转换为可以正常编译为逻辑的形式
    /// 例如`严格不等`, `永不`等变体是无法直接被编译为逻辑的
    /// 所以需要进行转换
    pub fn do_start_compile_into(self) -> Self {
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
}

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
    pub fn build(self, meta: &mut CompileMeta, mut do_tag: Var) {
        use CmpTree::*;

        // 获取如果在常量展开内则被重命名后的标签
        do_tag = meta.get_in_const_label(do_tag);

        match self {
            Or(a, b) => {
                a.build(meta, do_tag.clone());
                b.build(meta, do_tag);
            },
            And(a, b) => {
                let end = meta.get_tmp_tag();
                a.reverse().build(meta, end.clone());
                b.build(meta, do_tag);
                let tag_id = meta.get_tag(end);
                meta.push(TagLine::TagDown(tag_id));
            },
            Atom(cmp) => {
                // 构建为可以进行接下来的编译的形式
                let cmp = cmp.do_start_compile_into();

                let cmp_str = cmp.cmp_str();
                let (a, b) = cmp.build_value(meta);

                let jump = Jump(
                    meta.get_tag(do_tag).into(),
                    format!("{} {} {}", cmp_str, a, b)
                );
                meta.push(jump.into())
            },
        }
    }
}
impl_enum_froms!(impl From for CmpTree {
    Atom => JumpCmp;
});

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
pub struct Goto(pub Var, pub CmpTree);
impl Compile for Goto {
    fn compile(self, meta: &mut CompileMeta) {
        self.1.build(meta, self.0)
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

/// 在此处计算后方的值, 并将句柄赋给前方值
/// 如果后方不是一个DExp, 而是Var, 那么自然等价于一个常量定义
#[derive(Debug, PartialEq, Clone)]
pub struct Take(pub Var, pub Value);
impl Compile for Take {
    fn compile(self, meta: &mut CompileMeta) {
        Const::new(self.0, self.1.take(meta).into()).compile(meta)
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
                    .map(|val| val.take(meta))
                    .collect();
                meta.push(TagLine::Line(handles.join(" ").into()))
            },
            Self::SetResultHandle(value) => {
                let new_dexp_handle = value.take(meta);
                meta.set_dexp_handle(new_dexp_handle);
            },
            Self::Select(select) => select.compile(meta),
            Self::Expand(expand) => expand.compile(meta),
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
        // 去掉调用次数 (*)
        // 将定义常量映射到常量改为直接把常量对应的值拿过来
        // 防止`const A = 1;const B = A;const A = 2;print B;`输出2
        // 这还需考虑const注册标签
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
        self.tags_map()
            .iter()
            .map(|(tag, id)| format!("{} \t-> {}", tag, id))
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
        let mut tags_map = self.debug_tags_map();
        let mut tag_lines = self.debug_tag_codes();
        line_first_add(&mut tags_map, "\t");
        line_first_add(&mut tag_lines, "\t");
        err!(
            concat!(
                "尝试在`DExp`的外部使用{}\n",
                "tag映射id:\n{}\n",
                "已经生成的代码:\n{}\n",
            ),
            value,
            tags_map.join("\n"),
            tag_lines.join("\n"),
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
        let parser = ExpandParser::new();
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
                LogicLine::Other(vec!["print".into(), r#""hello""#.into()]),
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
        let parser = ExpandParser::new();

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
                            LogicLine::Other(vec!["print".into(), "0".into()]),
                            LogicLine::Other(vec!["print".into(), "end".into()]),
                    ]).into(),
                    Expand(vec![
                            LogicLine::Other(vec!["print".into(), "1".into()]),
                            LogicLine::Other(vec!["print".into(), "end".into()]),
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
                            LogicLine::Other(vec!["print".into(), "end".into()]),
                    ]).into(),
                    Expand(vec![
                            LogicLine::Other(vec!["print".into(), "1".into()]),
                            LogicLine::Other(vec!["print".into(), "end".into()]),
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
        let parser = ExpandParser::new();
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

    #[test]
    fn in_const_label_test() {
        let parser = ExpandParser::new();
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
                        LogicLine::Other(vec!["print".into(), "\"hi\"".into()])
                    ].into()
                ).into(),
                vec!["in_const".into()]
            ).into()
        );
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn const_expand_label_rename_test() {
        let parser = ExpandParser::new();

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
                r#"jump 4 always 0 0"#,
                r#"print "num < 2""#,
                r#"jump 0 always 0 0"#,
                r#"jump 8 lessThan num 2"#,
                r#"print "num >= 2""#,
                r#"jump 9 always 0 0"#,
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
        let parser = ExpandParser::new();

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
        let parser = ExpandParser::new();

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
        let parser = ExpandParser::new();

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
        let parser = ExpandParser::new();

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
        let parser = ExpandParser::new();

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
        let parser = ExpandParser::new();

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
                    Box::new(JumpCmp::NotEqual("a".into(), Value::ReprVar("false".into()).into()).into()),
                    Box::new(JumpCmp::NotEqual("b".into(), Value::ReprVar("false".into()).into()).into()),
                ).into(),
                Box::new(JumpCmp::NotEqual("c".into(), Value::ReprVar("false".into()).into()).into()),
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
                    Box::new(JumpCmp::NotEqual("a".into(), Value::ReprVar("false".into()).into()).into()),
                    Box::new(JumpCmp::NotEqual("b".into(), Value::ReprVar("false".into()).into()).into()),
                ).into(),
                Box::new(JumpCmp::NotEqual("c".into(), Value::ReprVar("false".into()).into()).into()),
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
                    Box::new(JumpCmp::NotEqual("a".into(), Value::ReprVar("false".into()).into()).into()),
                    Box::new(JumpCmp::NotEqual("b".into(), Value::ReprVar("false".into()).into()).into()),
                ).into(),
                CmpTree::And(
                    Box::new(JumpCmp::NotEqual("c".into(), Value::ReprVar("false".into()).into()).into()),
                    Box::new(JumpCmp::NotEqual("d".into(), Value::ReprVar("false".into()).into()).into()),
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
                    Box::new(JumpCmp::NotEqual("a".into(), Value::ReprVar("false".into()).into()).into()),
                    CmpTree::Or(
                        Box::new(JumpCmp::NotEqual("b".into(), Value::ReprVar("false".into()).into()).into()),
                        Box::new(JumpCmp::NotEqual("c".into(), Value::ReprVar("false".into()).into()).into()),
                    ).into(),
                ).into(),
                Box::new(JumpCmp::NotEqual("d".into(), Value::ReprVar("false".into()).into()).into()),
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
        let parser = ExpandParser::new();

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
        let parser = ExpandParser::new();

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
}
