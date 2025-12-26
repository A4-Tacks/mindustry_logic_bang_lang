mod parser;
pub use parser::*;
pub use ::lalrpop_util;

use lalrpop_util::ParseError;
use ::syntax::{
    Expand,
    Select,
    Goto,
    Var,
    Value,
    ValueBind,
    ValueBindRef,
    ValueBindRefTarget,
    LogicLine,
    InlineBlock,
    SwitchCatch,
    JumpCmp,
    CmpTree,
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

fn make_skip(
    meta: &mut Meta,
    cmp: CmpTree,
    body: LogicLine,
) -> LogicLine {
    let lab = meta.get_tag();
    Expand(vec![
        Goto(lab.clone(), cmp).into(),
        body,
        LogicLine::new_label(lab, meta),
    ]).into()
}

fn make_do_while(
    meta: &mut Meta,
    body: LogicLine,
    (break_lab, continue_lab): (Option<Var>, Option<Var>),
    cmp: CmpTree,
) -> LogicLine {
    let head = meta.get_tag();
    let mut res = Vec::with_capacity(5);

    res.extend([
        LogicLine::new_label(head.clone(), meta),
        body,
    ]);
    meta.push_some_label_to(&mut res, continue_lab);
    res.push(Goto(head, cmp).into());
    meta.push_some_label_to(&mut res, break_lab);

    Expand(res).into()
}

fn make_while(
    meta: &mut Meta,
    cmp: CmpTree,
    body: LogicLine,
    (break_lab, continue_lab): (Option<Var>, Option<Var>),
) -> LogicLine {
    let [end, head] = [meta.get_tag(), meta.get_tag()];
    let rev_cmp = cmp.clone().reverse();
    let mut res = Vec::with_capacity(7);

    res.extend([
        Goto(end.clone(), rev_cmp).into(),
        LogicLine::new_label(head.clone(), meta),
        body,
    ]);
    meta.push_some_label_to(&mut res, continue_lab);
    res.extend([
        Goto(head, cmp).into(),
        LogicLine::new_label(end, meta),
    ]);
    meta.push_some_label_to(&mut res, break_lab);

    Expand(res).into()

}

fn make_gwhile(
    meta: &mut Meta,
    cmp: CmpTree,
    body: LogicLine,
    (break_lab, continue_lab): (Option<Var>, Option<Var>),
) -> LogicLine {
    let [to, head] = [meta.get_tag(), meta.get_tag()];
    let mut res = Vec::with_capacity(7);

    res.extend([
        Goto(to.clone(), JumpCmp::Always.into()).into(),
        LogicLine::new_label(head.clone(), meta),
        body,
        LogicLine::new_label(to, meta),
    ]);
    meta.push_some_label_to(&mut res, continue_lab);
    res.push(Goto(head, cmp).into());
    meta.push_some_label_to(&mut res, break_lab);

    Expand(res).into()
}

fn make_switch(
    meta: &mut Meta,
    value: Value,
    mut append: Vec<LogicLine>,
    catchs: Vec<(Vec<SwitchCatch>, Option<Var>, Expand)>,
    cases: Vec<(bool, Vec<usize>, Expand)>,
    (break_lab, continue_lab): (Option<Var>, Option<Var>),
) -> LogicLine {
    let catchs_is_empty = catchs.is_empty();

    let mut next_case_num = 0;
    let case_num_max = cases
        .iter()
        .map(
            |(_, nums, _)| {
                let num = nums
                    .iter()
                    .max()
                    .copied()
                    .unwrap_or(next_case_num);
                next_case_num = num + 1;
                num
            }
        )
        .max()
        .unwrap();

    // 用于填充填充case的行, 如果有追加在末尾的行则将其封装并替换填充
    let (mut fill_line, append) = match &append[..] {
        [] => (LogicLine::Ignore, None),
        [_] => (
            Expand(vec![append.last().unwrap().clone()]).into(),
            append.pop().unwrap().into(),
        ),
        [..] => (
            Expand(append.clone()).into(),
            Some(Expand(append).into()),
        ),
    };

    // 用于添加到头部的捕获块
    let mut catch_lines = Vec::new();
    let value_handle: Var = if catchs_is_empty {
        Var::new()
    } else { meta.get_tmp_var() };

    // 这里开始遍历捕获
    // 如果遇到了未命中捕获, 则改变fill_line为总是跳转到未命中捕获
    for (flags, name, lines) in catchs {
        let mut out_block = Vec::new();
        let skip_cmp = CmpTree::new_ands(
            flags
                .into_iter()
                .filter(|flag| {
                    if flag.is_misses() {
                        // 是一个未命中捕获
                        let tag = meta.get_tag();
                        out_block.push(LogicLine::new_label(tag.clone(), meta));
                        fill_line = Goto(tag, JumpCmp::Always.into()).into();
                        false // 已处理, 过滤掉
                    } else {
                        true
                    }
                })
                .map(|flag|
                    flag.build(value_handle.as_str().into(), case_num_max)
                )
        ).unwrap_or(JumpCmp::Always.into());
        let skip_tag = meta.get_tag();
        out_block.insert(0, Goto(skip_tag.clone(), skip_cmp).into());
        if let Some(name) = name {
            // 如果有捕获变量则使用一个const进行映射
            // 这需要插入在头部, 也就是条件前
            // 防止`case (a) a:`时, a还没被const就进行了判断
            out_block.insert(
                0,
                Const::new(
                    name.into(),
                    value_handle.as_str().into()
                ).into()
            )
        }
        out_block.push(lines.into());
        out_block.push(LogicLine::new_label(skip_tag, meta));

        catch_lines.push(Expand(out_block).into())
    }

    let mut cases_res = Vec::with_capacity(case_num_max + 1);
    let mut cases_res_isline = vec![false; case_num_max + 1];

    let mut next_ignored_num = 0;
    for (ignore_append, mut nums, mut expand) in cases {
        if let Some(append) = &append {
            if !ignore_append {
                expand.push(append.clone())
            }
        }
        if nums.is_empty() { nums.push(next_ignored_num) }
        for num in nums {
            for _ in cases_res.len()..=num {
                cases_res.push(LogicLine::Ignore);
            }
            cases_res[num] = expand.clone().into();
            cases_res_isline[num] = true;
            next_ignored_num = num + 1;
        }
    }
    // 将填充行填入填充case
    let mut iter = cases_res_isline.into_iter().enumerate().peekable();
    while let Some((idx, is_line)) = iter.next() {
        if is_line { continue }
        match iter.peek() {
            Some((_, true)) => cases_res[idx] = fill_line.clone(),
            _ => (),
        }
    }
    debug_assert_eq!(cases_res.len(), case_num_max + 1);
    debug_assert_eq!(cases_res.len(), cases_res.capacity());

    if catchs_is_empty {
        // 没有捕获块
        let mut res = Vec::with_capacity(3);

        meta.push_some_label_to(&mut res, continue_lab);
        res.push(Select(value, Expand(cases_res)).into());
        meta.push_some_label_to(&mut res, break_lab);

        if res.len() == 1 {
            res.pop().unwrap()
        } else {
            Expand(res).into()
        }
    } else {
        // 有捕获块
        // 保证我们拿到了一个临时返回句柄, 而不是一个空值
        assert_ne!(&value_handle, "");
        let mut res = Vec::with_capacity(5);

        meta.push_some_label_to(&mut res, continue_lab);
        res.extend([
            // 求值
            Take(value_handle.as_str().into(), value).into(),
            // 捕获
            Expand(catch_lines).into(),
            // 主体
            Select(value_handle.into(), Expand(cases_res)).into()
        ]);
        meta.push_some_label_to(&mut res, break_lab);

        Expand(res).into()
    }
}

fn make_select(
    meta: &mut Meta,
    value: Value,
    lines: Expand,
    (break_lab, continue_lab): (Option<Var>, Option<Var>),
) -> LogicLine {
    let mut res = Vec::with_capacity(3);

    meta.push_some_label_to(&mut res, continue_lab);
    res.push(Select(value, lines).into());
    meta.push_some_label_to(&mut res, break_lab);

    if res.len() == 1 {
        res.pop().unwrap()
    } else {
        Expand(res).into()
    }
}

fn get_locations<const N: usize>(src: &str, indexs: [usize; N]) -> [[u32; 2]; N] {
    line_column::line_columns(src, indexs)
        .map(|(line, col)| [line, col])
}

pub fn format_parse_err<const MAX_INVALID_TOKEN_VIEW: usize>(
    e: ParseError<usize, parser::Token<'_>, syntax::Error>,
    src: &str,
) -> String {
    use syntax::{Errors, Error};
    use std::ops::Deref;
    fn fmt_token<'a>(i: impl IntoIterator<Item = &'a str>)
    -> impl Iterator<Item = &'a str> {
        i.into_iter()
            .map(|s| get_token_name(s).unwrap_or(s))
    }
    match e {
        ParseError::UnrecognizedToken {
            token: (start, token, end),
            expected
        } => {
            let [start, end] = get_locations(src, [start, end]);
            format!(
                "在位置 {:?} 至 {:?} 处找到不应出现的令牌: {:?}\n\
                预期: [{}]",
                start, end,
                token.1,
                fmt_token(expected.iter().map(Deref::deref))
                    .collect::<Vec<_>>()
                    .join(", "),
            )
        },
        ParseError::ExtraToken { token: (start, token, end) } => {
            let [start, end] = get_locations(src, [start, end]);
            format!(
                "在位置 {:?} 至 {:?} 处找到多余的令牌: {:?}",
                start, end,
                fmt_token(Some(token.1)).next().unwrap(),
            )
        },
        ParseError::InvalidToken { location } => {
            let [loc] = get_locations(src, [location]);
            let view = &src[
                location
                ..
                src.len().min(
                    src[location..]
                        .char_indices()
                        .map(|(i, _ch)| location+i)
                        .take(MAX_INVALID_TOKEN_VIEW+1)
                        .last()
                        .unwrap_or(location))
            ];
            format!(
                "在位置 {:?} 处找到无效的令牌: {:?}",
                loc,
                view.trim_end(),
            )
        },
        ParseError::UnrecognizedEof {
            location,
            expected
        } => {
            let [start] = get_locations(src, [location]);
            format!(
                "在位置 {:?} 处意外的结束\n\
                预期: [{}]",
                start,
                fmt_token(expected.iter().map(Deref::deref))
                    .collect::<Vec<_>>()
                    .join(", "),
            )
        },
        ParseError::User {
            error: Error {
                start,
                end,
                err
            }
        } => {
            let [start, end]
                = get_locations(src, [start, end]);
            let out = |msg| format!(
                "在位置 {:?} 至 {:?} 处的错误:\n{}",
                start,
                end,
                msg
            );
            match err {
                Errors::NotALiteralUInteger(str, err) => {
                    out(format_args!(
                        "{:?} is not a valided unsigned integer: {}",
                        str,
                        err,
                    ))
                },
                Errors::UnpairBranches(left, right) => {
                    out(format_args!(
                        "unpair branches, left: {}, right: {}",
                        left,
                        right,
                    ))
                },
                Errors::OpExprInvalidResult { found, right } => {
                    out(format_args!(
                        "{} op-expr can't pattern {} results, expected 1 or {}",
                        right,
                        found,
                        found,
                    ))
                },
                Errors::MultipleOpExpr => {
                    out(format_args!(
                        "此处不应展开多个 op-expr",
                    ))
                },
                #[allow(unreachable_patterns)]
                e => {
                    out(format_args!("未被枚举的错误: {:?}", e))
                },
            }
        },
    }
}

fn get_token_name(s: &str) -> Option<&'static str> {
    match s {
        r###"r#"[_\\p{XID_Start}]\\p{XID_Continue}*|__lsp_hover__@"#"###
            => "Identify",
        r###"r#"@[_\\p{XID_Start}][\\p{XID_Continue}\\-]*"#"###
            => "OIdentify",
        r###"r#"-?(?:0x-?[\\da-fA-F][_\\da-fA-F]*|0b-?[01][_01]*|\\d[_\\d]*(?:\\.\\d[\\d_]*|e[+\\-]?\\d[\\d_]*)?)"#"###
            => "Number",
        r###"r#"\"(?:\\\\\\r?\\n\\s*(?:\\\\ )?|\\r?\\n|\\\\[n\\\\\\[]|[^\"\\r\\n\\\\])*\""#"###
            => "String",
        r###"r#"'[^'\\s]+'"#"###
            => "OtherVariable",
        _ => return None,
    }.into()
}
