mod builtins;
#[cfg(test)]
mod tests;

use std::{
    borrow::{Borrow, Cow},
    collections::{HashMap, HashSet},
    convert::identity,
    fmt::{self, Debug, Display},
    hash::Hash,
    iter::{repeat_with, zip, once},
    mem::{self, replace},
    num::ParseIntError,
    ops::{self, Deref},
    process::exit,
    rc::Rc,
    cell::Cell,
};

use builtins::{build_builtins, BuiltinFunc};
use either::Either;
use tag_code::{Jump, TagCodes, TagLine, mdt_logic_split};
use utils::counter::Counter;
use var_utils::{string_unescape, AsVarType};

pub use either;

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
macro_rules! impl_enum_try_into {
    (impl TryInto for $ty:ty {$(
        $variant:ident => $target:ty ;
    )*}) => {
        $(
            impl TryFrom<$ty> for $target {
                type Error = $ty;

                fn try_from(value: $ty) -> Result<Self, Self::Error> {
                    use $ty::*;
                    match value {
                        $variant(v) => Ok(v),
                        this => Err(this),
                    }
                }
            }
        )*
    };
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
    (let $pat:pat = $expr:expr $(=> $v:expr)?) => {
        let $pat = $expr else { return $($v)? };
    };
    ($e:expr $(=> $v:expr)?) => {
        if $e {
            return $($v)?
        }
    };
}
macro_rules! csi {
    (@ignore($($i:tt)*) $($t:tt)*) => ($($t)*);
    ($fcode:expr $(, $code:expr)*; $($arg:tt)*) => {
        format_args!(
            concat!("\x1b[{}", $(csi!(@ignore($code) ";{}"), )* "m{}\x1b[0m"),
            $fcode,
            $($code,)*
            format_args!($($arg)*),
        )
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
    NotALiteralUInteger(Var, ParseIntError),
    SetVarNoPatternValue(usize, usize),
    ArgsRepeatChunkByZero,
}

/// 带有错误前缀, 并且文本为红色的eprintln
macro_rules! err {
    ( $fmtter:expr $(, $args:expr)* $(,)? ) => {
        eprintln!(concat!("\x1b[1;31m", "CompileError:\n", $fmtter, "\x1b[0m"), $($args),*);
    };
}

#[derive(Debug, Default, Clone)]
pub struct Var {
    value: Rc<String>,
}
impl Var {
    pub fn new() -> Self {
        Self::default()
    }
}
impl FromIterator<char> for Var {
    fn from_iter<T: IntoIterator<Item = char>>(iter: T) -> Self {
        String::from_iter(iter).into()
    }
}
impl From<&'_ Var> for Var {
    fn from(value: &'_ Var) -> Self {
        value.clone()
    }
}
impl From<Var> for String {
    fn from(mut value: Var) -> Self {
        Rc::make_mut(&mut value.value);
        Rc::try_unwrap(value.value).unwrap()
    }
}
impl From<Var> for Rc<String> {
    fn from(value: Var) -> Self {
        value.value
    }
}
impl Display for Var {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.value, f)
    }
}
impl Hash for Var {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.value.hash(state)
    }
}
impl Borrow<String> for Var {
    fn borrow(&self) -> &String {
        &self.value
    }
}
impl Borrow<str> for Var {
    fn borrow(&self) -> &str {
        &***self
    }
}
impl AsRef<String> for Var {
    fn as_ref(&self) -> &String {
        self.borrow()
    }
}
impl AsRef<str> for Var {
    fn as_ref(&self) -> &str {
        self.borrow()
    }
}
impl From<String> for Var {
    fn from(value: String) -> Self {
        Self { value: value.into() }
    }
}
impl From<&'_ str> for Var {
    fn from(value: &'_ str) -> Self {
        Self { value: Rc::new(value.into()) }
    }
}
impl Deref for Var {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &*self.value
    }
}
impl Eq for Var { }
impl PartialEq for Var {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}
impl PartialEq<String> for Var {
    fn eq(&self, other: &String) -> bool {
        &**self == other
    }
}
impl PartialEq<str> for Var {
    fn eq(&self, other: &str) -> bool {
        **self == other
    }
}
impl PartialEq<&str> for Var {
    fn eq(&self, other: &&str) -> bool {
        **self == *other
    }
}

pub type Location = usize;
pub type Float = f64;

fn default<T: Default>() -> T {
    T::default()
}

pub const COUNTER: &str = "@counter";
pub const FALSE_VAR: &str = "false";
pub const ZERO_VAR: &str = "0";
pub const UNUSED_VAR: &str = "0";

pub trait TakeHandle: Sized {
    /// 编译依赖并返回句柄
    fn take_handle(self, meta: &mut CompileMeta) -> Var;
    /// 编译并拿取句柄, 但是是在被const的值的情况下
    fn take_handle_with_consted(self, meta: &mut CompileMeta) -> Var {
        self.take_handle(meta)
    }
}

impl TakeHandle for Var {
    fn take_handle(self, meta: &mut CompileMeta) -> Var {
        if let Some(value) = meta.const_expand_enter(&self) {
            // 是一个常量
            let res = value.take_handle_with_consted(meta);
            meta.const_expand_exit();
            res
        } else {
            self
        }
    }
    fn take_handle_with_consted(self, _meta: &mut CompileMeta) -> Var {
        self
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
    ValueBindRef(ValueBindRef),
    /// 一个跳转条件, 未决目标的跳转, 它会被内联
    /// 如果它被take, 那么它将引起报错
    Cmper(Box<CmpTree>),
    /// 本层应该指向的绑定者, 也就是ValueBind的被绑定的值
    Binder,
    ClosuredValue(ClosuredValue),
    BuiltinFunc(BuiltinFunc),
}
impl Value {
    pub fn fmt_literal_num(num: f64) -> Var {
        use std::num::FpCategory as FpC;

        let n = num.round() - num;
        if let FpC::Zero | FpC::Subnormal = n.classify() {
            let rng = i64::MIN as f64..=i64::MAX as f64;
            let v = 999999;
            if rng.contains(&num)
                && !(-v..=v).contains(&(num as i64))
            {
                let num = num.round() as i64;
                return if !num.is_negative() {
                    format!("0x{num:X}").into()
                } else {
                    let num = -num;
                    format!("0x-{num:X}").into()
                }
            }
        }
        return num.to_string().into()
    }

    pub fn try_eval_const_num_to_var(&self, meta: &CompileMeta) -> Option<Var> {
        if let Some((num, true)) = self.try_eval_const_num(meta) {
            use std::num::FpCategory as FpC;
            // 仅对复杂数据也就是有效运算后的数据
            return match num.classify() {
                FpC::Nan
                    => "null".into(),
                FpC::Infinite
                if num.is_sign_negative()
                    => Self::fmt_literal_num(-i64::MAX as f64),
                FpC::Infinite
                    => Self::fmt_literal_num(i64::MAX as f64),
                _ => Self::fmt_literal_num(num),
            }.into()
        } else {
            None
        }
    }
}
impl TakeHandle for Value {
    fn take_handle(self, meta: &mut CompileMeta) -> Var {
        if let Some(var) = self.try_eval_const_num_to_var(meta) {
            return var;
        }
        // 改为使用空字符串代表空返回字符串
        // 如果空的返回字符串被编译将会被编译为tmp_var
        match self {
            Self::Var(var) => var.take_handle(meta),
            Self::DExp(dexp) => dexp.take_handle(meta),
            Self::ResultHandle => meta.dexp_handle().clone(),
            Self::ReprVar(var) => var,
            Self::ValueBind(val_bind) => val_bind.take_handle(meta),
            Self::ValueBindRef(bindref) => bindref.take_handle(meta),
            Self::Binder => {
                meta.get_dexp_expand_binder().cloned()
                    .unwrap_or_else(|| "__".into())
            },
            Self::Cmper(cmp) => {
                err!(
                    "{}\n最终未被展开的cmper, {:#?}",
                    meta.err_info().join("\n"),
                    cmp,
                );
                exit(6);
            },
            Self::ClosuredValue(closurev) => closurev.take_handle(meta),
            Self::BuiltinFunc(func) => func.call(meta),
        }
    }
    fn take_handle_with_consted(self, meta: &mut CompileMeta) -> Var {
        if let Some(var) = self.try_eval_const_num_to_var(meta) {
            return var;
        }
        match self {
            Self::Var(var) => var,
            Self::ReprVar(var) => {
                panic!("Fail const reprvar {}, meta: {:#?}", var, meta);
            },
            other => other.take_handle(meta),
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

    pub fn is_ident(s: &str) -> bool {
        use var_utils::is_ident;
        is_ident(s)
    }

    /// 判断是否是一个标识符(包括数字)关键字
    pub fn is_ident_keyword(s: &str) -> bool {
        var_utils::is_ident_keyword(s)
    }

    /// 判断是否不应该由原始标识符包裹
    /// 注意是原始标识符(原始字面量), 不要与原始值混淆
    pub fn no_use_repr_var(s: &str) -> bool {
        Self::is_string(s)
            || (
                Self::is_ident(s)
                    && ! Self::is_ident_keyword(s)
            )
    }

    /// 返回被规范化的标识符
    pub fn replace_ident(s: &str) -> String {
        if Self::no_use_repr_var(s) {
            if Self::is_string(s) {
                string_unescape(s)
            } else {
                s.into()
            }
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

    pub fn as_result_handle(&self) -> Option<()> {
        if let Self::ResultHandle = self {
            ().into()
        } else {
            None
        }
    }

    /// 尝试解析为一个常量数字
    ///
    /// 如果直接得到结果例如直接编写一个数字, 那么复杂标志为假
    pub fn try_eval_const_num(&self, meta: &CompileMeta) -> Option<(f64, bool)> {
        fn num(s: &str, complex: bool) -> Option<(f64, bool)> {
            s.as_var_type().as_number().map(|&x| (x, complex))
        }
        match self {
            Self::ReprVar(var) => num(var, false),
            Self::Var(name) => {
                match meta.get_const_value(name) {
                    Some(ConstData { value: Self::Var(var), .. }) => {
                        num(var, false)
                    },
                    Some(ConstData { value: Self::ReprVar(repr_var), .. }) => {
                        unreachable!("被const的reprvar {:?}", repr_var)
                    },
                    Some(ConstData { value: x @ Self::DExp(_), .. }) => {
                        x.try_eval_const_num(meta)
                    },
                    Some(_) => None?,
                    None => num(name, false),
                }
            },
            Self::DExp(dexp) if dexp.len() == 1 && dexp.result.is_empty() => {
                let logic_line = &dexp.first().unwrap();
                match logic_line {
                    LogicLine::Op(op) => {
                        op.try_eval_const_num(meta)
                            .map(|x| (x, true))
                    },
                    LogicLine::Other(args) => {
                        let args = args.as_normal()?;
                        let Value::ReprVar(cmd) = &args[0] else {
                            return None;
                        };
                        match &***cmd {
                            "set"
                            if args.len() == 3
                            && args[1].is_result_handle()
                            => (args[2].try_eval_const_num(meta)?.0, true).into(),
                            _ => None,
                        }
                    },
                    _ => None,
                }
            },
            Value::Binder => num(meta.get_dexp_expand_binder()?, true),
            // NOTE: 故意的不实现, 常量求值应该'简单'
            Value::ValueBind(..) => None,
            Value::ValueBindRef(..) => None,
            // NOTE: 这不能实现, 否则可能牵扯一些不希望的作用域问题
            Value::ResultHandle => None,
            Value::BuiltinFunc(_) | Value::DExp(_)
            | Value::Cmper(_) | Value::ClosuredValue(_) => None,
        }
    }
}
impl_enum_froms!(impl From for Value {
    Var => Var;
    Var => &Var;
    Var => String;
    Var => &str;
    DExp => DExp;
    ValueBind => ValueBind;
    ClosuredValue => ClosuredValue;
    BuiltinFunc => BuiltinFunc;
    ValueBindRef => ValueBindRef;
});
impl_enum_try_into!(impl TryInto for Value {
    Var => Var;
    DExp => DExp;
    ValueBind => ValueBind;
    ValueBindRef => ValueBindRef;
});

/// 一次性的迭代器格式化包装器
struct IterFmtter<I> {
    iter: Cell<Option<I>>,
}
impl<I> From<I> for IterFmtter<I> {
    fn from(value: I) -> Self {
        Self::new(value)
    }
}
impl<I> IterFmtter<I> {
    fn new(iter: I) -> Self {
        Self {
            iter: Some(iter).into(),
        }
    }
}
trait IntoIterFmtter: Sized {
    fn into_iter_fmtter(self) -> IterFmtter<Self>;
}
impl<I: IntoIterator> IntoIterFmtter for I {
    fn into_iter_fmtter(self) -> IterFmtter<Self> {
        self.into()
    }
}
impl<I> Debug for IterFmtter<I>
where I: IntoIterator,
      I::Item: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.iter.take()
            .ok_or(fmt::Error)?
            .into_iter()
            .try_for_each(|s| {
                s.fmt(f)
            })
    }
}
impl<I> Display for IterFmtter<I>
where I: IntoIterator,
      I::Item: Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.iter.take()
            .ok_or(fmt::Error)?
            .into_iter()
            .try_for_each(|s| {
                s.fmt(f)
            })
    }
}

/// 在常量追溯时就发生绑定追溯, 而不是take时
/// 不会take追溯到的值
///
/// 当未经过常量追溯, 那么它将采用ValueBind的行为
#[derive(Debug, PartialEq, Clone)]
pub struct ValueBindRef {
    value: Box<Value>,
    bind_target: ValueBindRefTarget,
}
impl ValueBindRef {
    pub fn new(value: Box<Value>, bind_target: ValueBindRefTarget) -> Self {
        Self { value, bind_target }
    }

