use super::check;
use expect_test::expect;

#[test]
fn simple_switch() {
    check(r#"
        op mul x input 3
        op add @counter @counter x
        set i 2
        set j 3
        jump end always 0 0
        set i 4
        set j 5
        jump end always 0 0
        set i 7
        set j 9
        jump end always 0 0
        end:
        print i
    "#, expect![]);
}
