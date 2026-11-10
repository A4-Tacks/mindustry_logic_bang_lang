//! 一系列通用工具

use std::{fmt, ops};

pub mod counter;

/// 当编号过大时, 使用62进制进行编码, 降低长度
pub fn gen_anon_name<R>(
    id: usize,
    f: impl FnOnce(fmt::Arguments<'_>) -> R,
) -> R {
    if id < 1000 {
        f(format_args!("{id}"))
    } else {
        f(format_args!("0{}", num_radix(id, 62)))
    }
}


fn num_radix<N>(mut n: N, radix: N) -> String
where N: ops::DivAssign + ops::Rem<Output = N>
         + Eq + fmt::Debug + Default + Into<usize>
         + Copy,
{
    const LIST: &[u8; 62]
        = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
    let zero = N::default();
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