    fn get_binder(value: Value, meta: &mut CompileMeta) -> Var {
        Const(ConstKey::Var(Var::new()), value, vec![])
            .run_value(meta)
            .unwrap_or_else(|| "__".into())
    }

    pub fn do_follow(
        self,
        meta: &mut CompileMeta,
    ) -> Either<Var, &ConstData> {
        let Self { value, bind_target } = self;
        match bind_target {
            ValueBindRefTarget::NameBind(name) => {
                let handle = ValueBind(value, name).take_unfollow_handle(meta);
                meta.get_const_value(&handle)
                    .map(Either::Right)
                    .unwrap_or_else(|| Either::Left(handle))
            },
            ValueBindRefTarget::Binder => {
                Either::Left(Self::get_binder(*value, meta))
            },
            ValueBindRefTarget::ResultHandle => {
                Either::Left(value.take_handle(meta))
            },
        }
    }

    pub fn value(&self) -> &Value {
        &self.value
    }

    pub fn bind_target(&self) -> &ValueBindRefTarget {
        &self.bind_target
    }
}
impl TakeHandle for ValueBindRef {
    fn take_handle(self, meta: &mut CompileMeta) -> Var {
        let Self { value, bind_target } = self;
        match bind_target {
            ValueBindRefTarget::NameBind(name) => {
                ValueBind(value, name).take_handle(meta)
            },
            ValueBindRefTarget::Binder => {
                Self::get_binder(*value, meta)
            },
            ValueBindRefTarget::ResultHandle => {
                value.take_handle(meta)
            },
        }
    }
    fn take_handle_with_consted(self, _meta: &mut CompileMeta) -> Var {
        panic!("它不应以常量追溯目标被take, \
                它应是直接take或者追溯时被替换成了其它值.\n\
                {:?}", self)
    }
}
#[derive(Debug, PartialEq, Clone)]
pub enum ValueBindRefTarget {
    NameBind(Var),
    Binder,
    ResultHandle,
}
impl_enum_froms!(impl From for ValueBindRefTarget {
    NameBind => Var;
    NameBind => &str;
});

#[derive(Debug, PartialEq, Clone)]
pub enum ClosuredValueMethod {
    Take(Take),
    Const(Const),
}
impl ClosuredValueMethod {
    pub fn do_catch(self, meta: &mut CompileMeta, bind: impl Into<Var>) {
        let make_key = |k: &mut ConstKey| {
            k.name_to_bind(Value::ReprVar(bind.into())).unwrap()
        };
        match self {
            Self::Take(mut take) => {
                make_key(&mut take.0);
                take.compile(meta);
            },
            Self::Const(mut r#const) => {
                make_key(&mut r#const.0);
                r#const.compile(meta);
            },
        }
    }

    pub fn get_key(&self) -> &ConstKey {
        match self {
            | Self::Take(Take(key, _))
            | Self::Const(Const(key, _, _))
            => key,
        }
    }
}
impl_enum_froms!(impl From for ClosuredValueMethod {
    Take => Take;
    Const => Const;
});

/// 闭包值, 将在常量追溯或take时设置闭包环境,
/// 然后在take时将环境加载
#[derive(Debug, PartialEq, Clone)]
pub enum ClosuredValue {
    Uninit {
        catch_values: Vec<ClosuredValueMethod>,
        value: Box<Value>,
        labels: Vec<Var>,
    },
    Inited {
        bind_handle: Var,
        vars: Vec<Var>,
    },
    /// 操作过程中使用, 过程外不能使用
    Empty,
}
impl ClosuredValue {
    const BINDED_VALUE_NAME: &'static str = "__Value";

    pub fn make_valkey(bindh: Var) -> ConstKey {
        let mut key = ConstKey::Var(Self::BINDED_VALUE_NAME.into());
        key.name_to_bind(Value::ReprVar(bindh)).unwrap();
        key
    }

    pub fn catch_vars(&mut self, meta: &mut CompileMeta) {
        let this = match self {
            Self::Uninit { .. } => mem::replace(self, Self::Empty),
            Self::Inited { .. } => return,
            Self::Empty => panic!(),
        };
        let Self::Uninit {
            catch_values,
            value,
            labels,
        } = this else { panic!() };

        let vars = catch_values.iter()
            .map(ClosuredValueMethod::get_key)
            .map(ConstKey::as_var)
            .map(Option::unwrap)
            .cloned()
            .collect();
        let bindh = meta.get_tmp_var();
        for catch in catch_values {
            catch.do_catch(meta, &**bindh);
        }

        let key = Self::make_valkey(bindh.clone());
        let r#const = Const(key, *value, labels);
        r#const.compile(meta);

        *self = Self::Inited { bind_handle: bindh, vars };
    }
}
impl TakeHandle for ClosuredValue {
    fn take_handle(mut self, meta: &mut CompileMeta) -> Var {
        self.catch_vars(meta);
        let Self::Inited { bind_handle, vars } = self else {
            panic!("self uninit, {:?}", self)
        };
        let mut result = None;
        meta.with_block(|meta| {
            for var in vars {
                let handle = meta.get_value_binded(
                    bind_handle.clone(),
                    var.clone(),
                );
                Const(ConstKey::Var(var), handle.into(), Vec::new())
                    .compile(meta);
            }
            result = Self::make_valkey(bind_handle)
                .take_handle(meta)
                .into();
        });
        result.unwrap()
    }
}

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
            result: default(),
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

