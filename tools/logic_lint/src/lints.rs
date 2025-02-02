use core::fmt;
use std::vec;

use lazy_regex::regex_is_match;
use var_utils::{AsVarType, VarType};

use crate::{Line, Source, Var};

macro_rules! color_str {
    ($fnum:literal $($num:literal)* : $str:literal) => {
        concat!(
            "\x1b[",
            stringify!($fnum),
            $(";", stringify!($num), )*
            "m",
            $str,
            "\x1b[22;39m",
        )
    };
}
macro_rules! make_lints {
    {
        $lint_vis:vis fn $lint_name:ident<$lifetime:lifetime>($src:ident, $line:ident) -> $res_ty:ty;
        let $lints:ident;
        $(
            $(|)? $($prefix:literal)|+ ($($argc:literal),+) $body:block
        )*
    } => {
        $lint_vis fn $lint_name<$lifetime>(
            $src: &$lifetime $crate::Source<$lifetime>,
            $line: &$lifetime $crate::Line<$lifetime>,
        ) -> Vec<$res_ty> {
            let mut $lints = Vec::<$res_ty>::new();
            match $line.args() {
                $(
                    [$crate::Var { value: $($prefix)|+, .. }, ..] => {
                        $lints.extend(check_argc($src, $line, &[$($argc),+]));
                        $body
                    },
                )*
                [] => unreachable!(),
                [cmd, args @ ..] => {
                    $lints.extend(check_cmd($src, $line, cmd, args))
                },
            }
            $lints
        }
    };
}

#[derive(Debug, Clone, Copy)]
pub enum VarUsedMethod {
    /// 匹配将要向其写入
    Assign,
    /// 匹配将要读取
    Read,
}
impl VarUsedMethod {
    /// Returns `true` if the var used method is [`Assign`].
    ///
    /// [`Assign`]: VarUsedMethod::Assign
    #[must_use]
    pub fn is_assign(&self) -> bool {
        matches!(self, Self::Assign)
    }

    /// Returns `true` if the var used method is [`Read`].
    ///
    /// [`Read`]: VarUsedMethod::Read
    #[must_use]
    pub fn is_read(&self) -> bool {
        matches!(self, Self::Read)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct VarUsed<'a> {
    method: VarUsedMethod,
    var: Var<'a>,
}
impl<'a> VarUsed<'a> {
    pub fn method(&self) -> VarUsedMethod {
        self.method
    }

    pub fn var(&self) -> &Var<'a> {
        &self.var
    }

    pub fn as_read(&self) -> Option<&Var<'a>> {
        if self.method.is_read() {
            Some(&self.var)
        } else {
            None
        }
    }

    pub fn as_assign(&self) -> Option<&Var<'a>> {
        if self.method.is_assign() {
            Some(&self.var)
        } else {
            None
        }
    }
}

enum VarPat {
    Catch(VarUsedMethod),
    Lit(&'static str),
    Any,
}
impl VarPat {
    pub fn pat<'a>(&self, var: Var<'a>) -> Result<Option<VarUsed<'a>>, ()> {
        match *self {
            VarPat::Catch(method) => Ok(VarUsed {
                method,
                var,
            }.into()),
            VarPat::Any => Ok(None),
            VarPat::Lit(s) if s == var.value() => Ok(None),
            VarPat::Lit(_) => Err(()),
        }
    }
}

