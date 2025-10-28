use std::process;

use var_utils::escape_doublequote;

use crate::*;

#[derive(Clone)]
pub struct BuiltinFunc {
    name: &'static str,
    func: fn(&Self, &mut CompileMeta) -> Var,
}
impl Debug for BuiltinFunc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        struct DotDot;
        impl Debug for DotDot {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>)
            -> std::fmt::Result {
                write!(f, "..")
            }
        }
        f.debug_struct(stringify!(BuiltinFunc))
            .field("name", &self.name)
            .field("func", &DotDot)
            .finish()
    }
}
impl PartialEq for BuiltinFunc {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.func as usize == other.func as usize
    }
}
impl BuiltinFunc {
    pub fn name(&self) -> &str {
        self.name
    }

    pub fn func(&self) -> fn(&Self, &mut CompileMeta) -> Var {
        self.func
    }

    pub fn call(&self, meta: &mut CompileMeta) -> Var {
        self.func()(self, meta)
    }
}

fn num(s: &str) -> Option<usize> {
    let &n = s.as_var_type().as_number()?;
    let int = n.trunc();

    if int != n {
        return None;
    }
    if n.abs() > 3.0 && (n + 2.0).abs().log2() > f64::MANTISSA_DIGITS as f64 {
        return None
    }

    Some(int as usize)
}

macro_rules! mutil {
    (@ignore($($i:tt)*) $($t:tt)*) => {
        $($t)*
    };
    (@if($($t:tt)*) $($f:tt)*) => {
        $($t)*
    };
    (@if $($f:tt)*) => {
        $($f)*
    };
}

