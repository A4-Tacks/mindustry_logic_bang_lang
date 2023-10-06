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
