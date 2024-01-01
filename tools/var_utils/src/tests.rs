use super::*;

#[test]
fn test() {
    macro_rules! test_syn {
        (
            true { $($a:literal),* $(,)? }
            false { $($b:literal),* $(,)? }
        ) => {
            ([$($a),*], [$($b),*])
        };
    }

    let (true_datas, false_datas)
        = test_syn! {
            true {
                "a",
                "abc",
                "我",
                "你我他",
                "_你我他",
                "_你_我他",
                "_foo",
                "_bar",
                "_bar_x",
                "@x",
                "@xyz",
                "@_yz",
                "@abc_def",
                "@abc-def",
                "@abc-def-",
                "@abc--def--",
                "@a--def--",
                "0123",
                "1_000_000",
                "123.456",
                "-123.456",
                "-1_23.456",
                "-1_23.4_56",
                "-1_23.4_56_",
                "1_23.4_56_",
                "0b0010_1011",
                "0b-0010_1011",
                "0x1b",
                "0x1B",
                "0x-1B",
                "0x-1B_2F",
                "0x-1B_2F_",
                "0x1B_2F_",
            }
            false {
                "+",
                "-",
                "0xg",
                "0x_2",
                "0b3",
                "-0b10",
                "-_2",
                "2._3",
                "0XFF",
                "abc-def",
                "@-abc-def",
                "'foo'",
            }
        };
    for str in true_datas {
        assert!(is_ident(str), "err: is_ident({str:?}) == true assert failed.");
    }
    for str in false_datas {
        assert!(! is_ident(str), "err: is_ident({str:?}) == false assert failed.");
    }
}

#[test]
fn float_parser_test() {
    let src = [
        "1.2",
        "0",
        "0.",
        ".0",
        "00.",
        ".00",
        ".01",
        "1e3",
        "1e03",
        "01e+3",
        "01e+03",
        "1e-3",
        "001e-3",
        "001e-03",
        "100000",
        "123.456",
        "000123.456",
        "1234567891011",
        "-2",
        "-2.3",
        "-234.345",
        "-234.",
        "-.345",
        "-2e12",
        "null",
        "true",
        "false",
    ];
    for src in src {
        let r#type = src.as_var_type();
        assert!(matches!(r#type, VarType::Number(_)), "{:?}", r#type);
    }

    let bad_src = [
        "-",
        "-.",
        ".",
        "1.2e3",
        "e3",
        "-e3",
    ];
    for src in bad_src {
        let r#type = src.as_var_type();
        assert!(!matches!(r#type, VarType::Number(_)), "{:?}", r#type);
    }
}

#[test]
fn mod_op_test() {
    assert_eq!(2.0 % 2.0, 0.0);
    assert_eq!(1.0 % 2.0, 1.0);
    assert_eq!(3.0 % 2.0, 1.0);

    assert_eq!(-0.2 % 1.0, -0.2); // 这是必要的防御, 在python, 它为0.8
    assert_eq!(-0.2 % 2.0, -0.2);
}

#[test]
fn string_escape_test() {
    let strs = [
        ("a", r"a"),
        ("a\nb", r"a\nb"),
        ("a\r\nb", r"a\nb"),
        ("a\r\r\nb\r", r"a\nb"),
        ("a\\\n     b", r"ab"),
        ("a\\\n    \\ b", r"a b"),
        ("a\\\n   \\  b", r"a  b"),
        ("a\\\n\\\n   \\  b", r"a  b"),
        ("a\\\n\\\r\n   \\  b", r"a  b"),
        ("a\\\n\\\r\n   \\ \\\\ b", r"a \ b"),
        ("a\\\n\n b", r"a\n b"),
        ("a\\\\b", r"a\b"),
        ("a\\\\n", r"a\[]n"),
        ("a\\[red]b", r"a[[red]b"),
        ("你好", r"你好"),
    ];
    for (src, dst) in strs {
        assert_eq!(string_escape(src), dst);
    }
}

#[test]
fn string_unescape_test() {
    let strs = [
        (r"a", r"a"),
        (r"a\nb", r"a\nb"),
        (r"a\r\nb", r"a\\r\nb"),
        (r"a \ b", r"a \\ b"),
        (r"a \[] b", r"a \\ b"),
        (r"a \[red] b", r"a \\[red] b"),
        (r"a \\[red] b", r"a \\\\[red] b"),
        (r"a [[red] b", r"a [[red] b"),
        (r"你好", r"你好"),
    ];
    for (src, dst) in strs {
        assert_eq!(string_unescape(src), dst);
    }
}
