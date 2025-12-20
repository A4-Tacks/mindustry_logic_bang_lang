use tag_code::logic_parser::Args;

use super::*;

macro_rules! case {
    (while $c:tt {$($t:tt)*}) => {
        Reduce::While(
            case!($c),
            std::iter::empty().collect(),
            [$(case!($t)),*].into_iter().collect(),
        )
    };
    (while[$($d:tt)*] $c:tt {$($t:tt)*}) => {
        Reduce::While(
            case!($c),
            [$(case!($d)),*].into_iter().collect(),
            [$(case!($t)),*].into_iter().collect(),
        )
    };
    (dowhile {$($t:tt)*}) => {
        Reduce::DoWhile(
            case!(-),
            [$(case!($t)),*].into_iter().collect(),
        )
    };
    (dowhile $c:tt {$($t:tt)*}) => {
        Reduce::DoWhile(
            case!($c),
            [$(case!($t)),*].into_iter().collect(),
        )
    };
    (skip {$($t:tt)*}) => {
        Reduce::Skip(
            case!(-),
            [$(case!($t)),*].into_iter().collect(),
        )
    };
    (skip $c:tt {$($t:tt)*}) => {
        Reduce::Skip(
            case!($c),
            [$(case!($t)),*].into_iter().collect(),
        )
    };
    (if $c:tt {$($t:tt)*} else {$($f:tt)*}) => {
        Reduce::IfElse(
            case!($c),
            [$(case!($t)),*].into_iter().collect(),
            [$(case!($f)),*].into_iter().collect(),
        )
    };
    (jump) => {
        Reduce::Jump(crate::Jump(crate::Label(0), case!(-)))
    };
    (jump $($c:tt)+) => {
        Reduce::Jump(crate::Jump(crate::Label(0), case!($($c)+)))
    };
    (($($t:tt)*)) => {
        case!($($t)*)
    };
    ({$($t:tt)*}) => {
        Reduce::Product(
            [$(case!($t)),*].into_iter().collect(),
        )
    };
    (-) => {
        Cmp::Cond(crate::supp::CondOp::Always, Args::try_from(vec!["0"]).unwrap())
    };
    ($a:tt & $b:tt $(& $r:tt)*) => {{
        let cond = Cmp::And(Box::new(case!($a)), Box::new(case!($b)));
        case!((.cond) $(& $r)*)
    }};
    ($a:tt | $b:tt $(| $r:tt)*) => {{
        let cond = Cmp::Or(Box::new(case!($a)), Box::new(case!($b)));
        case!((.cond) $(& $r)*)
    }};
    (:) => {
        Reduce::Label(crate::Label(0))
    };
    ($n:literal) => {
        Reduce::Pure(std::iter::repeat_n(Args::try_from(vec!["cmd"]).unwrap(), $n).collect())
    };
    (.$i:ident) => {
        $i.clone()
    };
}

#[track_caller]
fn check_lossless_impl(a: Reduce<'_>, b: Reduce<'_>, eq: bool) {
    let loss_a = a.loss();
    let loss_b = b.loss();
    if loss_a > loss_b || !eq && loss_a == loss_b {
        let op = if eq { ">" } else { ">=" };
        println!("nodeA:=========================\n{a:x}");
        println!("nodeB:=========================\n{b:x}");
        panic!("loss_a({loss_a}) {op} loss_b({loss_b})")
    }
}

#[track_caller]
fn check_lossless(a: Reduce<'_>, b: Reduce<'_>) {
    check_lossless_impl(a, b, false);
}

#[track_caller]
fn check_eq(a: Reduce<'_>, b: Reduce<'_>) {
    check_lossless_impl(a.clone(), b.clone(), true);
    check_lossless_impl(b, a, true);
}

fn inputs() -> impl Iterator<Item = (Cmp<'static>, Reduce<'static>)> {
    [
        case!(1),
        case!(10),
        case!({0}),
        case!({1}),
        case!({10}),
        case!(:),
        case!({:}),
        case!({: :}),
        case!(jump),
        case!(jump -&-),
        case!({jump jump}),
    ].into_iter().flat_map(|reduce| [
        case!(-),
        case!(-&-),
        case!(-&-&-),
        case!(-&(-&-)),
        case!(-&(-|-)),
        case!(-|(-&-)),
        case!(-|-|-|-|-),
    ].into_iter().map(move |cmp| (cmp, reduce.clone())))
}

