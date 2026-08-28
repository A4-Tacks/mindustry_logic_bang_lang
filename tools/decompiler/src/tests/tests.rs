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
    "#, expect![[r#"
          1/30  limite        5 -> 5        <13.70000 $ 32.00000>
          2/30  limite       10 -> 10       <13.70000 $ 32.00000>
          3/30  limite       12 -> 12       <13.70000 $ 32.00000>
          4/30  limite       12 -> 12       <13.70000 $ 32.00000>
        -- Early Reconstruction Completed

        #---------- reduce[0/0] case 0 <5.6000004> ----------
        gswitch input {
        case 0:
            set i 2;
            set j 3;
            break _;
        case 1:
            set i 4;
            set j 5;
            break _;
        case 2:
            set i 7;
            set j 9;
            break _;
        }
        print i;
    "#]]);
}

#[test]
fn simple_gswitch() {
    check(r#"
            op add @counter @counter input
            jump __0 always 0 0
            jump __1 always 0 0
            jump __2 always 0 0
        __0:
            set i 2
            set j 3
            jump end always 0 0
        __1:
            set i 4
            set j 5
            jump end always 0 0
        __2:
            set i 7
            set j 9
            jump end always 0 0
        end:
            print i
    "#, expect![[r#"
          1/30  limite        8 -> 8        <13.70000 $ 52.50000>
          2/30  limite       18 -> 18       <11.20000 $ 52.50000>
          3/30  limite       23 -> 23       <11.20000 $ 52.50000>
          4/30  limite       24 -> 24       <11.20000 $ 52.50000>
          5/30  limite       24 -> 24       <11.20000 $ 52.50000>
        -- Early Reconstruction Completed

        #---------- reduce[0/0] case 0 <5.6000004> ----------
        gswitch input {
        case 0:
            set i 2;
            set j 3;
            break _;
        case 1:
            set i 4;
            set j 5;
            break _;
        case 2:
            set i 7;
            set j 9;
            if !_ {}
        }
        print i;
    "#]]);
}

#[test]
fn simple_while() {
    check(r#"
            jump ___0 greaterThanEq i 2
        ___1:
            print i
            jump ___1 lessThan i 2
        ___0:
            end
    "#, expect![[r#"
          1/30  limite        3 -> 3        <10.65000 $ 17.00000>
          2/30  limite        5 -> 5        <3.65000 $ 17.00000>
          3/30  limite        5 -> 5        <3.65000 $ 17.00000>
        -- Early Reconstruction Completed

        #---------- reduce[0/0] case 0 <3.65> ----------
        while i < 2 {
            print i;
        }
        end;
    "#]]);
}

#[test]
fn complex_while() {
    check(r#"
            jump ___1 greaterThanEq i 2
            jump ___1 lessThanEq j 3
        ___2:
            print i
            jump ___0 equal j 4
            jump __0 greaterThanEq i 2
            jump ___2 greaterThan j 3
        __0:
        ___1:
        ___0:
            end
    "#, expect![[r#"
          1/30  limite        8 -> 8        <31.00000 $ 38.00000>
          2/30  limite       23 -> 23       <25.34853 $ 38.00000>
          3/30  limite       34 -> 34       <19.58338 $ 38.00000>
          4/30  limite       37 -> 37       <13.84853 $ 38.00000>
          5/30  limite       37 -> 37       <13.84853 $ 38.00000>
        -- Early Reconstruction Completed

        #---------- reduce[0/0] case 0 <7.7071066> ----------
        while i < 2 && j > 3 {
            print i;
            break j == 4;
        }
        end;
    "#]]);
}

#[test]
fn simple_gwhile() {
    check(r#"
            jump ___0 always 0 0
        ___1:
            print i
        ___0:
            jump ___1 lessThan i j
            end
    "#, expect![[r#"
          1/30  limite        4 -> 4        <3.65000 $ 17.00000>
          2/30  limite        4 -> 4        <3.65000 $ 17.00000>
        -- Early Reconstruction Completed

        #---------- reduce[0/0] case 0 <3.65> ----------
        while i < j {
            print i;
        }
        end;
    "#]]);
}

#[test]
fn complex_gwhile() {
    check(r#"
            jump ___0 always 0 0
        ___1:
            print i
        ___0:
            jump __0 greaterThanEq i j
            jump ___1 greaterThan j k
        __0:
            end
    "#, expect![[r#"
          1/30  limite        6 -> 6        <11.09623 $ 24.00000>
          2/30  limite       10 -> 10       <3.65000 $ 24.00000>
          3/30  limite       10 -> 10       <3.65000 $ 24.00000>
        -- Early Reconstruction Completed

        #---------- reduce[0/0] case 0 <3.65> ----------
        while i < j && j > k {
            print i;
        }
        end;
    "#]]);
}

#[test]
fn simple_gwhile_with_dep() {
    check(r#"
            jump ___0 always 0 0
        ___1:
            print i
        ___0:
            dep
            jump ___1 lessThan i j
            end
    "#, expect![[r#"
          1/30  limite        4 -> 4        <4.83581 $ 18.50000>
          2/30  limite        4 -> 4        <4.83581 $ 18.50000>
        -- Early Reconstruction Completed

        #---------- reduce[0/0] case 0 <4.8358145> ----------
        while({
            dep;
        } => i < j) {
            print i;
        }
        end;
    "#]]);
}

#[test]
fn simple_gwhile_with_unfollow_linked_dep() {
    check(r#"
            jump x equal otherCond 0
            some
        x:
            jump ___0 always 0 0
        ___1:
            print i
        ___0:
            dep
            jump ___1 lessThan i j
            end
    "#, expect![[r#"
          1/30  limite        5 -> 5        <13.33581 $ 27.00000>
          2/30  limite        8 -> 8        <6.98581 $ 27.00000>
          3/30  limite        8 -> 8        <6.98581 $ 27.00000>
        -- Early Reconstruction Completed

        #---------- reduce[0/0] case 0 <6.9858146> ----------
        if otherCond != 0 {
            some;
        }
        while({
            dep;
        } => i < j) {
            print i;
        }
        end;
    "#]]);
}

#[test]
fn simple_gwhile_with_follow_linked_dep() {
    // FIXME; 这应该是一个 pattern 而不是 clean,
    // 否则 _0 提取出来也只能被 clean 成 break, 而不能被 reduce 成 skip
    check(r#"
            jump ___0 equal otherCond 0
            some
            jump ___0 always 0 0
        ___1:
            print i
        ___0:
            dep
            jump ___1 lessThan i j
            end
    "#, expect![[r#"
          1/30  limite        5 -> 5        <13.89401 $ 27.00000>
          2/30  limite        6 -> 6        <13.89401 $ 27.00000>
          3/30  limite        6 -> 6        <13.89401 $ 27.00000>
        -- Early Reconstruction Completed

        #---------- reduce[1/1] case 0 <13.894007> ----------
        goto :_0 otherCond == 0;
        some;
        while({
            :_0
            dep;
        } => i < j) {
            print i;
        }
        end;
    "#]]);
}

#[test]
fn simple_while1() {
    check(r#"
            jump :5 greaterThanEq i 6
            jump :5 lessThanEq j 3
        :2:
            print i
            jump :5 greaterThanEq i 6
            jump :2 greaterThan j 3
        :5:
            printflush message1
    "#, expect![[r#"
          1/30  limite        7 -> 7        <24.00000 $ 31.00000>
          2/30  limite       20 -> 20       <17.65000 $ 31.00000>
          3/30  limite       30 -> 30       <11.36500 $ 31.00000>
          4/30  limite       33 -> 33       <5.15150 $ 31.00000>
          5/30  limite       33 -> 33       <5.15150 $ 31.00000>
        -- Early Reconstruction Completed

        #---------- reduce[0/0] case 0 <3.65> ----------
        while i < 6 && j > 3 {
            print i;
        }
        printflush message1;
    "#]]);
}

#[test]
fn jump_to_break_nested() {
    check(r#"
            jump ___2 greaterThanEq i links
            sensor __32 __29 my_item
            jump ___1 notEqual __32 0
        ___0:
            jump _23 greaterThanEq i links
            sensor __33 __29 my_item
            jump ___0 equal __33 0
        ___1:
        ___2:
        _23:
            end
    "#, expect![[r#"
          1/30  limite        5 -> 5        <26.84853 $ 32.50000>
          2/30  limite        8 -> 8        <21.08338 $ 32.50000>
          3/30  limite        9 -> 9        <16.00939 $ 32.50000>
          4/30  limite        9 -> 9        <16.00939 $ 32.50000>
        -- Early Reconstruction Completed

        #---------- reduce[0/0] case 0 <12.07626> ----------
        if i < links {
            sensor __32 __29 my_item;
            if __32 == 0 {
                do {
                    break i >= links;
                    sensor __33 __29 my_item;
                } while __33 == 0;
            }
        }
        end;
    "#]]);
}

#[test]
fn simple_skip() {
    check(r#"
            jump ___0 lessThan a b
            print a
        ___0:
            end
    "#, expect![[r#"
          1/30  limite        2 -> 2        <3.65000 $ 10.00000>
          2/30  limite        2 -> 2        <3.65000 $ 10.00000>
        -- Early Reconstruction Completed

        #---------- reduce[0/0] case 0 <3.65> ----------
        if a >= b {
            print a;
        }
        end;
    "#]]);
}

#[test]
fn complex_skip() {
    check(r#"
            jump __0 greaterThanEq a b
            jump ___0 lessThan b c
        __0:
            print a
        ___0:
            end
    "#, expect![[r#"
          1/30  limite        4 -> 4        <10.00000 $ 17.00000>
          2/30  limite        5 -> 5        <3.65000 $ 17.00000>
          3/30  limite        5 -> 5        <3.65000 $ 17.00000>
        -- Early Reconstruction Completed

        #---------- reduce[0/0] case 0 <3.65> ----------
        if a >= b || b >= c {
            print a;
        }
        end;
    "#]]);
}

#[test]
fn simple_if_else() {
    check(r#"
            jump ___1 lessThan a b
            print b
            jump ___0 always 0 0
        ___1:
            print a
        ___0:
            end
    "#, expect![[r#"
          1/30  limite        4 -> 4        <5.30000 $ 18.50000>
          2/30  limite        4 -> 4        <5.30000 $ 18.50000>
        -- Early Reconstruction Completed

        #---------- reduce[0/0] case 0 <5.3> ----------
        if a >= b {
            print b;
        } else {
            print a;
        }
        end;
    "#]]);
}

#[test]
fn complex_if_else() {
    check(r#"
            jump __0 greaterThanEq a b
            jump ___1 lessThan b c
        __0:
            print b
            jump ___0 always 0 0
        ___1:
            print a
        ___0:
            end
    "#, expect![[r#"
          1/30  limite        6 -> 6        <12.71568 $ 25.50000>
          2/30  limite       10 -> 10       <5.30000 $ 25.50000>
          3/30  limite       10 -> 10       <5.30000 $ 25.50000>
        -- Early Reconstruction Completed

        #---------- reduce[0/0] case 0 <5.3> ----------
        if a >= b || b >= c {
            print b;
        } else {
            print a;
        }
        end;
    "#]]);
}

#[test]
fn complex_full_test() {
    check(r#"
        :0:
            set links @links
            jump :5 greaterThanEq links 2
        :2:
            wait 0.2
            set links @links
            jump :2 lessThan links 2
        :5:
            set unit_type @flare
            set approach_range 3
            getlink sorter 0
            sensor my_item sorter @config
            sensor sorter_ty sorter @type
            op notEqual is_invert sorter_ty @sorter
            jump :0 strictEqual my_item null
            op add __1 __0 1
            op mod __0 __1 5
            op mul __7 __0 2
            op add @counter @counter __7
            set my_unit __2
            jump :36 always 0 0
            set my_unit __3
            jump :36 always 0 0
            set my_unit __4
            jump :36 always 0 0
            set my_unit __5
            jump :36 always 0 0
            set my_unit __6
            jump :36 always 0 0
            jump :36 always 0 0
        :27:
            ubind unit_type
            jump :31 strictEqual @unit null
            sensor __12 @unit @controlled
            jump :35 equal __12 false
        :31:
            ubind unit_type
            jump :31 strictEqual @unit null
            sensor __13 @unit @controlled
            jump :31 notEqual __13 false
        :35:
            set my_unit @unit
        :36:
            sensor __14 my_unit @dead
            jump :27 notEqual __14 false
            sensor __11 my_unit @controller
            jump :41 equal __11 @this
            jump :27 notEqual __11 @unit
        :41:
            sensor __15 my_unit @controlled
            jump :27 equal __15 @ctrlPlayer
            op mul __16 __0 2
            op add @counter @counter __16
            set __2 my_unit
            jump :55 always 0 0
            set __3 my_unit
            jump :55 always 0 0
            set __4 my_unit
            jump :55 always 0 0
            set __5 my_unit
            jump :55 always 0 0
            set __6 my_unit
            jump :55 always 0 0
        :55:
            ubind my_unit
            sensor unit_item @unit @firstItem
            sensor unit_item_cap @unit @itemCapacity
            jump :60 equal unit_item null
            jump :95 notEqual unit_item my_item
        :60:
            jump :75 strictEqual unit_item null
            jump :64 equal is_invert false
            ulocate building core false 0 __18 __19 0 __17
            jump :72 always 0 0
        :64:
            jump :69 equal links 2
            op rand __21 links 0
            op max __20 1 __21
            getlink __17 __20
            jump :70 always 0 0
        :69:
            getlink __17 1
        :70:
            sensor __18 __17 @x
            sensor __19 __17 @y
        :72:
            ucontrol approach __18 __19 approach_range 0 0
            ucontrol itemDrop __17 unit_item_cap 0 0 0
            jump :0 always 0 0
        :75:
            jump :78 notEqual is_invert false
            ulocate building core false 0 __24 __25 0 __23
            jump :92 always 0 0
        :78:
            set i 1
            jump :88 greaterThanEq i links
            getlink __23 i
            sensor __26 __23 my_item
            jump :88 notEqual __26 0
        :83:
            op add i i 1
            jump :88 greaterThanEq i links
            getlink __23 i
            sensor __27 __23 my_item
            jump :83 equal __27 0
        :88:
            jump :90 notEqual i links
            getlink __23 1
        :90:
            sensor __24 __23 @x
            sensor __25 __23 @y
        :92:
            ucontrol approach __24 __25 approach_range 0 0
            ucontrol itemTake __23 my_item unit_item_cap 0 0
            jump :0 always 0 0
        :95:
            jump :98 notEqual is_invert false
            ulocate building core false 0 __30 __31 0 __29
            jump :112 always 0 0
        :98:
            set i 1
            jump :108 greaterThanEq i links
            getlink __29 i
            sensor __32 __29 my_item
            jump :108 notEqual __32 0
        :103:
            op add i i 1
            jump :108 greaterThanEq i links
            getlink __29 i
            sensor __33 __29 my_item
            jump :103 equal __33 0
        :108:
            jump :110 notEqual i links
            getlink __29 1
        :110:
            sensor __30 __29 @x
            sensor __31 __29 @y
        :112:
            ucontrol approach __30 __31 approach_range 0 0
            ucontrol itemDrop @air unit_item_cap 0 0 0
    "#, expect![[r#"
          1/30  limite       57 -> 57       <377.60001 $ 476.79205>
          2/30  limite     1548 -> 902      <350.20001 $ 406.92499>
          3/30  limite    23695 -> 902      <337.20001 $ 370.98132>
          4/30  limite    25346 -> 901      <325.05209 $ 353.01599>
          5/30  limite    21030 -> 902      <318.05206 $ 331.99545>
          6/30  limite    22447 -> 904      <311.05206 $ 319.78848>
          7/30  limite    21220 -> 905      <304.70206 $ 309.03601>
          8/30  limite    20194 -> 901      <297.80206 $ 300.95822>
          9/30  limite    21277 -> 903      <291.45206 $ 294.16626>
         10/30  limite    21111 -> 904      <285.10208 $ 287.63171>
         11/30  limite    20480 -> 902      <278.75208 $ 281.31677>
         12/30  limite    19518 -> 905      <272.76825 $ 275.18906>
         13/30  limite    18534 -> 902      <267.11679 $ 269.24423>
         14/30  limite    17392 -> 902      <261.46530 $ 263.44507>
         15/30  limite    16269 -> 906      <255.81384 $ 257.85956>
         16/30  limite    15160 -> 904      <250.24289 $ 252.44095>
         17/30  limite    13946 -> 903      <244.59142 $ 247.22345>
         18/30  limite    12683 -> 901      <238.93996 $ 242.09082>
         19/30  limite    11389 -> 901      <233.42990 $ 237.15161>
         20/30  limite    10178 -> 901      <227.71753 $ 232.35634>
         21/30  limite     9040 -> 902      <222.06607 $ 227.86148>
         22/30  limite     8063 -> 901      <217.51462 $ 223.97934>
         23/30  limite     7027 -> 903      <214.27991 $ 221.48834>
         24/30  limite     5608 -> 902      <211.72662 $ 220.09903>
         25/30  limite     4567 -> 901      <211.72662 $ 219.91142>
         26/30  limite     4438 -> 902      <211.72662 $ 219.78802>
         27/30  limite     4379 -> 902      <211.72662 $ 219.76758>
         28/30  limite     4395 -> 902      <211.72662 $ 219.76758>
         29/30  limite     4395 -> 902      <211.72662 $ 219.76758>
        -- Early Reconstruction Completed

        #---------- reduce[5/11] case 0 <167.07674> ----------
        :_0
        do {
            set links @links;
            while links < 2 {
                wait 0.2;
                set links @links;
            }
            set unit_type @flare;
            set approach_range 3;
            getlink sorter 0;
            sensor my_item sorter @config;
            sensor sorter_ty sorter @type;
            op notEqual is_invert sorter_ty @sorter;
        } while my_item === null;
        op add __1 __0 1;
        op mod __0 __1 5;
        gswitch __0 {
        case 0:
            set my_unit __2;
            goto :_3;
        case 1:
            set my_unit __3;
            goto :_3;
        case 2:
            set my_unit __4;
            goto :_3;
        case 3:
            set my_unit __5;
            goto :_3;
        case 4:
            set my_unit __6;
            goto :_3;
        }
        while({
            :_3
            sensor __14 my_unit @dead;
        } => __14 != false) {
            :_4
            ubind unit_type;
            if @unit !== null {
                sensor __12 @unit @controlled;
                goto :_6 __12 == false;
            }
            do {
                do {
                    ubind unit_type;
                } while @unit === null;
                sensor __13 @unit @controlled;
            } while __13 != false;
            :_6
            set my_unit @unit;
        }
        sensor __11 my_unit @controller;
        goto :_4 __11 != @this && __11 != @unit;
        sensor __15 my_unit @controlled;
        goto :_4 __15 == @ctrlPlayer;
        gswitch __0 {
        case 0:
            set __2 my_unit;
            break _;
        case 1:
            set __3 my_unit;
            break _;
        case 2:
            set __4 my_unit;
            break _;
        case 3:
            set __5 my_unit;
            break _;
        case 4:
            set __6 my_unit;
            break _;
        }
        ubind my_unit;
        sensor unit_item @unit @firstItem;
        sensor unit_item_cap @unit @itemCapacity;
        goto :_10 unit_item != null && unit_item != my_item;
        if unit_item !== null {
            if is_invert != false {
                ulocate building core false 0 __18 __19 0 __17;
            } else {
                if links != 2 {
                    op rand __21 links 0;
                    op max __20 1 __21;
                    getlink __17 __20;
                } else {
                    getlink __17 1;
                }
                sensor __18 __17 @x;
                sensor __19 __17 @y;
            }
            ucontrol approach __18 __19 approach_range 0 0;
            ucontrol itemDrop __17 unit_item_cap 0 0 0;
            goto :_0;
        }
        if is_invert == false {
            ulocate building core false 0 __24 __25 0 __23;
        } else {
            set i 1;
            if i < links {
                getlink __23 i;
                sensor __26 __23 my_item;
                if __26 == 0 {
                    do {
                        op add i i 1;
                        break i >= links;
                        getlink __23 i;
                        sensor __27 __23 my_item;
                    } while __27 == 0;
                }
            }
            if i == links {
                getlink __23 1;
            }
            sensor __24 __23 @x;
            sensor __25 __23 @y;
        }
        ucontrol approach __24 __25 approach_range 0 0;
        ucontrol itemTake __23 my_item unit_item_cap 0 0;
        goto :_0;
        :_10
        if is_invert == false {
            ulocate building core false 0 __30 __31 0 __29;
        } else {
            set i 1;
            if i < links {
                getlink __29 i;
                sensor __32 __29 my_item;
                if __32 == 0 {
                    do {
                        op add i i 1;
                        break i >= links;
                        getlink __29 i;
                        sensor __33 __29 my_item;
                    } while __33 == 0;
                }
            }
            if i == links {
                getlink __29 1;
            }
            sensor __30 __29 @x;
            sensor __31 __29 @y;
        }
        ucontrol approach __30 __31 approach_range 0 0;
        ucontrol itemDrop @air unit_item_cap 0 0 0;
    "#]]);
}

#[test]
fn bang_style_quote() {
    check(r#"
        set a "x"
        set x-y @foo-bar
        set y_z @xxx
        op add x-y y-z z-t
        print x-y
        jump 0 equal x-y "m"
    "#, expect![[r#"
          1/30  limite        2 -> 2        <6.55000 $ 12.50000>
          2/30  limite        2 -> 2        <6.55000 $ 12.50000>
        -- Early Reconstruction Completed

        #---------- reduce[0/0] case 0 <6.55> ----------
        do {
            set a "x";
            set 'x-y' @foo-bar;
            set y_z @xxx;
            op add 'x-y' 'y-z' 'z-t';
            print 'x-y';
        } while 'x-y' == "m";
    "#]]);
}