thread_local! {
    static LINE_PAT: Vec<Vec<VarPat>> = {
        macro_rules! pat {
            (_) => {
                VarPat::Any
            };
            ($lit:literal) => {
                VarPat::Lit($lit)
            };
            (a) => {
                VarPat::Catch(VarUsedMethod::Assign)
            };
            (v) => {
                VarPat::Catch(VarUsedMethod::Read)
            };
        }
        macro_rules! make_pats {
            {
                $([
                    $($t:tt)*
                ])*
            } => {
                vec![
                    $(
                        vec![$(pat!($t)),*]
                    ),*
                ]
            };
        }
        make_pats! {
            ["read" a v v]
            ["write" v v v]
            ["draw" "clear" v v v]
            ["draw" "color" v v v v]
            ["draw" "col" v]
            ["draw" "stroke" v]
            ["draw" "line" v v v v]
            ["draw" "rect" v v v v]
            ["draw" "lineRect" v v v v]
            ["draw" "poly" v v v v v]
            ["draw" "linePoly" v v v v v]
            ["draw" "triangle" v v v v v v]
            ["draw" "image" v v v v v]
            ["draw" "print" v v _]
            ["draw" "translate" v v]
            ["draw" "scale" v v]
            ["draw" "rotate" _ _ v]
            ["draw" "reset"]
            ["print" v]
            ["format" v]
            ["drawflush" v]
            ["printflush" v]
            ["getlink" a v]
            ["control" "shoot" v v v v]
            ["control" "shootp" v v v]
            ["control" _ v v]
            ["radar" _ _ _ _ v v a]
            ["sensor" a v v]
            ["set" a v]
            ["op" _ a v v]
            ["lookup" _ a v]
            ["packcolor" a v v v v]
            ["wait" v]
            ["stop"]
            ["end"]
            ["jump" _ _ v v]
            ["ubind" v]
            ["ucontrol" "within" v v v a]
            ["ucontrol" _        v v v v v]
            ["uradar" _ _ _ _ _ v a]
            ["ulocate" "ore"        _ _ v a a a]
            ["ulocate" "building"   _ v _ a a a a]
            ["ulocate" "spawn"      _ _ _ a a a a]
            ["ulocate" "damaged"    _ _ _ a a a a]
            // world
            ["getblock" _ a v v]
            ["setblock" _ v v v v v]
            ["spawn" v v v v v a]
            ["status" "true" _ v]
            ["status" _ _ v v]
            ["weathersense" a v]
            ["weatherset" v v]
            ["spawnwave" v v v]
            ["setrule" _ v v v v v]
            ["message" _ v v]
            ["cutscene" _ v v v v]
            ["effect" _ v v v v v]
            ["explosion" v v v v v v v v v]
            ["setrate" v]
            ["fetch" _ a v v v]
            ["sync" v]
            ["getflag" a v]
            ["setflag" v v]
            ["setprop" v v v]
            ["playsound" "true"  v v v _ v v v]
            ["playsound" "false" v v v v _ _ v]
            ["playsound" _       v v v v v v v]
            ["setmarker" _ v v v v]
            ["makemarker" _ v v v v]
            ["localeprint" v]
            // 兜底, 对未录入的语句参数统一为使用
            [_ v v v v v v v v v v v v v v v v v v]
        }
    };
}
pub fn get_useds<'a>(line: &Line<'a>) -> Option<Vec<VarUsed<'a>>> {
    LINE_PAT.with(|pats| {
        pats.iter()
            .find_map(|pat| {
                let mut useds = vec![];
                for (pat, var) in pat.iter().zip(line.args()) {
                    match pat.pat(*var) {
                        Ok(Some(used)) => useds.push(used),
                        Ok(None) => {},
                        Err(()) => return None,
                    }
                }
                useds.into()
            })
    })
}

