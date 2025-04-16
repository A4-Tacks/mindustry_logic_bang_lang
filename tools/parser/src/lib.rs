mod parser;
pub use crate::parser::*;
pub use ::lalrpop_util;

use ::syntax::{
    Var,
    ValueBind,
    ValueBindRef,
    ValueBindRefTarget,
    Const,
    Take,
    ConstKey,
    Meta,
};

fn make_take_destructs(
    meta: &mut Meta,
    key: &mut ConstKey,
    de: Vec<(bool, Var, Var)>,
) {
    if let ConstKey::ValueBind(ValueBind(binder, _)) = key {
        let tmp = meta.get_tmp_var();
        let binder = core::mem::replace(&mut **binder, tmp.clone().into());
        meta.add_line_dep(Take(tmp.into(), binder).into());
    }
    for (c, dst, src) in de {
        meta.add_line_post(if c {
            Const(dst.into(), ValueBindRef::new(
                Box::new((*key).clone().into()),
                ValueBindRefTarget::NameBind(src),
            ).into(), vec![]).into()
        } else {
            Take(dst.into(), ValueBind(
                Box::new(key.clone().into()),
                src,
            ).into()).into()
        });
    }
}
