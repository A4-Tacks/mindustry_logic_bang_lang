mod parser;
pub use crate::parser::*;
pub use ::lalrpop_util;

use ::syntax::{
    Var,
    Value,
    ValueBind,
    ValueBindRef,
    ValueBindRefTarget,
    LogicLine,
    InlineBlock,
    Const,
    Take,
    ConstKey,
    Meta,
};

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
        .chain(std::iter::once(Take(key.clone(), val).into()))
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