fn vec_optiter<T>(value: Option<Vec<T>>) -> vec::IntoIter<T> {
    match value {
        Some(x) => x.into_iter(),
        None => Vec::new().into_iter(),
    }
}
#[must_use]
fn check_assign_var<'a>(
    src: &'a crate::Source<'a>,
    line: &'a crate::Line<'a>,
    var: &'a Var<'a>,
) -> impl IntoIterator<Item = Lint<'a>> + 'a {
    match var.as_var_type() {
        VarType::String(_) | VarType::Number(_) => {
            vec_optiter(vec![Lint::new(var, WarningLint::AssignLiteral)].into())
        },
        VarType::Var(_) => {
            let mut lints = Vec::new();
            lints.extend(check_var(src, line, var));
            if !src.used_vars().contains(var.value())
                && !regex_is_match!(r"^_(?:$|[^_])", var.value())
            {
                lints.push(Lint::new(var, WarningLint::NeverUsed));
            }
            vec_optiter(lints.into())
        },
    }
}
#[must_use]
fn check_var<'a>(
    _src: &'a crate::Source<'a>,
    _line: &'a crate::Line<'a>,
    var: &'a Var<'a>,
) -> Option<Lint<'a>> {
    match var.value() {
        "__" => Lint::new(
            var,
            WarningLint::UsedDoubleUnderline).into(),
        s if regex_is_match!(r"^_\d+$", s) => Lint::new(
            var,
            WarningLint::UsedRawArgs).into(),
        s if !s.is_empty() && s.chars().next().unwrap().is_uppercase() => {
            Lint::new(
                var,
                WarningLint::SuspectedConstant
            ).into()
        },
        _ => None,
    }
}
#[must_use]
fn check_vars<'a>(
    src: &'a crate::Source<'a>,
    line: &'a crate::Line<'a>,
    vars: impl IntoIterator<Item = &'a Var<'a>> + 'a,
) -> impl Iterator<Item = Lint<'a>> + 'a {
    vars.into_iter()
        .filter_map(|var| check_var(src, line, var))
}
fn check_cmd<'a>(
    src: &'a crate::Source<'a>,
    line: &'a crate::Line<'a>,
    var: &'a Var<'a>,
    args: &'a [Var<'a>],
) -> impl Iterator<Item = Lint<'a>> + 'a {
    (regex_is_match!(r"^__", var) && !regex_is_match!(r".__$", var))
        .then(|| Lint::new(var, WarningLint::SuspectedVarCmd))
        .into_iter()
        .chain(check_vars(src, line, args))
}
#[must_use]
#[track_caller]
fn check_argc<'a>(
    _src: &'a crate::Source<'a>,
    line: &'a crate::Line<'a>,
    expected: &[usize],
) -> Option<Lint<'a>> {
    assert_ne!(expected.len(), 0);
    let len = line.args().len() - 1;
    if expected.contains(&len) {
        return None;
    }
    Lint::new(
        line.args().first().unwrap(),
        WarningLint::ArgsCountNotMatch {
            expected: expected[0],
            found: len,
        }
    ).into()
}
#[must_use]
fn check_oper<'a>(
    oper: &'a Var<'a>,
    expected: &'static [&'static str],
) -> Option<Lint<'a>> {
    if expected.contains(&oper.value()) { return None; }
    Lint::new(oper, ErrorLint::InvalidOper { expected }).into()
}

const OP_METHODS: &[&str] = &[
    "add", "sub", "mul", "div", "idiv", "mod",
    "pow", "equal", "notEqual", "land", "lessThan", "lessThanEq",
    "greaterThan", "greaterThanEq", "strictEqual", "shl", "shr", "or",
    "and", "xor", "not", "max", "min", "angle",
    "angleDiff", "len", "noise", "abs", "log", "log10",
    "floor", "ceil", "sqrt", "rand", "sin", "cos",
    "tan", "asin", "acos", "atan",
];
const JUMP_METHODS: &[&str] = &[
    "equal", "notEqual", "lessThan", "lessThanEq",
    "greaterThan", "greaterThanEq", "strictEqual",
    "always",
];
const UNIT_CONTROL_METHODS: &[&str] = &[
    "idle", "stop", "move", "approach", "pathfind",
    "autoPathfind", "boost", "target", "targetp", "itemDrop",
    "itemTake", "payDrop", "payTake", "payEnter", "mine",
    "flag", "build", "getBlock", "within", "unbind",
];
const FETCH_METHODS: &[&str] = &[
    "unit",     "unitCount",
    "player",   "playerCount",
    "core",     "coreCount",
    "build",    "buildCount",
];