#[test]
fn basic_test_case_make() {
    let _node = case!(while - {
        (while[3] - {})
        (while (-&-) {2})
        (dowhile (-&-&-) {2 3})
        (skip ((-&-)&(-|-)) {2 {3 4} {}})
        (if - {2} else {{{}}})
        (if - {jump} else {:})
    });
}

#[test]
fn basic_pure() {
    check_eq(
        case!(0),
        case!(0),
    );
    check_eq(
        case!(1),
        case!(1),
    );
    check_eq(
        case!(10),
        case!(10),
    );
}

#[test]
fn while_pefer_skip_dowhile() {
    for (cmp, reduce) in inputs() {
        check_lossless(
            case!(while (.cmp) {(.reduce)}),
            case!(skip (.cmp) {(dowhile (.cmp) {(.reduce)})}),
        );
    }
}

#[test]
fn ifelse_pefer_goto_skip() {
    for (cmp, reduce) in inputs() {
        for (_, reduce1) in inputs() {
            check_lossless(
                case!({(if (.cmp) {(.reduce)} else {(.reduce1)})}),
                case!({(jump .cmp) (.reduce)(skip {:(.reduce1)})}),
            );
        }
    }
}

#[test]
fn ifelse_pefer_skip_goto() {
    for (cmp, reduce) in inputs() {
        for (_, reduce1) in inputs() {
            check_lossless(
                case!({(if (.cmp) {(.reduce)} else {(.reduce1)})}),
                case!({(skip (.cmp) {(.reduce1) jump}) (.reduce) :}),
            );
        }
    }
}

#[test]
fn make_skip() {
    for (cmp, reduce) in inputs() {
        for (_, body) in inputs() {
            check_lossless(
                case!({(.reduce)(skip (.cmp) {(.body)})(.reduce)}),
                case!({(.reduce)jump(.body):(.reduce)}),
            );
        }
    }
}

#[test]
fn make_small_skip_pefer_big_skip() {
    for (_, reduce) in inputs() {
        check_lossless(
            case!({1 (skip {(.reduce)}) 8}),
            case!({(skip {1 (.reduce) 8})}),
        );
        check_lossless(
            case!({4 (skip {(.reduce) 8})}),
            case!({(skip {4 (.reduce) 8})}),
        );
    }
}

#[test]
fn make_small_dowhile_pefer_big_dowhile() {
    check_lossless(
        case!({:1 (dowhile {2}) 6 jump}),
        case!({(dowhile {1 : 2 jump 6})}),
    );
    for (_, body) in inputs() {
        check_lossless(
            case!({1 (skip {(.body)}) 8}),
            case!({(skip {1 (.body) 8})}),
        );
    }
    for (_, body) in inputs() {
        for (_, before) in inputs() {
            check_lossless(
                case!({1 (.before) (skip {(.body)}) 8}),
                case!({(skip {1 (.before) (.body) 8})}),
            );
            check_lossless(
                case!({1 (.before) (skip {(.body)}) 8}),
                case!({1 (skip {(.before) (.body) 8})}),
            );
        }
    }
}

#[test]
fn comb_cond_pefer_multi_goto() {
    for (a, _) in inputs() {
        for (b, _) in inputs() {
            check_lossless(
                case!({:(jump (.a)&(.b))}),
                case!({:(jump .a)(jump .b)}),
            );
            check_lossless(
                case!({:(jump (.a)|(.b))}),
                case!({:(jump .a)(jump .b)}),
            );
        }
    }
}

#[test]
fn comb_cond_pefer_nested_dowhile() {
    for (a, reduce) in inputs() {
        for (b, _) in inputs() {
            check_lossless(
                case!(dowhile ((.a)&(.b)) {(.reduce)}),
                case!(dowhile (.a) {(dowhile (.b) {(.reduce)})}),
            );
        }
    }
}
