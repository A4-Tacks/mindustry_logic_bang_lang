use std::process;

use crate::*;

#[derive(Debug, Clone)]
pub struct BuiltinFunc {
    name: &'static str,
    func: fn(&Self, &mut CompileMeta) -> Var,
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
    fn value_type(value: &Value) -> &'static str {
        match value {
            Value::Var(_) => "var",
            Value::DExp(_) => "dexp",
            Value::ReprVar(_) => "reprvar",
            Value::ResultHandle => "resulthandle",
            Value::ValueBind(_) => "valuebind",
            Value::Cmper(_) => "cmper",
            Value::Binder => "binder",
            Value::BuiltinFunc(_) => "builtinfunc",
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
                    format!("\"{var}\"")
                })
            })
        }

        fn status:Status(meta) [] {
            Ok(meta.last_builtin_exit_code().to_string())
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
                    Ok([&a[..a.len()-1], &b[1..]].concat())
                })
            })
        }

        fn info:Info(meta) [var:data] {
            let value = data.value();
            check_type!("var" Value::Var(var) = value => {
                let msg = String::from(var);
                meta.log_info(msg.clone());
                Ok(msg)
            })
        }

        fn err:Err(meta) [var:data] {
            let value = data.value();
            check_type!("var" Value::Var(var) = value => {
                let msg = String::from(var);
                meta.log_err(msg.clone());
                Ok(msg)
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
                let name: String = name.into();
                Const(name.clone().into(), value.value().clone(), vec![]).compile(meta);
                meta.add_const_value_leak(name);
                Ok("__".into())
            })
        }

        fn binder:Binder(meta) [n:name v:value] {
            check_type!("var" Value::Var(name) = name.value() => {
                check_type!("valuebind" Value::ValueBind(ValueBind(binder, _)) = value.value() => {
                    let name: String = name.into();
                    Const(name.clone().into(), binder.as_ref().clone(), vec![])
                        .compile(meta);
                    meta.add_const_value_leak(name);
                    Ok("__".into())
                })
            })
        }

        fn exit:Exit(meta) [n:code] {
            check_type!("var" Value::Var(code) = code.value() => {
                let num_code = match code.parse() {
                    Ok(code) => code,
                    Err(e) => {
                        meta.log_err(format!(
                            "Invalid exit code: {code},\nerr: {e}",
                        ));
                        128
                    },
                };
                process::exit(num_code)
            })
        }

        /// 以Debug形式显示一个值
        fn debug:Debug(meta) [v:value] {
            meta.log_info(format!("Value Debug:\n{:#?}", value));
            Ok("__".into())
        }
    }
}