make_lints! {
    pub fn lint<'a>(src, line) -> Lint<'a>;
    let lints;
    "set" | "getlink" (2) {
        if let [_, result, ..] = line.args() {
            lints.extend(check_assign_var(src, line, result))
        }
        if let [_, _, var, ..] = line.args() {
            lints.extend(check_vars(src, line, [var]))
        }
    }
    "op" (4) {
        if let [_, oper, ..] = line.args() {
            lints.extend(check_oper(oper, OP_METHODS));
        }
        if let [_, _, result, ..] = line.args() {
            lints.extend(check_assign_var(src, line, result))
        }
        if let [_, _, _, var, var1, ..] = line.args() {
            lints.extend(check_vars(src, line, [var, var1]))
        }
    }
    "lookup" (3) {
        if let [_, mode, result, index, ..] = line.args() {
            lints.extend(check_oper(mode, &["block", "unit", "item", "liquid"]));
            lints.extend(check_assign_var(src, line, result));
            lints.extend(check_vars(src, line, [index]));
        }
    }
    "ucontrol" (6) {
        if let [_, oper, args @ ..] = line.args() {
            lints.extend(check_oper(oper, UNIT_CONTROL_METHODS));
            lints.extend(check_vars(src, line, args));
        }
    }
    "ulocate" (8) {
        if let [_, mode, btype, args @ ..] = line.args() {
            // 考虑到经常需要不用这里的参数, 所以不使用assign
            lints.extend(check_oper(mode, &[
                "building", "ore", "spawn", "damaged",
            ]));
            lints.extend(check_oper(btype, &[
                "core", "storage", "generator", "turret", "factory",
                "repair", "rally", "battery", "reactor",
            ]));
            lints.extend(check_vars(src, line, args))
        }
    }
    "end" | "stop" (0) {}
    "print" | "format" | "printflush" | "drawflush" | "wait" | "ubind" (1) {
        if let [_, var, ..] = line.args() {
            lints.extend(check_vars(src, line, [var]))
        }
    }
    "packcolor" (5) {
        if let [_, result, args @ ..] = line.args() {
            lints.extend(check_assign_var(src, line, result));
            lints.extend(check_vars(src, line, args));
        }
    }
    "control" (6) {
        if let [_, mode, args @ ..] = line.args() {
            lints.extend(check_oper(mode, &[
                "enabled", "shoot", "shootp", "config", "color",
            ]));
            lints.extend(check_vars(src, line, args));
        }
    }
    "read" | "sensor" (3) {
        if let [_, result, args @ ..] = line.args() {
            lints.extend(check_assign_var(src, line, result));
            lints.extend(check_vars(src, line, args));
        }
    }
    "draw" (7) {
        if let [_, mode, args @ ..] = line.args() {
            lints.extend(check_oper(mode, &[
               "clear", "color", "col", "stroke", "line", "rect",
               "lineRect", "poly", "linePoly", "triangle", "image",
               "print", "translate", "scale", "rotate", "reset",
            ]));
            lints.extend(check_vars(src, line, args));
        }
    }
    "write" (3) {
        if let [_, args @ ..] = line.args() {
            lints.extend(check_vars(src, line, args));
        }
    }
    "radar" | "uradar" (7) {
        fn check_filter<'a>(arg: &'a Var<'a>) -> Option<Lint<'a>> {
            check_oper(arg, &[
               "any", "enemy", "ally", "player", "attacker",
               "flying", "boss", "ground",
            ])
        }
        fn check_order<'a>(arg: &'a Var<'a>) -> Option<Lint<'a>> {
            check_oper(arg, &[
               "distance", "health", "shield",
               "armor", "maxHealth",
            ])
        }
        if let [_, filt1, filt2, filt3, order, from, rev, result]
        = line.args() {
            lints.extend(check_filter(filt1));
            lints.extend(check_filter(filt2));
            lints.extend(check_filter(filt3));
            lints.extend(check_order(order));
            lints.extend(check_vars(src, line, [from, rev]));
            lints.extend(check_assign_var(src, line, result));
        }
    }
    "jump" (4) {
        if let [_, target, method, a, b]
        = line.args() {
            if target.value() == "-1" {
                lints.push(Lint::new(target, WarningLint::NoTargetJump));
            }
            lints.extend(check_oper(method, JUMP_METHODS));
            lints.extend(check_vars(src, line, [a, b]));
        }
    }
    // world
    "getblock" (4) {
        if let [_, method, result, x, y]
        = line.args() {
            lints.extend(check_oper(method, &[
                    "floor", "ore", "block", "building"]));
            lints.extend(check_assign_var(src, line, result));
            lints.extend(check_vars(src, line, [x, y]));
        }
    }
    "setblock" (6) {
        if let [_, method, args @ ..] = line.args() {
            lints.extend(check_oper(method, &["floor", "ore", "block"]));
            lints.extend(check_vars(src, line, args));
        }
    }
    "spawn" (6) {
        if let [_, args @ .., result] = line.args() {
            lints.extend(check_assign_var(src, line, result));
            lints.extend(check_vars(src, line, args));
        }
    }
    "status" (4) {
        if let [_, method, _status, args @ ..] = line.args() {
            lints.extend(check_oper(method, &["true", "false"]));
            lints.extend(check_vars(src, line, args));
        }
    }
    "weathersense" (2) {
        if let [_, result, args @ ..] = line.args() {
            lints.extend(check_assign_var(src, line, result));
            lints.extend(check_vars(src, line, args));
        }
    }
    "weatherset" (2) {
        if let [_, args @ ..] = line.args() {
            lints.extend(check_vars(src, line, args));
        }
    }
    "spawnwave" (3) {
        if let [_, args @ ..] = line.args() {
            lints.extend(check_vars(src, line, args));
        }
    }
    "setrule" (6) {
        if let [_, _method, args @ ..] = line.args() {
            lints.extend(check_vars(src, line, args));
        }
    }
    "message" (3) {
        if let [_, method, args @ ..] = line.args() {
            lints.extend(check_oper(method, &[
                    "notify", "announce", "toast", "mission"]));
            lints.extend(check_vars(src, line, args));
        }
    }
    "cutscene" (5) {
        if let [_, method, args @ ..] = line.args() {
            lints.extend(check_oper(method, &["pan", "zoom", "stop"]));
            lints.extend(check_vars(src, line, args));
        }
    }
    "effect" (5, 6) {
        if let [_, _method, args @ ..] = line.args() {
            lints.extend(check_vars(src, line, args));
        }
    }
    "explosion" (9) {
        if let [_, args @ ..] = line.args() {
            lints.extend(check_vars(src, line, args));
        }
    }
    "setrate" (1) {
        if let [_, args @ ..] = line.args() {
            lints.extend(check_vars(src, line, args));
        }
    }
    "fetch" (5) {
        if let [_, method, result, a, b, c] = line.args() {
            lints.extend(check_oper(method, FETCH_METHODS));
            lints.extend(check_assign_var(src, line, result));
            lints.extend(check_vars(src, line, [a, b, c]));
        }
    }
    "sync" (1) {
        if let [_, args @ ..] = line.args() {
            lints.extend(check_vars(src, line, args));
        }
    }
    "getflag" (2) {
        if let [_, result, args @ ..] = line.args() {
            lints.extend(check_assign_var(src, line, result));
            lints.extend(check_vars(src, line, args));
        }
    }
    "setflag" (2) {
        if let [_, args @ ..] = line.args() {
            lints.extend(check_vars(src, line, args));
        }
    }
    "setprop" (3) {
        if let [_, args @ ..] = line.args() {
            lints.extend(check_vars(src, line, args));
        }
    }
    "playsound" (8) {
        if let [_, method, args @ ..] = line.args() {
            lints.extend(check_oper(method, &["true", "false"]));
            lints.extend(check_vars(src, line, args));
        }
    }
    "setmarker" (5) {
        if let [_, _method, args @ ..] = line.args() {
            lints.extend(check_vars(src, line, args));
        }
    }
    "makemarker" (5) {
        if let [_, _method, args @ ..] = line.args() {
            lints.extend(check_vars(src, line, args));
        }
    }
    "localeprint" (1) {
        if let [_, args @ ..] = line.args() {
            lints.extend(check_vars(src, line, args));
        }
    }
}