macro_rules! build_builtin_funcs {
    {
        $(
            $(#[$attr:meta])*
            fn $func_name:ident : $vfunc_name:ident
            ($meta:ident) $([$($var:ident $(: $taked_var:ident)?)*])?
            {$($t:tt)*}
        )*
    } => {
        $(
            $(#[$attr])*
            fn $func_name(this: &BuiltinFunc, $meta: &mut CompileMeta) -> Var
            {
                fn f($meta: &mut CompileMeta)
                -> Result<Var, (u8, String)> {
                    $(
                        let args = $meta.get_env_args();
                        let [$($var),*] = args else {
                            return Err((2, format!(
                                "args count error: argc := {}",
                                args.len()
                            )));
                        };
                        $($(
                            let $taked_var = $meta.get_const_value($var)
                                .unwrap();
                        )?)*
                    )?
                    $($t)*
                }
                match f($meta) {
                    Ok(var) => {
                        $meta.set_last_builtin_exit_code(0);
                        var
                    },
                    Err((code, e)) => {
                        $meta.log_err(format_args!(
                            "\
                            Builtin Function Error:\n\
                            name: {}, argc: {}\n\
                            exit_code: {code}\n\
                            msg: {}\
                            ",
                            this.name(),
                            0$($(+mutil!(@ignore($var) 1))*)?,
                            e,
                        ));
                        $meta.set_last_builtin_exit_code(code);
                        "__".into()
                    },
                }
            }
        )*
        vec![$(
            BuiltinFunc {
                name: stringify!($vfunc_name),
                func: $func_name,
            }
        ),*]
    };
}

pub fn build_builtins() -> Vec<BuiltinFunc> {
    fn value_type(value: impl Borrow<Value>) -> &'static str {
        match value.borrow() {
            Value::Var(_) => "var",
            Value::DExp(_) => "dexp",
            Value::ReprVar(_) => "reprvar",
            Value::ResultHandle => "resulthandle",
            Value::ValueBind(_) => "valuebind",
            Value::ValueBindRef(_) => "valuebindref",
            Value::Cmper(_) => "cmper",
            Value::Binder => "binder",
            Value::BuiltinFunc(_) => "builtinfunc",
            Value::ClosuredValue(_) => "closuredvalue",
        }
    }
    macro_rules! check_type {
        ($type:literal $pat:pat = $value:expr => $body:expr) => {{
            let value = $value;
            if let $pat = value {
                $body
            } else {
                return Err((1, format!(
                    "value type error:\n\
                    expected: {}\n\
                    found: {}\
                    ",
                    $type,
                    value_type(value),
                )))
            }
        }};
    }
    build_builtin_funcs! {
        fn r#type:Type(meta) [var:data] {
            Ok(value_type(data.value()).into())
        }

        fn stringify:Stringify(meta) [var:data] {
            check_type!("var" Value::Var(var) = data.value() => {
                Ok(if Value::is_string(var) {
                    var.into()
                } else {
                    format!("\"{var}\"").into()
                })
            })
        }

        fn status:Status(meta) [] {
            Ok(meta.last_builtin_exit_code().to_string().into())
        }

        fn concat:Concat(meta) [va:a vb:b] {
            check_type!("var" Value::Var(a) = a.value() => {
                check_type!("var" Value::Var(b) = b.value() => {
                    if !Value::is_string(a) {
                        return Err((2, format!("{a} is not a string")));
                    }
                    if !Value::is_string(b) {
                        return Err((2, format!("{b} is not a string")));
                    }
                    Ok([&a[..a.len()-1], &b[1..]].concat().into())
                })
            })
        }

        fn info:Info(meta) [var:data] {
            let value = data.value();
            check_type!("var" Value::Var(var) = value => {
                let var = var.clone();
                meta.log_info(var.clone());
                Ok(var)
            })
        }

        fn err:Err(meta) [var:data] {
            let value = data.value();
            check_type!("var" Value::Var(var) = value => {
                let var = var.clone();
                meta.log_err(var.clone());
                Ok(var)
            })
        }

        fn unbind:Unbind(meta) [var:data] {
            let value = data.value();
            check_type!("valuebind" Value::ValueBind(ValueBind(_, bind)) = value => {
                Ok(bind.into())
            })
        }

        /// 动态名称的const
        ///
        /// 向上层泄露结果, 所以调用时距离目标层必须有且只有一个expand
        /// 否则会击穿或者达不到目标层
        ///
        /// 建议直接使用quick_dexp_take
        fn r#const:Const(meta) [n:name v:value] {
            check_type!("var" Value::Var(name) = name.value() => {
                let name = &name.clone();
                Const(name.into(), value.value().clone(), vec![]).compile(meta);
                meta.add_const_value_leak(name.clone());
                Ok("__".into())
            })
        }

        fn binder:Binder(meta) [n:name v:value] {
            check_type!("var" Value::Var(name) = name.value() => {
                check_type!("valuebind" Value::ValueBind(ValueBind(binder, _)) = value.value() => {
                    let name = &name.clone();
                    Const(name.into(), binder.as_ref().clone(), vec![])
                        .compile(meta);
                    meta.add_const_value_leak(name.clone());
                    Ok("__".into())
                })
            })
        }

        fn bind_handle:BindHandle(meta) [v:value] {
            check_type!("valuebind" Value::ValueBind(bind) = value.value() => {
                let handle = bind.clone().take_unfollow_handle(meta);
                Ok(handle)
            })
        }

        fn bind_handle2:BindHandle2(meta) [b:binder n:name] {
            check_type!("var" Value::Var(name) = name.value() => {
                let bind = ValueBind(
                    binder.value().clone().into(),
                    name.clone(),
                );
                Ok(bind.take_unfollow_handle(meta))
            })
        }

        fn exit:Exit(meta) [n:code] {
            check_type!("var" Value::Var(code) = code.value() => {
                let num_code = match num(code) {
                    Some(code) => code,
                    None => {
                        meta.log_err(format!(
                            "Invalid exit code: {code}",
                        ));
                        128
                    },
                };
                process::exit(num_code.try_into().unwrap_or(124))
            })
        }

        /// 以Debug形式显示一个值
        fn debug:Debug(meta) [v:value] {
            if let Some(ref ext) = meta.extender {
                let mut sep = None;
                meta.log_info(format!(
                    "Value Debug:\n\
                    binder: ..{}\n\
                    labels: [{}]\n\
                    value: {}\
                    ",
                    value.binder.as_ref().map(|s| s.as_str()).unwrap_or(""),
                    value.labels().iter()
                        .flat_map(|v| sep.replace(", ")
                            .into_iter()
                            .chain(once(v.as_str())))
                        .into_iter_fmtter(),
                    ext.display_value(value.value()),
                ));
            } else {
                meta.log_info(format!("Value Debug:\n{:#?}", value));
            }
            Ok("__".into())
        }

        /// 以Debug形式显示一批绑定量
        fn debug_binds:DebugBinds(meta) [v:value] {
            check_type!("var" Value::Var(handle) = value.value() => {
                let msg = meta.with_get_binds(handle.clone(), |displayer| {
                    if let Some(ext) = meta.extender.as_deref() {
                        ext.display_binds(displayer).into_owned()
                    } else {
                        format!("{handle:?}")
                    }
                });
                meta.log_info(format_args!("Debug Binds: {msg}"));
                Ok("__".into())
            })
        }

        fn args_len:ArgsLen(meta) [] {
            Ok(meta.get_env_second_args().len().to_string().into())
        }

        fn slice_args:SliceArgs(meta) [s:start e:end] {
            check_type!("var" Value::Var(start) = start.value() => {
                check_type!("var" Value::Var(end) = end.value() => {
                    let Some(start) = num(start) else {
                        return Err((2, format!("Invalid start value: {start}")))
                    };
                    let Some(end) = num(end) else {
                        return Err((2, format!("Invalid end value: {end}")))
                    };
                    meta.slice_env_second_args((start, end));
                    Ok("__".into())
                })
            })
        }

        fn make_select:MakeSelect(meta) [i] {
            let args = meta.get_env_second_args();
            let lines = args.iter()
                .map(|var| Take(UNNAMED_VAR.into(), var.into()).into())
                .collect();
            Select(i.into(), lines).compile(meta);
            Ok("__".into())
        }

        fn args_handle:ArgsHandle(meta) [i:idx] {
            check_type!("var" Value::Var(idx) = idx.value() => {
                let Some(idx) = num(idx) else {
                    return Err((2, format!("Invalid index value: {idx}")))
                };
                Ok(meta.get_env_second_args()
                    .get(idx)
                    .cloned()
                    .unwrap_or_else(|| "__".into()))
            })
        }

        fn meta_dbg:MetaDebug(meta) [] {
            let msg = format!("{:#?}", meta);
            meta.log_info(msg);
            Ok("__".into())
        }

        fn max_expand_depth:MaxExpandDepth(meta) [] {
            Ok(meta.const_expand_max_depth().to_string().into())
        }

        fn set_max_expand_depth:SetMaxExpandDepth(meta) [d:depth] {
            check_type!("var" Value::Var(depth) = depth.value() => {
                let depth: usize = match num(depth) {
                    Some(n) => n,
                    None => {
                        return Err((2, format!("Invalid max expand depth: `{depth}`")))
                    },
                };
                meta.set_const_expand_max_depth(depth);
                Ok("__".into())
            })
        }

        fn stop_repeat:StopRepeat(meta) [] {
            let Some(flag) = meta.args_repeat_flags.last_mut() else {
                return Err((2, "Not in repeat block".into()))
            };
            *flag = false;
            Ok("__".into())
        }

        fn repeat_limit:RepeatLimit(meta) [] {
            Ok(meta.args_repeat_limit().to_string().into())
        }

        fn set_repeat_limit:SetRepeatLimit(meta) [d:limit] {
            check_type!("var" Value::Var(limit) = limit.value() => {
                let limit: usize = match num(limit) {
                    Some(n) => n,
                    None => {
                        return Err((2, format!("Invalid number `{limit}`")))
                    },
                };
                meta.set_args_repeat_limit(limit);
                Ok("__".into())
            })
        }

        fn expand_stack:ExpandStack(meta) [] {
            meta.log_expand_stack::<false>();
            Ok("__".into())
        }

        fn eval_num:EvalNum(meta) [v:value] {
            let Some((num, _)) = value.value().try_eval_const_num(meta) else {
                return Ok("__".into());
            };
            Ok(Value::fmt_literal_num(num))
        }

        fn is_string:IsString(meta) [v:value] {
            Ok(value.value().as_var()
                .and_then(|v| Value::is_string(v)
                    .then(|| "1".into()))
                .unwrap_or_else(|| "0".into()))
        }

        fn ref_arg:RefArg(meta) [i:index] {
            check_type!("var" Value::Var(index) = index.value() => {
                let Some(index) = num(index) else {
                    return Err((2, format!("Invalid start value: `{index}`")))
                };
                let args = meta.get_env_second_args();
                args.get(index)
                    .cloned()
                    .ok_or_else(|| {
                        (2, format!(
                            "Index out of range ({index} >= {})",
                            args.len()))
                    })
            })
        }

        fn misses_match:MissesMatch(meta) [v:enable] {
            check_type!("var" Value::Var(enable) = enable.value() => {
                meta.enable_misses_match_log_info = enable != "0";
                Ok("__".into())
            })
        }

        fn unused_no_effect:UnusedNoEffect(meta) [v:enable] {
            check_type!("var" Value::Var(enable) = enable.value() => {
                meta.enable_unused_no_effect_info = enable != "0";
                Ok("__".into())
            })
        }

        fn misses_bind:MissesBind(meta) [v:enable] {
            check_type!("var" Value::Var(enable) = enable.value() => {
                meta.enable_misses_bind_info = enable != "0";
                Ok("__".into())
            })
        }

        fn misses_binder_ref:MissesBinderRef(meta) [v:enable] {
            check_type!("var" Value::Var(enable) = enable.value() => {
                meta.enable_misses_binder_ref_info = enable != "0";
                Ok("__".into())
            })
        }

        fn set_noop:SetNoOp(meta) [l:line] {
            check_type!("var" Value::Var(line) = line.value() => {
                let line = if Value::is_string(line) {
                    &line[1..line.len()-1]
                } else {
                    line
                };
                let line = match escape_doublequote(line.trim()) {
                    Ok(s) => s,
                    Err(e) => return Err((2, e.into())),
                };
                if 0 != line.chars().filter(|&c| c == '"').count() & 1 {
                    return Err((2, "双引号未配对".into()));
                }
                meta.noop_line = line;
                Ok("__".into())
            })
        }

        fn bind_sep:BindSep(meta) [v:sep] {
            check_type!("var" Value::Var(sep) = sep.value() => {
                match sep.as_var_type().as_string() {
                    Some(&"") => {
                        meta.bind_custom_sep = None;
                    },
                    Some(s) => {
                        return Err((2, format!(
                            "expected empty string, found: {s:?}")));
                    },
                    None => {
                        meta.bind_custom_sep = sep.to_owned().into();
                    },
                }
                Ok("__".into())
            })
        }

        fn chr:Chr(meta) [n:code] {
            check_type!("var" Value::Var(code) = code.value() => {
                let Some(code) = num(code) else {
                    return Err((2, format!("Invalid chr number: {code}")))
                };
                let Ok(code) = code.try_into() else {
                    return Err((2, format!("Invalid number range: {code}")))
                };
                let Some(char) = char::from_u32(code) else {
                    return Err((2, format!("Invalid char point: {code}")))
                };
                if char == '"' || char == '\n' {
                    return Err((2, format!("Invalid string char: {char:?}")))
                }
                Ok(format!("\"{char}\"").into())
            })
        }

        fn ord:Ord(meta) [n:code] {
            check_type!("var" Value::Var(char) = code.value() => {
                if char == r"\n" {
                    Ok(format!("{}", b'\n').into())
                } else if char == r"\r" {
                    Ok(format!("{}", b'\r').into())
                } else if char == r"\t" {
                    Ok(format!("{}", b'\t').into())
                } else if char == r"\e" {
                    Ok(format!("{}", b'\x1b').into())
                } else if char == r"'" {
                    Ok(format!("{}", b'"').into())
                } else if Value::is_string(char) {
                    if char.chars().nth(2).is_none() || char.chars().nth(3).is_some() {
                        return Err((2, format!("Ord[] cannot support multi chars {char}")))
                    }
                    Ok((char.chars().nth(1).unwrap() as u32).to_string().into())
                } else {
                    if char.chars().nth(1).is_some() {
                        return Err((2, format!("Ord[] cannot support multi chars '{char}'")))
                    }
                    Ok((char.chars().next().unwrap() as u32).to_string().into())
                }
            })
        }
    }
}