        let dexp_res_is_alloced = result.is_empty();
        if dexp_res_is_alloced {
            result = meta.get_tmp_var(); /* init tmp_var */
        } else if let Some(ConstData { value, .. })
                = meta.get_const_value(&result) {
            // 对返回句柄使用常量值的处理
            if !value.is_var() {
                err!(
                    concat!(
                        "{}\n尝试在`DExp`的返回句柄处使用值不为Var的const, ",
                        "此处仅允许使用`Var`\n",
                        "值: {:#?}\n",
                        "名称: {:?}",
                    ),
                    meta.err_info().join("\n"),
                    value,
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
impl_derefs!(impl for DExp => (self: self.lines): Expand);

/// 将一个Value与一个Var以特定格式组合起来,
/// 可完成如属性调用的功能
#[derive(Debug, PartialEq, Clone)]
pub struct ValueBind(pub Box<Value>, pub Var);
impl ValueBind {
    pub fn take_unfollow_handle(self, meta: &mut CompileMeta) -> Var {
        let handle = self.0.take_handle(meta);
        meta.get_value_binded(handle, self.1)
    }
}
impl TakeHandle for ValueBind {
    fn take_handle(self, meta: &mut CompileMeta) -> Var {
        self.take_unfollow_handle(meta)
            .take_handle(meta)  // 进行通常是全局表的常量表查询
    }
}

/// 进行`词法&语法`分析时所依赖的元数据
#[derive(Debug)]
pub struct Meta {
    tmp_var_count: usize,
    tag_number: usize,
    /// 被跳转的label
    defined_labels: Vec<HashSet<Var>>,
    break_labels: Vec<Option<Var>>,
    continue_labels: Vec<Option<Var>>,
}
impl Default for Meta {
    fn default() -> Self {
        Self {
            tmp_var_count: 0,
            tag_number: 0,
            defined_labels: vec![HashSet::new()],
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
        self.tmp_var_count += 1;
        gen_anon_name(var, |arg| format!("___{}", arg)).into()
    }

    /// 获取一个标签, 并且进行内部自增以保证不会获取到获取过的
    pub fn get_tag(&mut self) -> Var {
        let tag = self.tag_number;
        self.tag_number += 1;
        let fmtted_arg = gen_anon_name(tag, |arg|
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
        LogicLine::Other(Args::Normal(vec![
                Value::ReprVar("set".into()),
                var,
                value,
        ]))
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
            => default(),
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

    /// 获取两个运算成员, 如果是没有运算成员的则返回空
    pub fn get_values_ref_mut(&mut self) -> Option<(&mut Value, &mut Value)> {
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

    /// Returns `true` if the jump cmp is [`Always`].
    ///
    /// [`Always`]: JumpCmp::Always
    #[must_use]
    pub fn is_always(&self) -> bool {
        matches!(self, Self::Always)
    }
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
impl FromMdtArgs for JumpCmp {
    type Err = LogicLineFromTagError;

    fn from_mdt_args(args: &[&str]) -> Result<Self, Self::Err> {
        let &[oper, a, b] = args else {
            return Err(JumpCmpRParseError::ArgsCountError(
                args.iter().cloned().map(Into::into).collect()
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
impl Default for JumpCmp {
    fn default() -> Self {
        Self::Always
    }
}

/// JumpCmp语法树从字符串生成时的错误
#[derive(Debug, PartialEq, Clone)]
pub enum JumpCmpRParseError {
    ArgsCountError(Vec<String>),
    UnknownComparer(String, [String; 2]),
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
    /// 整棵条件树的依赖
    Deps(InlineBlock, Box<Self>),
    And(Box<Self>, Box<Self>),
    Or(Box<Self>, Box<Self>),
    Atom(JumpCmp),
}
impl CmpTree {
    pub const ALWAYS: Self = Self::Atom(JumpCmp::Always);
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
            Self::Deps(deps, cmp)
                => Self::Deps(deps, cmp.reverse().into()),
            Self::Or(a, b)
                => Self::And(a.reverse().into(), b.reverse().into()),
            Self::And(a, b)
                => Self::Or(a.reverse().into(), b.reverse().into()),
            Self::Atom(cmp)
                => Self::Atom(cmp.reverse()),
        }
    }

    /// 尝试进行内联处理, 断言自身为Atom
    pub fn try_inline(&mut self, meta: &mut CompileMeta) {
        use {
            JumpCmp as JC,
            Value as V,
            LogicLine as LL,
        };
        do_return!(let Self::Atom(this) = self);
        fn get<'a>(meta: &'a CompileMeta, value: &'a V) -> Option<&'a str> {
            fn f<'a>(meta: &'a CompileMeta, s: &'a Var) -> Option<&'a str> {
                match meta.get_const_value(s) {
                    Some(ConstData { value: V::Var(s), .. }) => Some(&**s),
                    Some(_) => None,
                    None => Some(&**s),
                }
            }
            match value {
                | V::Var(s)
                => {
                    match meta.get_const_value(s) {
                        | Some(ConstData { value: V::Var(s), .. })
                        => Some(&**s),
                        // 二级展开 A=0; B=(A:); use B;
                        | Some(ConstData {
                            value: V::DExp(DExp {
                                result: s,
                                lines
                            }),
                            ..
                        }) => {
                            if lines.is_empty() {
                                f(meta, s)
                            } else {
                                None
                            }
                        },
                        Some(_) => None,
                        None => Some(&**s),
                    }
                },
                | V::DExp(DExp { result: s, lines })
                => {
                    if lines.is_empty() {
                        f(meta, s)
                    } else {
                        None
                    }
                },
                | V::ReprVar(s)
                => Some(&**s),
                | V::ResultHandle
                => Some(&**meta.dexp_handle()),
                | V::Binder
                => Some(meta.get_dexp_expand_binder().map(|s| &***s).unwrap_or("__")),
                | V::ValueBind(_)
                | V::ValueBindRef(_)
                | V::Cmper(_)
                | V::BuiltinFunc(_)
                | V::ClosuredValue(_)
                => None,
            }
        }
        /// 是否为假, 如果为真或无效返回否
        fn is_false(meta: &mut CompileMeta, value: &V) -> bool {
            do_return!(let Some(value) = get(meta, value) => false);
            value == FALSE_VAR || value == ZERO_VAR
        }
        fn check_inline_op(dexp: &DExp) -> bool {
            do_return!(! (dexp.result.is_empty() && dexp.len() == 1) => false);
            do_return!(let LL::Op(op) = &dexp[0] => false);
            do_return!(! op.get_result().is_result_handle() => false);
            do_return!(let Some(_) = op.get_cmper() => false);
            true
        }
        let (left, right) = match this {
            | JC::Equal(lhs, rhs)
            | JC::NotEqual(lhs, rhs)
            => {
                (is_false(meta, lhs), is_false(meta, rhs))
            },
            | _ => return,
        };
        do_return!(left == right);
        let cmp_rev = if let JC::Equal(..) = this {
            |cmp: CmpTree| cmp.reverse()
        } else {
            identity
        };
        let values = this.get_values_ref_mut().unwrap();
        let value = if left { values.1 } else { values.0 };
        match value {
            V::Cmper(cmper) => {
                // (a < b) != false => a < b
                // (a < b) == false => !(a < b)
                let cmper: CmpTree = mem::take(&mut **cmper);
                *self = cmp_rev(cmper)
            }
            V::DExp(dexp) => {
                do_return!(! check_inline_op(dexp));
                let LL::Op(op) = dexp.pop().unwrap() else { unreachable!() };
                let cmper = op.get_cmper().unwrap();
                let info = op.into_info();
                let cmp = cmper(info.arg1, info.arg2.unwrap());
                *self = cmp_rev(cmp.into())

            }
            V::Var(name) => {
                match meta.get_const_value(name) {
                    Some(ConstData { value: V::DExp(dexp), .. }) => {
                        do_return!(! check_inline_op(dexp));
                        let op = dexp[0].as_op().unwrap().clone();
                        let cmper = op.get_cmper().unwrap();
                        let info = op.into_info();
                        let cmp = cmper(info.arg1, info.arg2.unwrap());
                        *self = cmp_rev(cmp.into())
                    }
                    Some(ConstData { value: V::Cmper(cmper), .. }) => {
                        *self = cmp_rev((**cmper).clone())
                    }
                    Some(_) | None => return,
                }
            }
            _ => return,
        }
        self.try_inline(meta)

    }

    /// 构建条件树为goto
    pub fn build(mut self, meta: &mut CompileMeta, do_tag: Var) {
        use CmpTree::*;

        // 获取如果在常量展开内则被重命名后的标签
        let do_tag_expanded = meta.get_in_const_label(do_tag);
        self.try_inline(meta);

        match self {
            Deps(deps, cmp) => {
                meta.with_block(|meta| {
                    deps.compile(meta);
                    cmp.build(meta, do_tag_expanded);
                });
            },
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

    pub fn is_always(&self) -> bool {
        match self {
            CmpTree::Deps(_, cond) => cond.is_always(),
            CmpTree::And(a, b) => a.is_always() && b.is_always(),
            CmpTree::Or(a, b) => a.is_always() && b.is_always(),
            CmpTree::Atom(atom) => atom.is_always(),
        }
    }
}
impl Default for CmpTree {
    fn default() -> Self {
        Self::Atom(default())
    }
}
impl_enum_froms!(impl From for CmpTree {
    Atom => JumpCmp;
});

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
        let mut args: Vec<String> = Vec::with_capacity(5);

        args.push("op".into());
        args.push(info.oper_str.into());
        args.push(info.result.take_handle(meta).into());
        args.push(info.arg1.take_handle(meta).into());
        args.push(
            info.arg2
                .map(|arg| arg.take_handle(meta))
                .map(Into::into)
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

    /// 在输出值为返回句柄替换符时, 尝试编译期计算它
    pub fn try_eval_const_num(&self, meta: &CompileMeta) -> Option<f64> {
        use std::num::FpCategory as FpC;
        fn conv(n: f64) -> Option<f64> {
            match n.classify() {
                FpC::Nan => 0.0.into(),
                FpC::Infinite => None,
                _ => n.into(),
            }
        }
        fn bool_as(x: bool) -> f64 {
            if x { 1. } else { 0. }
        }
        let OpInfo { result, arg1, arg2, .. } = self.get_info();
        result.as_result_handle()?;
        let (a, b) = (
            arg1.try_eval_const_num(meta)?.0,
            match arg2 {
                Some(value) => value.try_eval_const_num(meta)?.0,
                None => 0.0,
            },
        );
        let (a, b) = (conv(a)?, conv(b)?);
        match self {
            Op::Add(..) => a + b,
            Op::Sub(..) => a - b,
            Op::Mul(..) => a * b,
            Op::Div(..) | Op::Idiv(..) | Op::Mod(..)
                if matches!(b.classify(), FpC::Zero | FpC::Subnormal) => f64::NAN,
            Op::Div(..) => a / b,
            Op::Idiv(..) => (a / b).floor(),
            Op::Mod(..) => a % b,
            Op::Pow(..) => a.powf(b),
            Op::Abs(..) => a.abs(),
            Op::Log(..) | Op::Log10(..) if a <= 0. => f64::NAN,
            Op::Log(..) => a.ln(),
            Op::Log10(..) => a.log10(),
            Op::Floor(..) => a.floor(),
            Op::Ceil(..) => a.ceil(),
            Op::Sqrt(..) => a.sqrt(),
            Op::Sin(..) => a.to_radians().sin(),
            Op::Cos(..) => a.to_radians().cos(),
            Op::Tan(..) => a.to_radians().tan(),
            Op::Asin(..) => a.asin().to_degrees(),
            Op::Acos(..) => a.acos().to_degrees(),
            Op::Atan(..) => a.atan().to_degrees(),

            Op::Equal(..) => bool_as(a == b),
            Op::NotEqual(..) => bool_as(a != b),
            Op::Land(..) => bool_as(a != 0. && b != 0.),
            Op::LessThan(..) => bool_as(a < b),
            Op::LessThanEq(..) => bool_as(a <= b),
            Op::GreaterThan(..) => bool_as(a > b),
            Op::GreaterThanEq(..) => bool_as(a >= b),

            Op::Shl(..) => ((a as i64) << b as i64) as f64,
            Op::Shr(..) => ((a as i64) >> b as i64) as f64,
            Op::Or(..) => ((a as i64) | b as i64) as f64,
            Op::And(..) => ((a as i64) & b as i64) as f64,
            Op::Xor(..) => ((a as i64) ^ b as i64) as f64,
            Op::Not(..) => !(a as i64) as f64,

            Op::Max(..) => a.max(b),
            Op::Min(..) => a.min(b),

            // Not Impl
            | Op::StrictEqual(..)
            | Op::Angle(..)
            | Op::Len(..)
            | Op::Noise(..)
            | Op::Rand(..) => None?,
        }.into()
    }
}
impl Compile for Op {
    fn compile(self, meta: &mut CompileMeta) {
        let args = self.generate_args(meta);
        meta.tag_codes.push(args.join(" ").into())
    }
}
impl FromMdtArgs for Op {
    type Err = OpRParseError;

    fn from_mdt_args(args: &[&str]) -> Result<Self, Self::Err> {
        let &[oper, res, a, b] = args else {
            return Err(OpRParseError::ArgsCountError(
                args.iter().cloned().map(Into::into).collect()
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
impl Goto {
    pub fn cond(&self) -> &CmpTree {
        &self.1
    }

    pub fn cond_mut(&mut self) -> &mut CmpTree {
        &mut self.1
    }

    pub fn is_always(&self) -> bool {
        self.cond().is_always()
    }
}
impl Compile for Goto {
    fn compile(self, meta: &mut CompileMeta) {
        self.1.build(meta, self.0)
    }
}

#[derive(Debug, PartialEq, Clone)]
#[derive(Default)]
pub struct Expand(pub Vec<LogicLine>);
impl Compile for Expand {
    fn compile(self, meta: &mut CompileMeta) {
        meta.with_block(|this| {
            this.with_env_args_block(|this| {
                for line in self.0 {
                    line.compile(this)
                }
            });
        });
    }
}
impl From<Vec<LogicLine>> for Expand {
    fn from(value: Vec<LogicLine>) -> Self {
        Self(value)
    }
}
impl<const N: usize> From<[LogicLine; N]> for Expand {
    fn from(value: [LogicLine; N]) -> Self {
        Self(value.into())
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
impl FromIterator<LogicLine> for Expand {
    fn from_iter<T: IntoIterator<Item = LogicLine>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}
impl_derefs!(impl for Expand => (self: self.0): Vec<LogicLine>);

#[derive(Debug, PartialEq, Clone)]
pub struct InlineBlock(pub Vec<LogicLine>);
impl Compile for InlineBlock {
    fn compile(self, meta: &mut CompileMeta) {
        for line in self.0 {
            line.compile(meta)
        }
    }
}
impl From<Vec<LogicLine>> for InlineBlock {
    fn from(value: Vec<LogicLine>) -> Self {
        Self(value)
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
        let target = self.0;

        if let 0 | 1 = cases.len() {
            Take("__".into(), target).compile(meta);
            if let Some(case) = cases.pop() {
                meta.tag_codes_mut().extend(case)
            }
            assert!(cases.is_empty(), "{}", cases.len());
            return
        }

        let simple_select_len = match max_len {
            0 => 0,
            1 => cases.len() + 1,
            n => n * cases.len() + 2,
        };
        let goto_table_select_len = match max_len {
            0 => 0,
            _ => cases.len() + 1 + cases.iter().map(Vec::len).sum::<usize>(),
        };

        #[cfg(debug_assertions)]
        let old_tag_codes_len = meta.tag_codes.count_no_tag();
        if simple_select_len <= goto_table_select_len {
            Self::build_simple_select(target, max_len, meta, lens, cases);
            #[cfg(debug_assertions)]
            assert_eq!(
                meta.tag_codes.count_no_tag(),
                old_tag_codes_len + simple_select_len,
                "预期长度公式错误\n{}",
                meta.tag_codes,
            );
        } else {
            Self::build_goto_table_select(target, max_len, meta, lens, cases);
            #[cfg(debug_assertions)]
            assert_eq!(
                meta.tag_codes.count_no_tag(),
                old_tag_codes_len + goto_table_select_len,
                "预期长度公式错误\n{}",
                meta.tag_codes,
            );
        }
    }
}

impl Select {
    fn build_goto_table_select(
        target: Value,
        max_len: usize,
        meta: &mut CompileMeta,
        lens: Vec<usize>,
        cases: Vec<Vec<TagLine>>,
    ) {
        let counter = Value::ReprVar(COUNTER.into());

        if max_len == 0 {
            return Self::build_simple_select(target, max_len, meta, lens, cases)
        }

        Op::Add(
            counter.clone(),
            counter,
            target
        ).compile(meta);

        let tmp_tags: Vec<Var>
            = repeat_with(|| meta.get_tmp_tag())
            .take(cases.len())
            .collect();
        tmp_tags.iter()
            .cloned()
            .map(|tag| Goto(tag, CmpTree::default()))
            .for_each(|goto| goto.compile(meta));

        let mut tags_iter = tmp_tags.into_iter();
        for case in cases {
            let tag = tags_iter.next().unwrap();
            LogicLine::Label(tag).compile(meta);
            meta.tag_codes.lines_mut().extend(case);
        }
    }

    fn build_simple_select(
        target: Value,
        max_len: usize,
        meta: &mut CompileMeta,
        lens: Vec<usize>,
        mut cases: Vec<Vec<TagLine>>,
    ) {
        let counter = Value::ReprVar(COUNTER.into());

        // build head
        let head = match max_len {
            0 => {          // no op
                Take("__".into(), target).compile_take(meta)
            },
            1 => {          // no mul
                Op::Add(
                    counter.clone(),
                    counter,
                    target
                ).compile_take(meta)
            },
            // normal
            _ => {
                let tmp_var = meta.get_tmp_var();
                let mut head = Op::Mul(
                    tmp_var.clone().into(),
                    target,
                    Value::ReprVar(max_len.to_string().into())
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
            match max_len - len {
                0 => continue,
                insert_counts => {
                    let end_tag = meta.get_tmp_tag();
                    let end_tag = meta.get_tag(end_tag);
                    case.push(TagLine::Jump(
                            tag_code::Jump::new_always(end_tag).into()
                    ));
                    case.extend(
                        repeat_with(||
                            LogicLine::NoOp
                                .compile_take(meta))
                            .take(insert_counts - 1)
                            .flatten()
                    );
                    case.push(TagLine::TagDown(end_tag));
                },
            }
        }

        let lines = meta.tag_codes.lines_mut();
        lines.extend(head);
        lines.extend(cases.into_iter().flatten());
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
                Value::ReprVar(max_case.to_string().into())
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

#[derive(Debug, PartialEq, Clone)]
pub enum ConstKey {
    Var(Var),
    ValueBind(ValueBind),
}
impl ConstKey {
    /// 当自身为Var时, 通过给定值转换成ValueBind
    /// 如果自身已经是ValueBind, 那么将会通过错误将给定值返回
    pub fn name_to_bind<T>(&mut self, bind: T) -> Result<(), T>
    where T: Into<Box<Value>>
    {
        match self {
            ConstKey::Var(var) => {
                let name = mem::take(var);
                *self = Self::ValueBind(ValueBind(bind.into(), name));
                Ok(())
            },
            ConstKey::ValueBind(_) => Err(bind),
        }
    }

    /// Returns `true` if the const key is [`Var`].
    ///
    /// [`Var`]: ConstKey::Var
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

    pub fn as_var_mut(&mut self) -> Option<&mut Var> {
        if let Self::Var(v) = self {
            Some(v)
        } else {
            None
        }
    }

    /// Returns `true` if the const key is [`ValueBind`].
    ///
    /// [`ValueBind`]: ConstKey::ValueBind
    #[must_use]
    pub fn is_value_bind(&self) -> bool {
        matches!(self, Self::ValueBind(..))
    }

    pub fn as_value_bind(&self) -> Option<&ValueBind> {
        if let Self::ValueBind(v) = self {
            Some(v)
        } else {
            None
        }
    }
}
impl From<ConstKey> for Value {
    fn from(value: ConstKey) -> Self {
        match value {
            ConstKey::Var(var) => var.into(),
            ConstKey::ValueBind(vb) => vb.into(),
        }
    }
}
impl TakeHandle for ConstKey {
    fn take_handle(self, meta: &mut CompileMeta) -> Var {
        match self {
            Self::Var(var) => var,
            Self::ValueBind(vb) => vb.take_handle(meta),
        }
    }
}
impl_enum_froms!(impl From for ConstKey {
    Var => &str;
    Var => String;
    Var => &Var;
    Var => Var;
    ValueBind => ValueBind;
});

/// 在块作用域将Var常量为后方值, 之后使用Var时都会被替换为后方值
#[derive(Debug, PartialEq, Clone)]
pub struct Const(pub ConstKey, pub Value, pub Vec<Var>);
impl Const {
    pub fn new(var: ConstKey, value: Value) -> Self {
        Self(var, value, default())
    }

    /// 在const编译前对右部分进行处理, 如果目标有的话, 返回绑定者
    pub fn run_value(&mut self, meta: &mut CompileMeta) -> Option<Var> {
        let value = &mut self.1;
        match value {
            Value::ReprVar(var) => {
                let var = mem::take(var);
                *value = Value::Var(var)
            },
            Value::Var(var) => {
                if let Some(data) = meta.get_const_value(var) {
                    return self.extend_data(data.clone());
                }
            },
            Value::ClosuredValue(closure) => closure.catch_vars(meta),
            val @ Value::ValueBindRef(_) => {
                let Value::ValueBindRef(bindref)
                    = replace(val, Value::ResultHandle)
                    else { unreachable!() };
                let data = bindref.do_follow(meta);
                match data {
                    Either::Left(var) => {
                        return self.extend_data(ConstData {
                            value: var.into(),
                            labels: vec![],
                            binder: None,
                        });
                    },
                    Either::Right(data) => {
                        return self.extend_data(data.clone());
                    },
                }
            },
            _ => (),
        }
        None
    }

    /// 使右半部分继承某个const值, 如果目标有的话, 返回绑定者
    fn extend_data(
        &mut self,
        ConstData { value, labels, binder }: ConstData,
    ) -> Option<Var> {
        self.1 = value;
        self.2 = labels;
        binder
    }
}
impl Compile for Const {
    fn compile(mut self, meta: &mut CompileMeta) {
        // 对同作用域定义过的常量形成覆盖
        // 如果要进行警告或者将信息传出则在此处理
        let extra_binder = self.run_value(meta);
        meta.add_const_value_with_extra_binder(self, extra_binder);
    }
}

/// 在此处计算后方的值, 并将句柄赋给前方值
/// 如果后方不是一个DExp, 而是Var, 那么自然等价于一个常量定义
#[derive(Debug, PartialEq, Clone)]
pub struct Take(pub ConstKey, pub Value);
impl Take {
    /// 过时的API
    ///
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
        args: Args,
        var: Var,
        do_leak_res: bool,
        value: Value,
    ) -> LogicLine {
        if matches!(args, Args::Normal(ref args) if args.is_empty()) {
            Take(var.into(), value).into()
        } else {
            let mut len = 2;
            if do_leak_res { len += 1 }
            let mut expand = Vec::with_capacity(len);
            expand.push(LogicLine::SetArgs(args));
            if do_leak_res {
                expand.push(Take(var.clone().into(), value).into());
                expand.push(LogicLine::ConstLeak(var));
            } else {
                expand.push(Take(var.into(), value).into())
            }
            debug_assert_eq!(expand.len(), len);
            Expand(expand).into()
        }
    }
}
impl Compile for Take {
    fn compile(self, meta: &mut CompileMeta) {
        let r#const = Const::new(self.0, self.1.take_handle(meta).into());
        meta.add_const_value(r#const);
    }
}

/// 可能含有一个展开的Args
#[derive(Debug, PartialEq, Clone)]
pub enum Args {
    /// 正常的参数
    Normal(Vec<Value>),
    /// 夹杂一个展开的参数
    Expanded(Vec<Value>, Vec<Value>),
}
impl Args {
    pub const GLOB_ONLY: Self = Self::Expanded(vec![], vec![]);

    /// 获取值
    pub fn into_value_args(self, meta: &CompileMeta) -> Vec<Value> {
        match self {
            Self::Normal(args) => args,
            Self::Expanded(left, right) => {
                left.into_iter()
                    .chain(meta.get_env_args().iter().cloned().map(Into::into))
                    .chain(right)
                    .collect()
            },
        }
    }

    /// 获取句柄, 但是假定环境中的args已经const过了
    ///
    /// 这不包括左部分和右部分
    pub fn into_taked_args_handle(self, meta: &mut CompileMeta) -> Vec<Var> {
        match self {
            Args::Normal(args) => {
                args.into_iter()
                    .map(|value| value.take_handle(meta))
                    .collect()
            },
            Args::Expanded(left, right) => {
                let expanded_args: Vec<Var>
                    = meta.get_env_args().iter().cloned().collect();
                left.into_iter()
                    .chain(expanded_args.into_iter()
                        .map(Value::Var))
                    .chain(right)
                    .map(|value| value.take_handle(meta))
                    .collect()
            },
        }
    }

    pub fn base_len(&self) -> usize {
        match self {
            Self::Normal(args) => args.len(),
            Self::Expanded(prefix, suffix) => prefix.len() + suffix.len(),
        }
    }

    /// Returns `true` if the args is [`Normal`].
    ///
    /// [`Normal`]: Args::Normal
    #[must_use]
    pub fn is_normal(&self) -> bool {
        matches!(self, Self::Normal(..))
    }

    pub fn as_normal(&self) -> Option<&Vec<Value>> {
        if let Self::Normal(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn as_normal_mut(&mut self) -> Option<&mut Vec<Value>> {
        if let Self::Normal(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn try_into_normal(self) -> Result<Vec<Value>, Self> {
        match self {
            Self::Normal(args) => Ok(args),
            this => Err(this),
        }
    }

    /// Returns `true` if the args is [`Expanded`].
    ///
    /// [`Expanded`]: Args::Expanded
    #[must_use]
    pub fn is_expanded(&self) -> bool {
        matches!(self, Self::Expanded(..))
    }
}
impl Default for Args {
    fn default() -> Self {
        Self::Normal(vec![])
    }
}
impl<const N: usize> From<[Value; N]> for Args {
    fn from(value: [Value; N]) -> Self {
        Self::Normal(value.into())
    }
}
impl_enum_froms!(impl From for Args {
    Normal => Vec<Value>;
});

/// 拿取指定个参数, 并重复块中代码
#[derive(Debug, PartialEq, Clone)]
pub struct ArgsRepeat {
    count: usize,
    block: InlineBlock,
}
impl ArgsRepeat {
    pub fn new(count: usize, block: InlineBlock) -> Self {
        Self { count, block }
    }

    pub fn count(&self) -> usize {
        self.count
    }

    pub fn count_mut(&mut self) -> &mut usize {
        &mut self.count
    }

    pub fn block(&self) -> &InlineBlock {
        &self.block
    }

    pub fn block_mut(&mut self) -> &mut InlineBlock {
        &mut self.block
    }
}
impl Compile for ArgsRepeat {
    fn compile(self, meta: &mut CompileMeta) {
        let chunks: Vec<Vec<Value>> = meta.get_env_args()
            .chunks(self.count)
            .map(|chunks| chunks.iter()
                .cloned()
                .map(Into::into)
                .collect())
            .collect();
        for args in chunks {
            let args = Vec::from_iter(args.iter().cloned());
            meta.with_env_args_block(|meta| {
                meta.set_env_args(args);
                self.block.clone().compile(meta)
            });
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Match {
    args: Args,
    cases: Vec<(MatchPat, InlineBlock)>,
}
impl Compile for Match {
    fn compile(self, meta: &mut CompileMeta) {
        let args = self.args.into_taked_args_handle(meta);
        for (pat, block) in self.cases {
            if pat.do_pattern(&args, meta) {
                block.compile(meta);
                return;
            }
        }
        if meta.enable_misses_match_log_info {
            meta.log_info(format_args!("Misses match, [{}]", args.join(" ")));
            meta.log_expand_stack::<false>();
        }
    }
}
impl Match {
    pub fn new(args: Args, cases: Vec<(MatchPat, InlineBlock)>) -> Self {
        Self { args, cases }
    }

    pub fn args(&self) -> &Args {
        &self.args
    }

    pub fn cases(&self) -> &[(MatchPat, InlineBlock)] {
        self.cases.as_ref()
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum MatchPat {
    Normal(Vec<MatchPatAtom>),
    Expanded(Vec<MatchPatAtom>, Vec<MatchPatAtom>),
}
impl MatchPat {
    pub const GLOB_ONLY: Self = Self::Expanded(vec![], vec![]);

    pub fn base_len(&self) -> usize {
        match self {
            Self::Normal(args) => args.len(),
            Self::Expanded(prefix, suffix) => prefix.len() + suffix.len(),
        }
    }

    /// 进行匹配, 如果成功则直接将量绑定
    pub fn do_pattern(self, args: &[Var], meta: &mut CompileMeta) -> bool {
        fn to_vars(args: Vec<MatchPatAtom>, meta: &mut CompileMeta)
        -> Vec<(Var, Vec<Var>)> {
            args.into_iter()
                .map(|arg| (
                        arg.name,
                        arg.pattern.into_iter()
                        .map(|pat| pat.take_handle(meta))
                        .collect()
                ))
                .collect()
        }
        fn cmp(pats: &[(Var, Vec<Var>)], args: &[Var]) -> bool {
            pats.iter()
                .map(|(_, x)| &x[..])
                .zip(args)
                .all(|(pat, var)| {
                    pat.is_empty()
                        || pat.iter().any(|x| x == var)
                })
        }
        fn binds(name: Var, value: &Var, meta: &mut CompileMeta) {
            if !name.is_empty() {
                meta.add_const_value(Const(name.into(), value.clone().into(), vec![]));
            }
        }
        match self {
            Self::Normal(iargs) if iargs.len() == args.len() => {
                let pats: Vec<(Var, Vec<Var>)> = to_vars(iargs, meta);
                cmp(&pats, args).then(|| {
                    for ((name, _), arg) in pats.into_iter().zip(args) {
                        binds(name, arg, meta)
                    }
                }).is_some()
            },
            Self::Expanded(prefix, suffix)
            if self.base_len() <= args.len() => {
                let (prefix, suffix)
                    = (to_vars(prefix, meta), to_vars(suffix, meta));
                let tl = args.len()-suffix.len();
                let extracted = &args[prefix.len()..tl];
                (cmp(&prefix, args) && cmp(&suffix, &args[tl..]))
                    .then(|| {
                        let (a, b) = (
                            prefix.into_iter().zip(args),
                            suffix.into_iter().zip(&args[tl..]),
                        );
                        for ((name, _), arg) in a.chain(b) {
                            binds(name, arg, meta)
                        }
                        meta.set_env_args(extracted)
                    }).is_some()
            },
            _ => false,
        }
    }
}
impl From<Vec<MatchPatAtom>> for MatchPat {
    fn from(value: Vec<MatchPatAtom>) -> Self {
        Self::Normal(value)
    }
}

#[derive(Debug, PartialEq, Clone, Default)]
pub struct MatchPatAtom {
    name: Var,
    pattern: Vec<Value>,
}
impl MatchPatAtom {
    pub fn new(name: Var, pattern: Vec<Value>) -> Self {
        Self { name, pattern }
    }

    pub fn new_unamed(pattern: Vec<Value>) -> Self {
        Self::new("".into(), pattern)
    }

    pub fn name(&self) -> &str {
        self.name.as_ref()
    }

    pub fn pattern(&self) -> &[Value] {
        self.pattern.as_ref()
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct ConstMatch {
    args: Args,
    cases: Vec<(ConstMatchPat, InlineBlock)>,
}
impl ConstMatch {
    pub fn new(args: Args, cases: Vec<(ConstMatchPat, InlineBlock)>) -> Self {
        Self { args, cases }
    }

    pub fn args(&self) -> &Args {
        &self.args
    }

    pub fn cases(&self) -> &[(ConstMatchPat, InlineBlock)] {
        self.cases.as_ref()
    }
}
impl Compile for ConstMatch {
    fn compile(self, meta: &mut CompileMeta) {
        let args = self.args.into_value_args(meta)
            .into_iter()
            .map(|value| {
                if let Some(var) = value.as_var() {
                    if meta.get_const_value(var).is_some() {
                        return value.try_into().unwrap();
                    }
                }
                let handle = meta.get_tmp_var();
                Const(
                    handle.clone().into(),
                    value,
                    vec![],
                ).compile(meta);
                handle
            })
            .collect::<Vec<_>>();
        for (pat, code) in self.cases {
            if let Ok(()) = pat.do_pattern(meta, &args, code) {
                return;
            }
        }
        if meta.enable_misses_match_log_info {
            meta.log_info(format_args!(
                "Misses const match, [{}]",
                args.join(" "),
            ));
            meta.log_expand_stack::<false>();
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum ConstMatchPat {
    Normal(Vec<ConstMatchPatAtom>),
    Expanded(Vec<ConstMatchPatAtom>, Vec<ConstMatchPatAtom>),
}
impl ConstMatchPat {
    pub fn check_len(&self, value_len: usize) -> bool {
        match self {
            ConstMatchPat::Normal(norm) => norm.len() == value_len,
            ConstMatchPat::Expanded(left, right) => {
                left.len() + right.len() <= value_len
            },
        }
    }

    pub fn do_pattern<'a, C>(
        self,
        meta: &'a mut CompileMeta,
        handles: &[Var],
        code: C,
    ) -> Result<(), C>
    where C: Compile,
    {
        fn do_iter<'a>(
            iter: impl IntoIterator<Item = ConstMatchPatAtom>,
            meta: &mut CompileMeta,
            handles: &'a [Var],
        ) -> Option<Vec<(bool, Var, &'a Var)>> {
            iter.into_iter()
                .zip(handles)
                .map(|(pat, handle)| {
                    pat.pat(meta, handle)
                        .map(|(do_take, name)| (do_take, name, handle))
                })
                .collect()
        }
        fn make(
            (do_take, name, handle): (bool, Var, &Var),
            meta: &mut CompileMeta,
        ) {
            if do_take {
                Take(
                    name.is_empty()
                        .then(|| "__".into())
                        .unwrap_or(name)
                        .into(),
                    handle.into(),
                ).compile(meta);
            } else if !name.is_empty() {
                Const(
                    name.into(),
                    handle.clone().into(),
                    vec![],
                ).compile(meta);
            }
        }

        if !self.check_len(handles.len()) {
            return Err(code);
        }
        match self {
            Self::Normal(norm) => {
                let Some(datas)
                    = do_iter(norm, meta, handles)
                    else {
                        return Err(code);
                    };
                for ele in datas {
                    make(ele, meta);
                }
                code.compile(meta);
                Ok(())
            },
            Self::Expanded(left, right) => {
                let mid_rng = left.len()..handles.len()-right.len();
                let Some(left_datas)
                    = do_iter(left, meta, handles)
                    else {
                        return Err(code);
                    };
                let Some(right_datas)
                    = do_iter(right, meta, &handles[mid_rng.end..])
                    else {
                        return Err(code);
                    };
                for ele in left_datas {
                    make(ele, meta);
                }
                for ele in right_datas {
                    make(ele, meta);
                }
                meta.set_env_args(&handles[mid_rng]);
                code.compile(meta);
                Ok(())
            },
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct ConstMatchPatAtom {
    do_take: bool,
    name: Var,
    pattern: Either<Vec<Value>, Value>,
}
impl ConstMatchPatAtom {
    pub fn new(do_take: bool, name: Var, pattern: Vec<Value>) -> Self {
        Self {
            do_take,
            name,
            pattern: Either::Left(pattern),
        }
    }

    pub fn new_guard(do_take: bool, name: Var, pattern: Value) -> Self {
        Self {
            do_take,
            name,
            pattern: Either::Right(pattern),
        }
    }

    pub fn new_unamed(do_take: bool, pattern: Vec<Value>) -> Self {
        Self::new(do_take, "".into(), pattern)
    }

    pub fn new_unamed_guard(do_take: bool, pattern: Value) -> Self {
        Self::new_guard(do_take, "".into(), pattern)
    }

    /// 尝试和某个句柄匹配, 返回是否take和需要给到的量
    pub fn pat(
        self,
        meta: &mut CompileMeta,
        handle: &Var,
    ) -> Option<(bool, Var)> {
        match self.pattern {
            Either::Left(pats) => {
                pats.is_empty() || {
                    let var = meta.get_const_value(handle)
                        .unwrap()
                        .value()
                        .as_var()
                        .cloned()?;
                    pats.into_iter()
                        .map(|v| v.take_handle(meta))
                        .any(|h| h == var)
                }
            },
            Either::Right(guard) => {
                let mut res = false;
                meta.with_block(|meta| {
                    LogicLine::SetArgs([handle.into()].into())
                        .compile(meta);
                    res = guard.take_handle(meta).ne("0")
                });
                res
            },
        }.then_some((self.do_take, self.name))
    }

    pub fn do_take(&self) -> bool {
        self.do_take
    }

    pub fn name(&self) -> &str {
        self.name.as_ref()
    }

    pub fn pattern(&self) -> &Either<Vec<Value>, Value> {
        &self.pattern
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum GSwitchCase {
    Catch {
        skip_extra: bool,
        underflow: bool,
        missed: bool,
        overflow: bool,
        to: Option<ConstKey>,
    },
    Normal {
        skip_extra: bool,
        /// 如果为空, 那么则继承上一个的编号
        ids: Args,
        guard: Option<CmpTree>,
    },
}
impl GSwitchCase {
    #[must_use]
    pub fn is_missed(&self) -> bool {
        matches!(self, Self::Catch { missed: true, .. })
    }

    #[must_use]
    pub fn is_underflow(&self) -> bool {
        matches!(self, Self::Catch { underflow: true, .. })
    }

    #[must_use]
    pub fn is_overflow(&self) -> bool {
        matches!(self, Self::Catch { overflow: true, .. })
    }

    /// Returns `true` if the gswitch case is [`Normal`].
    ///
    /// [`Normal`]: GSwitchCase::Normal
    #[must_use]
    pub fn is_normal(&self) -> bool {
        matches!(self, Self::Normal { .. })
    }

    /// Returns `true` if the gswitch case is [`Catch`].
    ///
    /// [`Catch`]: GSwitchCase::Catch
    #[must_use]
    pub fn is_catch(&self) -> bool {
        matches!(self, Self::Catch { .. })
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct GSwitch {
    pub value: Value,
    pub extra: Expand,
    pub cases: Vec<(GSwitchCase, Expand)>,
}
impl GSwitch {
    const MAX_CONST_ID: usize = 750;

    fn get_ids(ids: Args, meta: &mut CompileMeta) -> impl Iterator<
        Item = usize
    > + '_ {
        ids.into_taked_args_handle(meta)
            .into_iter()
            .map(Value::ReprVar)
            .map(|var| var.try_eval_const_num(meta)
                .map(|x| match x.0.round() {
                    n if n < 0. => {
                        meta.log_expand_stack::<true>();
                        err!("小于0的gswitch id: {}", n);
                        exit(4);
                    },
                    n if n >= GSwitch::MAX_CONST_ID as f64 => {
                        meta.log_expand_stack::<true>();
                        err!("过大的gswitch id: {}", n);
                        exit(4);
                    },
                    n => n as usize,
                })
                .unwrap_or_else(|| {
                    meta.log_expand_stack::<true>();
                    err!("gswitch id并不是一个数字");
                    exit(4);
                }))
    }

    fn case_lab_with_cond(
        &self, 
        mut f: impl FnMut(&GSwitchCase) -> bool,
        meta: &mut CompileMeta,
    ) -> Option<Var> {
        self.cases.iter()
            .find(|x| f(&x.0))
            .is_some()
            .then(|| meta.get_tmp_tag())
    }
}
impl Compile for GSwitch {
    fn compile(self, meta: &mut CompileMeta) {
        use GSwitchCase as GSC;

        meta.with_block(|meta| {
            let mut missed_lab = self
                .case_lab_with_cond(GSwitchCase::is_missed, meta);
            let underflow_lab = self
                .case_lab_with_cond(GSwitchCase::is_underflow, meta);
            let overflow_lab = self
                .case_lab_with_cond(GSwitchCase::is_overflow, meta);

            let val_h = meta.get_tmp_var();
            let mut head = Select(
                val_h.clone().into(),
                vec![].into(),
            );
            macro_rules! push_id {
                ($id:expr => $e:expr) => {{
                    let id = $id;
                    for _ in head.1.len()..=id {
                        head.1.push(Expand(vec![]).into())
                    }
                    head.1[id].as_expand_mut()
                        .unwrap()
                        .push($e.into());
                }};
            }
            let [mut prev_case, mut max_case] = [-1isize; 2];
            let mut case_lines = vec![];
            for (case, code) in self.cases {
                let GSC::Normal {
                    skip_extra,
                    mut ids,
                    guard
                } = case else {
                    let GSwitchCase::Catch {
                        skip_extra,
                        underflow,
                        missed,
                        overflow,
                        to,
                    } = case else {
                        unreachable!("{meta:#?}")
                    };
                    case_lines.push(LogicLine::Expand(
                        None.into_iter()
                            .chain(underflow.then(||
                                    underflow_lab.clone().unwrap())
                                .map(LogicLine::Label))
                            .chain(missed.then(||
                                    missed_lab.clone().unwrap())
                                .map(LogicLine::Label))
                            .chain(overflow.then(||
                                    overflow_lab.clone().unwrap())
                                .map(LogicLine::Label))
                            .chain(to.into_iter()
                                .map(|k| Take(k, val_h.clone().into()).into()))
                            .chain(once(code.into()))
                            .chain((!skip_extra)
                                .then(|| self.extra.0.iter().cloned())
                                .into_iter().flatten())
                            .collect()
                    ));
                    continue;
                };
                if ids.as_normal()
                    .map(Vec::is_empty)
                    .unwrap_or_default()
                {
                    let id = (prev_case+1).to_string();
                    ids.as_normal_mut()
                        .unwrap()
                        .push(Value::ReprVar(id.into()));
                }
                let cmp = guard.unwrap_or_default();
                let lab = meta.get_tmp_tag();
                Self::get_ids(ids, meta)
                    .for_each(|id| {
                        push_id!(id => Goto(lab.clone(), cmp.clone()));
                        prev_case = id.try_into().unwrap();
                        max_case = max_case.max(prev_case);
                    });
                let mut lines = vec![
                    LogicLine::Label(lab),
                    code.into(),
                ];
                if !skip_extra { lines.push(self.extra.clone().into()) }
                case_lines.push(LogicLine::Expand(lines.into()));
            }
            let mut default_missed_catch = false;
            for to_case in &mut *head.1 {
                let expand = to_case.as_expand_mut().unwrap();
                expand.last()
                    .and_then(|line| line.as_goto())
                    .map(|goto| !goto.is_always())
                    .unwrap_or(true)
                    .then(|| {
                        let lab = missed_lab
                            .get_or_insert_with(|| {
                                default_missed_catch = true;
                                meta.get_tmp_tag()
                            })
                            .clone();
                        expand.push(Goto(
                            lab,
                            CmpTree::ALWAYS,
                        ).into());
                    });
            }

            // generate
            LogicLine::from(Take(val_h.clone().into(), self.value))
                .compile(meta);
            underflow_lab
                .map(|lab| Goto(lab, JumpCmp::LessThan(
                    val_h.clone().into(),
                    Value::ReprVar("0".into()),
                ).into()))
                .map(LogicLine::from)
                .map(|line| line.compile(meta));
            overflow_lab
                .map(|lab| Goto(lab, JumpCmp::GreaterThan(
                    val_h.clone().into(),
                    Value::ReprVar(max_case.to_string().into()),
                ).into()))
                .map(LogicLine::from)
                .map(|line| line.compile(meta));
            LogicLine::from(head)
                .compile(meta);
            LogicLine::from(Expand(case_lines))
                .compile(meta);
            if default_missed_catch {
                LogicLine::Label(missed_lab.unwrap())
                    .compile(meta);
            }
        });
    }
}

#[must_use]
#[derive(Debug, PartialEq, Clone)]
pub enum LogicLine {
    Op(Op),
    /// 不要去直接创建它, 而是使用`new_label`去创建
    /// 否则无法将它注册到可能的`const`
    Label(Var),
    Goto(Goto),
    Other(Args),
    Expand(Expand),
    InlineBlock(InlineBlock),
    Select(Select),
    GSwitch(GSwitch),
    NoOp,
    /// 空语句, 什么也不生成
    Ignore,
    Const(Const),
    Take(Take),
    /// 将指定const在块末尾进行泄漏
    ConstLeak(Var),
    /// 将返回句柄设置为一个指定值
    SetResultHandle(Value),
    SetArgs(Args),
    ArgsRepeat(ArgsRepeat),
    Match(Match),
    ConstMatch(ConstMatch),
}
impl Compile for LogicLine {
    fn compile(self, meta: &mut CompileMeta) {
        match self {
            Self::NoOp => meta.push(meta.noop_line.clone().into()),
            Self::Label(mut lab) => {
                // 如果在常量展开中, 尝试将这个标记替换
                lab = meta.get_in_const_label(lab);
                let data = TagLine::TagDown(meta.get_tag(lab));
                meta.push(data)
            },
            Self::Other(args) => {
                let handles: Vec<Var> = args.into_taked_args_handle(meta);
                meta.push(TagLine::Line(handles.join(" ").into()));
            },
            Self::SetResultHandle(value) => {
                let new_dexp_handle = value.take_handle(meta);
                meta.set_dexp_handle(new_dexp_handle);
            },
            Self::SetArgs(args) => {
                fn iarg(i: usize) -> Var {
                    format!("_{i}").into()
                }
                let expand_args = args.into_value_args(meta);
                let len = expand_args.len();
                let mut f = |r#const: Const| r#const.compile(meta);
                let iter = expand_args.into_iter().enumerate();
                for (i, value) in iter {
                    f(Const(
                            iarg(i).into(),
                            value.into(),
                            Vec::with_capacity(0),
                    ).into());
                }
                meta.set_env_args((0..len).map(iarg));
            },
            Self::Select(select) => select.compile(meta),
            Self::GSwitch(gs) => gs.compile(meta),
            Self::Expand(expand) => expand.compile(meta),
            Self::InlineBlock(block) => block.compile(meta),
            Self::Goto(goto) => goto.compile(meta),
            Self::Op(op) => op.compile(meta),
            Self::Const(r#const) => r#const.compile(meta),
            Self::Take(take) => take.compile(meta),
            Self::ConstLeak(r#const) => meta.add_const_value_leak(r#const),
            Self::ArgsRepeat(args_repeat) => args_repeat.compile(meta),
            Self::Match(r#match) => r#match.compile(meta),
            Self::ConstMatch(r#match) => r#match.compile(meta),
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

    pub fn as_other(&self) -> Option<&Args> {
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

    pub fn as_expand_mut(&mut self) -> Option<&mut Expand> {
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
    GSwitch => GSwitch;
    Const => Const;
    Take => Take;
    ArgsRepeat => ArgsRepeat;
    Match => Match;
    ConstMatch => ConstMatch;
});
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
                    to_tag.to_string().into(),
                    match JumpCmp::from_mdt_args(&args) {
                        Ok(cmp) => cmp.into(),
                        Err(e) => return Err(e.into()),
                    }
                ).into())
            },
            TagLine::TagDown(tag) => Ok(Self::Label(tag.to_string().into())),
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
                        Ok(Self::Other(Args::Normal(args_value)))
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
    #[must_use]
    fn compile_take(self, meta: &mut CompileMeta) -> Vec<TagLine>
    where Self: Sized
    {
        let start = meta.tag_codes.len();
        self.compile(meta);
        meta.tag_codes.lines_mut().split_off(start)
    }
}

/// 一个常量映射的数据
#[derive(Debug, PartialEq, Clone)]
pub struct ConstData {
    value: Value,
    labels: Vec<Var>,
    binder: Option<Var>,
}
impl ConstData {
    pub fn new(value: Value, labels: Vec<Var>) -> Self {
        Self { value, labels, binder: None }
    }

    pub fn new_nolabel(value: Value) -> Self {
        Self::new(value, vec![])
    }

    pub fn set_binder(mut self, binder: Var) -> Self {
        self.binder = binder.into();
        self
    }

    pub fn value(&self) -> &Value {
        &self.value
    }

    pub fn labels(&self) -> &[Var] {
        self.labels.as_ref()
    }

    pub fn binder(&self) -> Option<&Var> {
        self.binder.as_ref()
    }

    pub fn binder_mut(&mut self) -> &mut Option<Var> {
        &mut self.binder
    }
}

fn num_radix<N>(mut n: N, radix: N) -> String
where N: ops::DivAssign + ops::Rem<Output = N>
         + Eq + Debug + Default + Into<usize>
         + Copy,
{
    const LIST: &[u8; 62]
        = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
    let zero = default();
    assert_ne!(radix, zero);
    let sub_list = &LIST[..radix.into()];
    if n == zero {
        return "0".into();
    }
    let mut res = Vec::new();
    while n != zero {
        let dig = n % radix;
        res.push(sub_list[dig.into()]);
        n /= radix;
    }
    res.reverse();
    String::from_utf8(res).unwrap()
}

/// 当编号过大时, 使用62进制进行编码, 降低长度
fn gen_anon_name<'a, R>(
    id: usize,
    f: impl FnOnce(fmt::Arguments<'_>) -> R,
) -> R {
    if id < 1000 {
        f(format_args!("{id}"))
    } else {
        f(format_args!("0{}", num_radix(id, 62)))
    }
}

/// 每层Expand的环境
#[derive(Debug, PartialEq, Clone)]
#[derive(Default)]
#[non_exhaustive]
pub struct ExpandEnv {
    leak_vars: Vec<Var>,
    consts: HashMap<Var, ConstData>,
}
impl ExpandEnv {
    pub fn new(leak_vars: Vec<Var>, consts: HashMap<Var, ConstData>) -> Self {
        Self {
            leak_vars,
            consts,
            ..default()
        }
    }

    pub fn leak_vars(&self) -> &[Var] {
        self.leak_vars.as_ref()
    }

    pub fn consts(&self) -> &HashMap<Var, ConstData> {
        &self.consts
    }

    pub fn consts_mut(&mut self) -> &mut HashMap<Var, ConstData> {
        &mut self.consts
    }

    pub fn leak_vars_mut(&mut self) -> &mut Vec<Var> {
        &mut self.leak_vars
    }
}

pub struct CompileMeta {
    /// 标记与`id`的映射关系表
    tags_map: HashMap<String, usize>,
    tag_count: usize,
    tag_codes: TagCodes,
    tmp_var_count: Counter<fn(&mut usize) -> Var>,
    expand_env: Vec<ExpandEnv>,
    env_args: Vec<Option<Vec<Var>>>,
    /// 每层DExp所使用的句柄, 末尾为当前层, 同时有一个是否为自动分配名称的标志
    dexp_result_handles: Vec<Var>,
    dexp_expand_binders: Vec<Option<Var>>,
    tmp_tag_count: Counter<fn(&mut usize) -> Var>,
    /// 每层const展开的标签
    /// 一个标签从尾部上寻, 寻到就返回找到的, 没找到就返回原本的
    /// 所以它支持在宏A内部展开的宏B跳转到宏A内部的标记
    const_expand_tag_name_map: Vec<HashMap<Var, Var>>,
    const_expand_names: Vec<Var>,
    const_expand_max_depth: usize,
    value_binds: HashMap<(Var, Var), Var>,
    /// 值绑定全局常量表, 只有值绑定在使用它
    value_bind_global_consts: HashMap<Var, ConstData>,
    last_builtin_exit_code: u8,
    enable_misses_match_log_info: bool,
    noop_line: String,
}
impl Debug for CompileMeta {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // 为适应GitHub Rust Actions的老版本Rust, 暂时将Debug宏手动实现
        struct DotDot;
        impl Debug for DotDot {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "..")
            }
        }
        f.debug_struct("CompileMeta")
            .field("tags_map", &self.tags_map)
            .field("tag_count", &self.tag_count)
            .field("tag_codes", &self.tag_codes)
            .field("tmp_var_count", &self.tmp_var_count.counter())
            .field("expand_env", &self.expand_env)
            .field("env_args", &self.env_args)
            .field("dexp_result_handles", &self.dexp_result_handles)
            .field("dexp_expand_binders", &self.dexp_expand_binders)
            .field("tmp_tag_count", &self.tmp_tag_count.counter())
            .field("const_expand_tag_name_map", &self.const_expand_tag_name_map)
            .field("value_binds", &self.value_binds)
            .field("value_bind_global_consts", &self.value_bind_global_consts)
            .field("last_builtin_exit_code", &self.last_builtin_exit_code)
            .field("enable_misses_match_log_info", &self.enable_misses_match_log_info)
            .field("..", &DotDot)
            .finish()
    }
}
impl Default for CompileMeta {
    fn default() -> Self {
        Self::with_tag_codes(TagCodes::new())
    }
}
impl CompileMeta {
    const BUILTIN_FUNCS_BINDER: &'static str = "Builtin";

    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_tag_codes(tag_codes: TagCodes) -> Self {
        let mut meta = Self {
            tags_map: HashMap::new(),
            tag_count: 0,
            tag_codes,
            tmp_var_count: Counter::new(Self::tmp_var_getter),
            expand_env: Vec::new(),
            env_args: Vec::new(),
            dexp_result_handles: Vec::new(),
            dexp_expand_binders: Vec::new(),
            tmp_tag_count: Counter::new(Self::tmp_tag_getter),
            const_expand_tag_name_map: Vec::new(),
            const_expand_names: Vec::new(),
            const_expand_max_depth: 500,
            value_binds: HashMap::new(),
            value_bind_global_consts: HashMap::new(),
            last_builtin_exit_code: 0,
            enable_misses_match_log_info: false,
            noop_line: "noop".into(),
        };
        let builtin = String::from(Self::BUILTIN_FUNCS_BINDER);
        for builtin_func in build_builtins() {
            let handle = format!("__{builtin}__{}", builtin_func.name());
            let key = (builtin.clone().into(), builtin_func.name().into());
            meta.value_binds.insert(key, handle.into());
            meta.add_const_value(Const(
                ConstKey::ValueBind(ValueBind(
                    Box::new(builtin.clone().into()),
                    builtin_func.name().into()
                )),
                builtin_func.into(),
                vec![],
            ));
        }
        meta
    }

    /// 获取一个标记的编号, 如果不存在则将其插入并返回新分配的编号.
    /// 注: `Tag`与`Label`是混用的, 表示同一个意思
    pub fn get_tag(&mut self, label: Var) -> usize {
        *self.tags_map.entry(label.to_string()).or_insert_with(|| {
            let id = self.tag_count;
            self.tag_count += 1;
            id
        })
    }

    fn tmp_tag_getter(id: &mut usize) -> Var {
        let old = *id;
        *id += 1;
        gen_anon_name(old, |arg| format!("__{arg}")).into()
    }

    /// 获取一个临时的`tag`
    pub fn get_tmp_tag(&mut self) -> Var {
        self.tmp_tag_count.get()
    }

    fn tmp_var_getter(id: &mut usize) -> Var {
        let old = *id;
        *id += 1;
        gen_anon_name(old, |arg| format!("__{arg}")).into()
    }

    pub fn get_tmp_var(&mut self) -> Var {
        self.tmp_var_count.get()
    }

    /// 获取绑定值, 如果绑定关系不存在则自动插入
    pub fn get_value_binded(&mut self, value: Var, bind: Var) -> Var {
        let mut warn_builtin = (value == Self::BUILTIN_FUNCS_BINDER)
            .then_some((false, bind.clone()));
        let key = (value, bind);
        let binded = self.value_binds.entry(key)
            .or_insert_with(|| {
                if let Some((ref mut warn, _)) = warn_builtin {
                    *warn = true;
                }
                self.tmp_var_count.get()
            }).clone();
        if let Some((true, bind)) = warn_builtin {
            self.log_info(format_args!("Missed Builtin Call: {bind}"));
        }
        binded
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

    /// 进入一个拥有子命名空间的子块
    /// 返回该子块结束后的命名空间
    pub fn with_block(&mut self,
        f: impl FnOnce(&mut Self),
    ) -> HashMap<Var, ConstData> {
        /// 进入一个子块, 创建一个新的子命名空间
        fn block_enter(this: &mut CompileMeta) {
            this.expand_env.push(ExpandEnv::default())
        }
        /// 退出一个子块, 弹出最顶层命名空间
        /// 如果无物可弹说明逻辑出现了问题, 所以内部处理为unwrap
        /// 一个enter对应一个exit
        fn block_exit(this: &mut CompileMeta) -> HashMap<Var, ConstData> {
            // this is poped block
            let ExpandEnv {
                leak_vars: leaks,
                consts: mut res,
                ..
            } = this.expand_env.pop().unwrap();

            // do leak
            for leak_const_name in leaks {
                let value
                    = res.remove(&leak_const_name).unwrap();

                // insert to prev block
                this.expand_env
                    .last_mut()
                    .expect("常量泄露击穿")
                    .consts
                    .insert(leak_const_name, value);
            }
            res
        }

        block_enter(self);
        f(self);
        block_exit(self)
    }

    /// 添加一个需泄露的const
    pub fn add_const_value_leak(&mut self, name: Var) {
        self.expand_env
            .last_mut()
            .unwrap()
            .leak_vars
            .push(name)
    }

    /// 获取一个常量到值的使用次数与映射与其内部标记的引用,
    /// 从当前作用域往顶层作用域一层层找, 都没找到就返回空
    pub fn get_const_value(&self, name: &Var) -> Option<&ConstData> {
        self.expand_env
            .iter()
            .rev()
            .find_map(|env| {
                env.consts().get(name)
            })
            .map(|x| { assert!(! x.value().is_repr_var()); x })
            .or_else(|| self.value_bind_global_consts.get(name))
    }

    /// 获取一个常量到值的使用次数与映射与其内部标记的可变引用,
    /// 从当前作用域往顶层作用域一层层找, 都没找到就返回空
    pub fn get_const_value_mut(&mut self, name: &Var)
    -> Option<&mut ConstData> {
        self.expand_env
            .iter_mut()
            .rev()
            .find_map(|env| {
                env.consts_mut().get_mut(name)
            })
            .map(|x| { assert!(! x.value().is_repr_var()); x })
            .or_else(|| self.value_bind_global_consts.get_mut(name))
    }

    /// 新增一个常量到值的映射, 如果当前作用域已有此映射则返回旧的值并插入新值
    pub fn add_const_value(&mut self, r#const: Const)
    -> Option<ConstData> {
        self.add_const_value_with_extra_binder(r#const, None)
    }

    /// 新增一个常量到值的映射, 如果当前作用域已有此映射则返回旧的值并插入新值
    ///
    /// 如果扩展绑定者存在的话, 忽略键中的绑定者而采用扩展绑定者
    pub fn add_const_value_with_extra_binder(
        &mut self,
        Const(var, value, labels): Const,
        extra_binder: Option<Var>,
    ) -> Option<ConstData> {
        match var {
            ConstKey::Var(_) => {
                let var = var.take_handle(self);

                let mut data = ConstData::new(value, labels);
                if let Some(extra_binder) = extra_binder {
                    data = data.set_binder(extra_binder)
                }
                self.expand_env
                    .last_mut()
                    .unwrap()
                    .consts
                    .insert(var, data)
            },
            ConstKey::ValueBind(ValueBind(binder, name)) => {
                let binder_handle = binder.take_handle(self);
                let data = ConstData::new(value, labels)
                    .set_binder(extra_binder
                        .unwrap_or_else(|| binder_handle.clone()));
                let binded = self.get_value_binded(
                    binder_handle, name);
                self.value_bind_global_consts
                    .insert(binded, data)
            },
        }
    }

    /// 从当前作用域移除一个常量到值的映射
    pub fn remove_const_value<Q>(&mut self, query: Q) -> Option<ConstData>
    where Var: Borrow<Q> + Eq + Hash,
          Q: Hash + Eq,
    {
        self.expand_env
            .last_mut()
            .unwrap()
            .consts
            .remove(&query)
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

    /// 尝试获取当前DExp返回句柄, 没有DExp的话返回空
    pub fn try_get_dexp_handle(&self) -> Option<&Var> {
        self.dexp_result_handles.last()
    }

    /// 获取当前DExp返回句柄
    pub fn dexp_handle(&self) -> &Var {
        self.try_get_dexp_handle()
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
        let label_count = self.get_const_value(name)?.labels().len();
        if self.const_expand_names.len() >= self.const_expand_max_depth {
            self.log_err(format!(
                    "Stack Expand:\n{}",
                    self.debug_expand_stack()
                        .flat_map(|var| [
                            var.to_string().into(),
                            Cow::Borrowed("\n"),
                        ]).into_iter_fmtter(),
            ));
            err!(
                "Maximum recursion depth exceeded ({})",
                self.const_expand_max_depth,
            );
            exit(6)
        }
        let mut tmp_tags = Vec::with_capacity(label_count);
        tmp_tags.extend(repeat_with(|| self.get_tmp_tag())
                        .take(label_count));

        let ConstData { value, labels, binder }
            = self.get_const_value(name).unwrap();
        let mut labels_map = HashMap::with_capacity(labels.len());
        for (tmp_tag, label) in zip(tmp_tags, labels.iter().cloned()) {
            labels_map.entry(label).or_insert_with_key(|label| {
                format!(
                    "{}_const_{}_{}",
                    tmp_tag,
                    &name,
                    &label
                ).into()
            });
        }
        let res = value.clone();
        self.dexp_expand_binders.push(binder.clone());
        self.const_expand_tag_name_map.push(labels_map);
        self.const_expand_names.push(name.clone());
        res.into()
    }

    pub fn const_expand_exit(&mut self)
    -> (Var, HashMap<Var, Var>, Option<Var>) {
        (
            self.const_expand_names.pop().unwrap(),
            self.const_expand_tag_name_map.pop().unwrap(),
            self.dexp_expand_binders.pop().unwrap(),
        )
    }

    pub fn get_dexp_expand_binder(&self) -> Option<&Var> {
        self.dexp_expand_binders.iter()
            .filter_map(Option::as_ref)
            .next_back()
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

    pub fn const_var_namespace(&self) -> &[ExpandEnv] {
        self.expand_env.as_ref()
    }

    /// 获取最内层args的每个句柄, 如果不存在则返回空切片
    pub fn get_env_args(&self) -> &[Var] {
        self.env_args.iter()
            .filter_map(Option::as_ref)
            .map(Vec::as_slice)
            .next_back()
            .unwrap_or(&[])
    }

    /// 获取次内层args的每个句柄
    pub fn get_env_second_args(&self) -> &[Var] {
        self.env_args.iter()
            .filter_map(Option::as_ref)
            .map(Vec::as_slice)
            .nth_back(1)
            .unwrap_or(&[])
    }

    /// 设置最内层args, 返回旧值
    ///
    /// 这将进行const的追溯
    pub fn set_env_args<I>(&mut self, expand_args: I) -> Option<Vec<Var>>
    where I: IntoIterator,
          I::Item: Into<Value>,
    {
        self.set_raw_env_args(
            expand_args.into_iter()
                .map(|v| (None, v.into())),
            true,
        )
    }

    /// 设置最内层args, 返回旧值
    ///
    /// 给定绑定者和值
    pub fn set_raw_env_args(
        &mut self,
        expand_binder_and_args: impl IntoIterator<
            Item = (Option<Var>, Value),
        >,
        do_follow: bool,
    ) -> Option<Vec<Var>> {
        let vars: Vec<Var> = expand_binder_and_args.into_iter()
            .map(|(mut binder, value)| {
                let var_key = self.get_tmp_var();
                let key = ConstKey::Var(var_key.clone());
                let mut r#const = Const::new(key, value);
                if do_follow {
                    binder = r#const.run_value(self).or(binder);
                }
                self.add_const_value_with_extra_binder(r#const, binder);
                var_key
            })
            .collect();
        let args = self.env_args.last_mut().unwrap();
        replace(args, vars.into())
    }

    /// 设置次内层args, 返回旧值
    pub fn set_env_second_args(&mut self, expand_args: Vec<Value>) -> Option<Vec<Var>> {
        let vars: Vec<Var> = expand_args.into_iter()
            .map(|value| {
                let var_key = self.get_tmp_var();
                self.add_const_value_leak(var_key.clone());
                let key = ConstKey::Var(var_key.clone());
                Const::new(key, value).compile(self);
                var_key
            })
            .collect();
        let args = self.env_args.iter_mut()
            .nth_back(1).expect("尝试设置次内层参数时产生了击穿");
        replace(args, vars.into())
    }

    /// 切片次内层args
    pub fn slice_env_second_args(&mut self, (start, end): (usize, usize)) {
        let args = self.get_env_second_args();
        let len = args.len();
        let range = start.min(len)..end.min(len);
        let args = args[range].iter().cloned().map(Value::Var).collect();
        self.set_env_second_args(args);
    }

    pub fn with_env_args_block<F>(&mut self, f: F) -> Option<Vec<Var>>
    where F: FnOnce(&mut Self)
    {
        self.env_args.push(None);
        let _ = f(self);
        self.env_args.pop().unwrap()
    }

    pub fn log_info(&mut self, s: impl std::fmt::Display) {
        eprintln!("{}", csi!(1; "[I] {}",
                s.to_string().trim_end().replace('\n', "\n    ")))
    }

    pub fn log_err(&mut self, s: impl std::fmt::Display) {
        eprintln!("{}", csi!(1, 91; "[E] {}",
                s.to_string().trim_end().replace('\n', "\n    ")))
    }

    pub fn debug_expand_stack(&self) -> impl Iterator<
        Item = Cow<'_, Var>
    > + '_ {
        self.const_expand_names().iter()
            .zip(&self.dexp_expand_binders)
            .map(|x| match x {
                (name, Some(binder)) => {
                    Cow::Owned(format!("{name} ..{binder}").into())
                },
                (name, None) => Cow::Borrowed(name),
            })
    }

    pub fn log_expand_stack<const E: bool>(&mut self) {
        let names = self.debug_expand_stack()
            .flat_map(|var| [
                var.to_string().into(),
                Cow::Borrowed("\n"),
            ])
            .into_iter_fmtter();
        let s = format!("Expand Stack:\n{names}");
        drop(names);
        if E {
            self.log_err(s);
        } else {
            self.log_info(s);
        }
    }

    pub fn last_builtin_exit_code(&self) -> u8 {
        self.last_builtin_exit_code
    }

    pub fn set_last_builtin_exit_code(&mut self, new_code: u8) -> u8 {
        replace(&mut self.last_builtin_exit_code, new_code)
    }

    pub fn const_expand_names(&self) -> &[Var] {
        self.const_expand_names.as_ref()
    }

    pub fn const_expand_max_depth(&self) -> usize {
        self.const_expand_max_depth
    }

    pub fn set_const_expand_max_depth(&mut self, const_expand_max_depth: usize) {
        self.const_expand_max_depth = const_expand_max_depth;
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
                    Take(child_result.into(), Value::ResultHandle).into(),
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
                    Take(child_result.into(), result).into(),
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
pub fn op_expr_build_results(
    meta: &mut Meta,
    mut results: Vec<Value>,
    mut values: Vec<OpExprInfo>,
) -> LogicLine {
    match (results.len(), values.len()) {
        e @ ((0, _) | (_, 0)) => unreachable!("len by zero, {e:?}"),
        (1, 1) => {
            let (result, value) = (
                results.pop().unwrap(),
                values.pop().unwrap(),
            );
            value.into_logic_line(meta, result)
        },
        (len, 1) => {
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
                    = OpExprInfo::Value(first_result_handle.clone().into());
                lines.push(value.into_logic_line(meta, result.clone()))
            }
            assert_eq!(lines.len(), len + 1);
            Expand(lines).into()
        },
        (res_len, val_len) => {
            assert_eq!(res_len, val_len);

            let mut lines = Vec::with_capacity(res_len);
            let ziped
                = results.into_iter().zip(values);

            for (result, value) in ziped {
                let line = value.into_logic_line(meta, result);
                lines.push(line)
            }
            Expand(lines).into()
        },
    }
}