pub trait ShowLint {
    fn show_lint(
        &self,
        src: &Source<'_>,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result;
}

#[derive(Debug, PartialEq, Eq)]
pub struct Lint<'a> {
    arg: &'a Var<'a>,
    msg: LintType,
}
impl<'a> Lint<'a> {
    pub fn new(arg: &'a Var<'a>, msg: impl Into<LintType>) -> Self {
        Self { arg, msg: msg.into() }
    }
}
impl ShowLint for Lint<'_> {
    fn show_lint(
        &self,
        src: &Source<'_>,
        f: &mut fmt::Formatter<'_>,
    ) -> Result<(), std::fmt::Error> {
        let lineno = self.arg.lineno();
        let arg_idx = self.arg.arg_idx();
        write!(
            f, concat!("{} ", color_str!(33: "[{}@{}]"), ": "),
            self.msg.lint_type(),
            lineno,
            arg_idx,
        )?;
        self.msg.show_lint(src, f)?;
        writeln!(f)?;

        let (prelines, suflines)
            = src.view_lines(lineno, (2, 2));

        macro_rules! show_lines {
            ($lines:expr) => {{
                for line in $lines {
                    writeln!(f, "    {}", line.hint_args(&[]).join(" "))?;
                }
            }};
        }

        let args = src.lines()[lineno].hint_args(&[arg_idx]);
        show_lines!(prelines);
        writeln!(f, concat!(color_str!(1 92: "==>"), " {}"), args.join(" "))?;
        show_lines!(suflines);
        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum LintType {
    Warning(WarningLint),
    Error(ErrorLint),
}
impl LintType {
    pub fn lint_type(&self) -> &'static str {
        match self {
            LintType::Warning(_) => color_str!(1 93: "Warning"),
            LintType::Error(_) => color_str!(1 91: "Error"),
        }
    }
}
impl From<WarningLint> for LintType {
    fn from(value: WarningLint) -> Self {
        Self::Warning(value)
    }
}
impl From<ErrorLint> for LintType {
    fn from(value: ErrorLint) -> Self {
        Self::Error(value)
    }
}
impl ShowLint for LintType {
    fn show_lint(
        &self,
        src: &Source<'_>,
        f: &mut fmt::Formatter<'_>,
    ) -> Result<(), std::fmt::Error> {
        match self {
            LintType::Warning(warn) => warn.show_lint(src, f),
            LintType::Error(err) => err.show_lint(src, f),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum WarningLint {
    /// 显式使用双下划线名称
    UsedDoubleUnderline,
    /// 直接使用未被替换的参数调用协定, 如`_0`
    UsedRawArgs,
    /// 参数数量不匹配
    ArgsCountNotMatch {
        expected: usize,
        found: usize,
    },
    /// 向字面量赋值
    AssignLiteral,
    /// 从命名来看疑似是未被替换的常量
    SuspectedConstant,
    /// 从命名来看疑似将变量作为命令执行
    SuspectedVarCmd,
    /// 未被使用
    NeverUsed,
    NoTargetJump,
}
impl ShowLint for WarningLint {
    fn show_lint(
        &self,
        _src: &Source<'_>,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        match self {
            WarningLint::UsedDoubleUnderline => write!(f, "使用了双下划线")?,
            WarningLint::UsedRawArgs => write!(f, "使用了参数协定的原始格式")?,
            WarningLint::ArgsCountNotMatch { expected, found } => {
                write!(
                    f,
                    "不合预期的参数个数, 期待{expected}个, 得到{found}个")?
            },
            WarningLint::AssignLiteral => write!(f, "对字面量进行操作")?,
            WarningLint::SuspectedConstant => {
                write!(f, "命名疑似未被替换的常量")?
            },
            WarningLint::SuspectedVarCmd => {
                write!(f, "命令疑似将变量作为命令执行")?
            },
            WarningLint::NeverUsed => write!(f, "未被使用到的量")?,
            WarningLint::NoTargetJump => write!(f, "没有目标的跳转")?,
        }
        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ErrorLint {
    InvalidOper {
        expected: &'static [&'static str],
    },
}
impl ShowLint for ErrorLint {
    fn show_lint(
        &self,
        _src: &Source<'_>,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        match self {
            ErrorLint::InvalidOper { expected } => {
                write!(f, "无效的操作符, 预期: [{}]", expected.join(" "))?
            },
        }
        Ok(())
    }
}
