新的部分
===
使用Args, 来完成Other 还有take传参等

使用星号语法来进行参数展开, 同时增加块循环语法, 会循环指定个参数进行展开

块循环
```
{
    setargs a b c;
    *{
        print *;
    }
}
# inline { print a; print b; print c; }

{
    setargs a b c;
    2*{
        print *;
    }
}
# inline { print a b; print c; }
```

做take参数
```
setargs a b c;
take[* d] Foo;
take Foo[* d];
# take Foo[a b c d];
```

---

添加内部语句setargs, 将由它进行传参(take等)
它将会独立构建一个`Option<usize>`, 同步到每层Expand中.

其中的usize是参数数目, 例如参数数目为3意味着在同一层中`_0 _1 _2`被const

当需要进行调用参数获取时会将栈中最后一个非空的获取
在最顶层时被初始化为Some(0)

当Option为空时意味着该层Expand没有进行setargs

---

添加match语句, 大致用途为根据值(多数为常量替换后的)进行条件编译

用法有两种
1. match未传入参数, 使用最顶层的setargs
2. match传入了参数, 使用参数进行匹配

匹配模式的语法为:
```
PatternAtom = "_" | Var | (Var ":")? "[" Value ("|" Value)* "]"
PatternAtom = {
    "_",                                    // unamed always pattern
    Var,                                    // named always pattern
    (Var ":")? "[" Value ("|" Value)* "]",  // optional named value pattern
}
Pattern = {
    PatternAtom+,                   // full
    PatternAtom+ "*",               // head
    "*" PatternAtom+,               // tail
    PatternAtom+ "*" PatternAtom+,  // head and tail
    ()                              // empty
    "*",                            // always
}
Match = "match" Value* "{" (Pattern Expand)+ "}"
```

它可以是一个通配模式, 

匹配模式示例
```
a           # all to a
a b         # all to a, all to b
[a] b       # a, all to b
[a|b] c     # a or b, all to c
a:[a|b]     # a or b to a
a * b c     # all to a, ..., all to b, all to c
```

匹配模式后面接上一个块,
意味着当模式匹配上时需要展开的代码, 但实际编译为inline-block.

```
match {
    [`true`] Val {
        print "tag is ture, value: " Val;
    }
    Tag:[`false`|`null`] Val {
        print "tag is false or null, value: " Val ", tag: " Tag;
    }
}
```

```
const Foo = 2;
match Foo {
    [`1`] {
        print "one";
    }
    [`2`] {
        print "two";
    }
    Num {
        print "unknown number: " Num;
    }
}

```
以上代码理应展开为 `inline { print "two"; }`
