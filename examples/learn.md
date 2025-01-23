# 引言
Bang 语言是为了在零开销的情况下, 快捷编写和封装抽象逻辑语言而诞生的语言

整体是基于逻辑语言本身的风格进行扩展的语言, 可能有些怪异与主流语言不太一样

操作重点在于编译期对代码的操作, 可以较为灵活

最为基本的可以避免到处使用行号跳转和标签跳转,
可以在构建时将`if` `while`等语句转换成`goto`, 不必自己手动编写


基本单元
===============================================================================
Bang 语言主要基于两种基本单元:

1. Value 值, 可快捷的进行传递、追溯(follow)、求值(take),
   有许多种, 最基础的就是 Var, 下文有讲
2. Statement (语句)[^1], 作为编译时转换成逻辑行的基本单元,
   最常见的可以用多个值组成一个语句, 基本就像在逻辑语言中直接编写一样

[^1]: 这也被称作 LogicLine (逻辑行),
      但是发展到现在其作用已经不适合使用这个命名了

最基础的 Statement, 也就是上文提到的以多个值(Value)组成的, 通常首个值为 Var

```
read foo cell1 15;
```

例如上述代码由四个值组成, 每个值都由 Var 构成,
分别是`read` `foo` `cell1` `15`, 三个逻辑变量和一个数字.

> [!TIP]
> 别被逻辑变量这个名字迷惑, 逻辑中不用作变量的东西比如上面的`read`虽然表示命令,
> 但是和变量长得一样, 所以 Bang 对其一视同仁

> [!NOTE]
> 分号是必要的, Bang 作为一个空白符号无关语言, 空白符不会影响其语法,
所以最好有一个明确的分隔符号来分隔


简单介绍 Value (值)
===============================================================================
值包含很多种, 在这里简单介绍较为基础与常用的几种


Var (量、变量)
-------------------------------------------------------------------------------
Var 指的是逻辑语言中一切的 logic-value 逻辑值,
也基本就是所有逻辑中可以用作字面量的部分, 如:

- 数字 `1` `1.25` `0x1f` `0x-3e`
- 字符串 `"test"`, 对于原生逻辑并不严格的反斜杠,
  Bang 的字符串中反斜杠转义会方便些, 可以使用反斜杠转义反斜杠、方括号,
  具体参见[多行字符串](./mult_line_string.mdtlbl)
- 逻辑变量 `foo` `a-b` `@copper` `true` `null` `let's`

> [!IMPORTANT]
> 需要注意的是, 以上的逻辑变量并不完全在 Bang 适用, 比如 `a-b` 和 `let's`,
> 逻辑变量过于自由, 除了某几个符号无法使用, 剩下的符号都可以拼在一起组成变量
>
> 如果 Bang 也完全使用逻辑格式将会很不方便, 所以 Bang 对逻辑变量的形式做了缩减,
> 依照常见编程语言的形式使用 unicode-xid, 所以可以支持许多中文变量名
>
> 由一个 (xid-start 或下划线) 和若干个 xid-continue 组成一个普通的变量,
> 如果在前面加上`@`符号, 后面若干个 xid-continue 还允许短横线,
> 用于逻辑常用的一些环境变量

常见的数字形式:

- 整数或浮点数: `123` `1_000_000` `1.29` `1e4` `-6`
- 进制数: `0x1f` `0b1001` `0x-2`

> [!NOTE]
> 注意, Bang 支持数字中加下划线来增加可读性,
> 不过除了这个基本全部是原样转换到逻辑语言的,
> 也就是逻辑语言本来就支持的形式`1e4`,
> 所以逻辑语言不支持的小数形式科学计数法就没进行支持了, 也就是如`1.2e3`

但是显然以上的三种并不能满足全部需求, 所以 Bang 还额外进行支持了一种万能格式

由单引号括起来的任意非空白或单引号符号, 将会组成一个 Var,
其中的双引号被转换成单引号, 因为逻辑语言本身就不允许双引号组成,
所以这种格式可以表示任意逻辑语言的变量, 例如上面举例不支持的格式:

```
set a 'a-b';
set b 'let"s';
```
编译为
```
set a a-b
set b let's
```

> [!WARNING]
> 注意不要使用一些特殊的符号, 虽然 Bang 支持但是逻辑里面有其它含义的符号,
> 但是编译到逻辑时逻辑就解析不了了
>
> 例如`#`在逻辑语言里面也是注释, `;`在逻辑语言里面也用于分隔语句


DExp (可译做依赖表达式)
-------------------------------------------------------------------------------
这也是一种值(Value), 意义是表示一个 Var,
但是这个 Var 的成立依赖于某些语句, 比如返回的 Var 是一个逻辑变量,
而依赖的语句给它赋值, 使这个逻辑变量成立,
可以在其开头使用一个 Var 接上冒号手动指定要返回的 Var

```
set a 1;
set b 2;
print (foo:
    foo = a+b;
);
```
编译为
```
set a 1
set b 2
op add foo a b
print foo
```

可以看到, DExp 作为一个值,
在求值时总是将其中包含的语句都编译后再将其自身的 Var 返回

**每个 Value 求值总是会返回一个 Var**


ResultHandle (返回句柄[^4])
-------------------------------------------------------------------------------
这也是一种值, 在 DExp 中使用,
代表的是当前 DExp 中要返回的那个 Var, 通常是方便对其赋值而使用的, 写法是`$`

还是以上面 DExp 的例子举例

```
set a 1;
set b 2;
print (
    $ = a+b;
);
```
和 DExp 示例中的不同, 这次我们没有手动指定返回的 Var,
所以编译器将随机生成一个 Var 来表示这个 DExp 的返回 Var,
但是这样随机生成的 Var 我们就不知道它叫什么了, 就要使用返回句柄`$`来引用它

编译为

```
set a 1
set b 2
op add __0 a b
print __0
```

可以看到, 生成了一个叫 `__0` 的变量来表示这个 DExp 的返回值

> [!NOTE]
> 尽量不要在变量中使用双下划线, 这是编译器内部约定使用的格式,
> 手动使用可能造成冲突


ReprVar (原始量)
-------------------------------------------------------------------------------
在之后的常量系统里面会细讲, 写法是普通的量包裹一层反引号

例如:
```
`read` result cell1 0;
```


ValueBind (值绑定)
-------------------------------------------------------------------------------
用于将一个量绑定到一个值的求值结果上, 也就是这两个量,
并生成对另一个量的唯一对应关系

只要绑定和被绑定的量在两次使用中一致, 那得到的绑定结果也会一致(仅单次编译中)

例如:
```
foo = 2;
foo.x = 3;
foo.y = 4;

print foo", "foo.x", "foo.y;
```
编译为
```
set foo 2
set __0 3
set __1 4
print foo
print ", "
print __0
print ", "
print __1
```

这主要是在常量系统中大量应用, 可以在求值过程中传递值而不仅仅是量,
并将多个值可以方便的单次传递


[^4]: 句柄, 通常指被求值后产生的 量(Var)


初探常用控制流
===============================================================================
在 Bang 中, 有许多方便的控制语句可以使用, 如if while等

> [!TIP]
> 当然也可以使用原本逻辑的 jump, 不过在 Bang 中, 它被改名为 goto,
> Bang 中的标签也从冒号结尾改成了冒号开头.

```
set a 1;
:x
set b 2;
goto :x a < b; # 因为直接跳转回了标签x, 所以执行不到下方赋值的地方
set 无法到达 3;
```
编译结果如下
```
set a 1
set b 2
jump 1 lessThan a b
set 无法到达 3
```
可以看出, 简单的 goto 使用直接被编译为相同的 jump,
大部分之后要介绍的控制流语句都是要编译成 goto 的

[^2]: 这是逻辑语言的程序计数器, 用来指示某行执行完毕后将要执行哪行,
      可以在某行更改它以调整接下来要执行哪些代码, 灵活度很高


条件分支 (if elif else skip)
-------------------------------------------------------------------------------
`if` 是使用条件来进入某个分支的控制语句, 表示条件成立则执行

`else` 表示"否则", 如果上述条件不成立则执行`else`的代码

`elif` 是`else if`的类似物, 虽然结构上有些不同, 以前有必要现在只是写着方便

```
if a < b {
    print "less than";
} elif a > b {
    print "greater than";
} else {
    print "equal";
}
printflush message1;
```
编译结果如下
```
jump 6 lessThan a b
jump 4 greaterThan a b
print "equal"
jump 7 always 0 0
print "greater than"
jump 7 always 0 0
print "less than"
printflush message1
```

还有`skip`语句, 用法和`if`类似, 但是不会有`else`,
简单的展开为一个满足条件则跳过某段代码的`goto`


循环条件 (while gwhile do-while)
-------------------------------------------------------------------------------
循环, 通常用于满足某个条件则重复执行某段代码

- while: 满足某个条件就重复执行某段代码, 直到条件不再被满足
- gwhile: 类似 while, 但是 while 是在头部重复一个相反的条件在首次不满足时跳过,
  gwhile 的头部直接跳转到末尾条件部分, 这会在进入循环时多执行一行,
  但是大型条件等一些情况可以让代码简化, 因为只需要生成一份条件跳转而不是两份
- do-while: 类似 while, 但是总是会执行至少一遍, 然后再用条件判断是否重复执行,
  非常简单, 只是在尾部简单的生成出一个跳转到头部的 goto


```
print "while";
while i < 2 { print 1; }
print "gwhile";
gwhile i < 2 { print 1; }
print "do-while";
do { print 1; } while i < 2;
end;
```
构建[^3]为以下的类似产物
```
print "while";
{
    goto :___0 i >= 2;
    :___1
    {
        `'print'` 1;
    }
    goto :___1 i < 2;
    :___0
}
print "gwhile";
{
    goto :___2;
    :___3
    {
        `'print'` 1;
    }
    :___2
    goto :___3 i < 2;
}
print "do-while";
{
    :___4
    {
        `'print'` 1;
    }
    goto :___4 i < 2;
}
end;
```


[^3]: Bang 主要流程分为两个阶段, 构建时(Build time) 和 编译时(Compile time),
      构建时将一些简单固定的东西展开, 比如if while等, 还有收集一些标签和标签绑定等,
      编译时处理更为复杂的东西, 常量、作用域、参数系统、追溯、求值等
      通常使用`c`选项来一次性构建与编译, 或者使用`A`选项来观察构建阶段的细节


循环内控制流 (break continue)
-------------------------------------------------------------------------------
在循环内(while gwhile do-while select switch gswitch)中,
可以使用`break` 或 `continue`语句,
直接跳出循环或跳到新一轮循环

```
i = 0; do {
    if (op mod $ i 2;) == 0 {
        op add j j 1;
        break i > 6;
    }
    op add i i 1;
} while i < 10;
```
比如上面这份示例代码, 编译为标签形式可以直接看到其作用
```
___0:
    set i 0
___2:
    op mod __0 i 2
    jump ___1 notEqual __0 0
    op add j j 1
    jump ___0 greaterThan i 6
___1:
    op add i i 1
    jump ___2 lessThan i 10
```

> [!NOTE]
> 在select switch gswitch中, continue是跳转到整个的头部,
> 而不在循环中时, break跳转到整个代码的尾部, continue跳转到整个代码的头部,
> 而这两种基本上没有区别


控制块
-------------------------------------------------------------------------------
可以使控制块里的`continue`和`break`指向控制块头部和末尾,
也可以加上叹号将含义反过来, 如果只加其中一个则不会影响另一个

```
print "begin";
break continue {
    break;
    continue;
}
print "split";
break! {
    break;
    continue;
}
print "end";
```
构建为
```
`'print'` "begin";
{
    {
        :___1
        {
            goto :___0 _;
            goto :___1 _;
        }
    }
    :___0
}
`'print'` "split";
{
    :___2
    {
        goto :___2 _;
        goto :___3 _;
    }
}
`'print'` "end";
:___3
```

整数分支结构 (select switch gswitch)
-------------------------------------------------------------------------------
这类结构通过动态的一个整数来选择第i块代码(从第0块开始), 原理依赖于`@counter`[^2]

注意不要输入非整数或者小于0的数, 或者不小于代码块数的数

`select`和`switch`会以得到优秀实践的两种形式之一展开, 区别在于两者代码量不同,
一个需要填充对齐语句块, 另一个需要构造一张跳转表, 会自动选取代码量少的形式构造

```
select i {
    print 0;
    print 1;
    print 2 2;
}
print "...";
select i {
    print 0;
    print 1 1;
    print 2 2;
}
```
比如以上代码编译为标签形式方便查看
```
    op add @counter @counter i
    jump __0 always 0 0
    jump __1 always 0 0
    jump __2 always 0 0
__0:
    print 0
__1:
    print 1
__2:
    print 2
    print 2
    print "..."
    op mul __0 i 2
    op add @counter @counter __0
    print 0
    jump __3 always 0 0
__3:
    print 1
    print 1
    print 2
    print 2
```
可以看出, 首先上面那种选择了跳转表形式生成, 下面那种选择了对齐块形式生成,
且没手动加上跳出的话, 会跳转到第i块, 然后以第i块开始将大于i的所有块都执行一遍

通常我们并不需要这样, 而是执行某一块结束后, 直接跳出整个部分,
所以可以使用`select`的进阶, `switch`

除了使用`case`来分隔每一块代码而不是每个 Statement 一块代码外,
还可以将每个case前的代码附加到每块`case`的最后, 称为 switch-append

```
switch i {
    break;
case: print 0;
case: print 1;
case: print 2 2;
}
```
我们可以用`A`选项来看到(`switch` 在构建期就会展开为 `select`)
```
{
    select i {
        {
            `'print'` 0;
            goto :___0 _;
        }
        {
            `'print'` 1;
            goto :___0 _;
        }
        {
            {
                `'print'` 2;
                `'print'` 2;
            }
            goto :___0 _;
        }
    }
    :___0
}
```
从展开后的代码可以看出`break`展开的`goto`被附加到了每个`case`最后

同时`switch`也可以不用使用空语句占位, 可以直接指定某个整数
```
switch i {
    break;
case 2:
    print 2;
case 4:
    print 4;
}
```
构建为
```
{
    select i {
        {} # ignore line
        {
            goto :___0 _;
        }
        {
            `'print'` 2;
            goto :___0 _;
        }
        {
            goto :___0 _;
        }
        {
            `'print'` 4;
            goto :___0 _;
        }
    }
    :___0
}
```
由其上也能观察出, 附加的`break`也会被附加在连续的空块最后(对于0,1, 只在1处有)

`gswitch`和`switch`没多少不同, 区别只在于其运行在编译期而不是构建期,
这可以拥有更多的高级操作, 不过它只会构建为跳转表形式而不是填充块形式,
但这样也让它可以方便的让不同的代码块指向同一份代码, 而不需要将代码块重复一份


简单条件 (CmpAtom)
-------------------------------------------------------------------------------
主要由可以在单条逻辑的 `jump` 语句中表示出来的条件组成,
作为复合条件中最小的单元出现, 以下列举其算符形式

- `_`: 无条件永远成立
- Never: 无条件永不成立
- `<` `>` `<=` `>=`: 基本的大小比较
- `==` `!=`: 基本的相等和不相等比较
- `===` `!==`: 基本的严格相等和严格不等比较

> [!NOTE]
> `!==` 是 Bang 额外扩展的算符, 在逻辑中并不存在,
> 如果最终用它生成逻辑代码, 会额外使用一条 op 语句然后再反转其结果,
> 所以需要注意
>
> Never 是 Bang 额外扩展的算符, 甚至你都无法在 简单条件 中编写出来,
> 但是它可以出现, 通过复合条件对 `_` 反转得到, 如 `!_`


复合条件 (CmpTree)
-------------------------------------------------------------------------------
简单单一的条件通常无法满足需求, 所以可以使用复合条件来将多个简单条件组合

复合条件通常使用三种运算符来组织

| 示例                      | 优先级      | 结合性      | 命名    |
| ---                       | ---         | ---         | ---     |
| `!a < b`                  | 4           | 右结合      | CmpNot  |
| `a && b`                  | 3           | 左结合      | CmpAnd  |
| `a \|\| b`                | 2           | 左结合      | CmpOr   |
| `({print 2;} => a < b)`   | 1           | 右结合      | CmpDeps |

也可以使用括号来手动规定比如`(a < 2 || b < 2) && c < 2`

> [!NOTE]
> CmpDeps, 可以在比较某个条件前展开某些代码,
> 和 DExp 有类似作用, 不过它的优先级是溢出的, 在许多地方需要加上括号使用
>
> `!` 运算并不实际存在, 它使用德摩根变换来反转内部条件, 直到反转到简单条件后结束


简单语句
===============================================================================
较为简单的语句

- noop: 逻辑中通常无法手动打出的语句, 显示为"Invalid", 解析失败的语句也会产出它
- op: 符合逻辑风格的运算语句
- print: 方便的展开为多个逻辑使用的`print`, 这样可以将内容贴在一起写很方便,
  例如: `print "foo: "foo", bar: "bar"\n";`
- 展开块 (Expand): 通常也被称作块,
  可以在其中包含多个语句, 通常用在循环等的后面, 例如:
  ```
  {
      print 1;
      print 2;
  }
  ```
- 内联块 (Inline Block): 类似展开块, 也可以在其中编写多个语句,
  但是不像 Expand 携带一个 Expand 的作用域, 它没有作用域,
  通常手动使用它用处不大, 写做 `inline {}`, 就是普通 Expand 前面加一个`inline`
- 标签 (Label): 用于被跳转的标签, 格式是一个冒号一个跟在后面的 Var
- 其它语句 (Other): 就是上文所提到的由多个 Value 组成的普通逻辑语句


运算和比较的逻辑风格兼容
-------------------------------------------------------------------------------
对于 CmpTree, 和 op 语句, 有做对于逻辑语言风格的兼容, 可以使用逻辑的风格来编写

例如以下的每个 skip 的条件都是相同的, 都可以经过编译

```
skip a < b print 2;
skip < a b print 2;
skip a lessThan b print 2;
skip lessThan a b print 2;
```

同样的, 以下的 op 也是产生相同的效果, 且可以正确编译
```
op add a a 1;
op a a add 1;
op + a a 1;
op a a + 1;
```

```
op floor r n 0; # 这无用的参数0不会被求值
op r floor n 0;
op floor r n;
op r floor n;
```

虽说这个风格兼容并没有什么用, 未来还可能删除, 不过或许有些人喜欢


简化运算 - 运算表达式 (op-expr)
-------------------------------------------------------------------------------
这让人们可以以传统的优先级、方便的形式不必在复杂的数学运算中编写原始的DExp形式
可以极大的降低心智负担

```
i, x = 2, abs(a-b) + sqrt(a)*2;
```
如果没有 op-expr, 我们将要编写以下构建形式, 将会非常地狱
```
{
    `set` i 2;
    op x (op $ abs (op $ a - b;);) + (op $ (op $ sqrt a;) * 2;);
}
```

同时也提供三元运算等, 详见 [op-expr](./op_expr.mdtlbl)


关于注释
===============================================================================
Bang 的注释基于逻辑语言进行扩展, 但是还添加了一种新的形式, `#*`开始直到`*#`的内容将会忽略,
可以跨越多行使用, 不必每行都添加注释符号,

当然为了习惯或美观还经常添加一个`*`

```
# 这是一个注释
set a 不是注释;
#* 这是一个多行注释
多行注释中
* 多行注释中
*# set b 不是注释;
set c 不是注释;
```
编译为
```
set a 不是注释
set b 不是注释
set c 不是注释
```


> 逻辑语言使用`#`符号进行注释, 将会忽略从`#`符号开始直到行末尾的内容


进阶入门 - Bang 语言的常量系统
===============================================================================
Bang 语言提供了一套非常强大的常量系统, 以实现元编程, 可以灵活操作代码,
以满足大部分逻辑的需要

最为基本的, 还记得在介绍值的时候说的"传递、追溯、求值"吗?
下面介绍一个核心语句: `const`

`const`语句最基础的用法可以将一个值绑定到一个量上面,
然后接下来这个量会在当前 Expand 及其子 Expand 内(简称作用域)生效,
当在生效范围内对相同的量进行追溯或求值时, 将会做*一些额外的操作*后,
将原本操作的 Var 转换成对其绑定到的值的操作

> [!WARNING]
> DExp 中也隐含了一层 Expand, 请不要忽视

```
const A = 2;
print A;
```
编译为
```
print 2
```
可以理解为, 直接将作用域内, 被 const 的值直接粘贴过来了,
但是别忽略那*一些额外的操作*, 当然现在可以先不管

同一个 Expand 中重复的对一个量进行 const 的话, 那么旧的将会被覆盖, 例如:
```
const A = 2;
const A = 3;
print A;
```
编译为
```
print 3
```


遮蔽 (shadow)
-------------------------------------------------------------------------------
如果是在子 Expand 中进行 const 的话, 那么在子 Expand 的范围内,
外部 const 的值将被遮蔽(shadow)

不管是求值还是追溯, 总是获取最内层里层的 const 值,
而内层 Expand 结束后, 外层 Expand 中 const 的值并没有改变,
只是被里层的同名量遮住了

```
const A = 2;
{
    const A = 3;
    print A;
}
print A;
```
编译为
```
print 3
print 2
```

可以看出, 子 Expand 发生遮蔽时, 父(外部) Expand 中 const 的值并没有被覆盖,
在子 Expand 结束后依旧可以被使用


追溯 (follow)
-------------------------------------------------------------------------------
这是让 const 不至于变得彻底混乱而在低版本增加的一个核心机制, 在 const 一个值时,
会对这个值进行**一次**追溯, 而对于最常见的 Var 的追溯则类似求值时, 就是查询 const

基于以上的描述, 我们可以写出以下代码
```
const A = 1;
const B = A;
const A = 2;
print B;
```
会编译出
```
print 1
```
而不是(*在远古版本中真的是*)
```
print 2
```

这也构成了值的基本传递能力

这是一份大致的追溯行为表, 没有提到的追溯结果都是其自身:

- Var: 查询常量表
- ReprVar: 解包原始量, 也就是`` `X` ``变成`X`, 且因为追溯进行一次,
  所以常量表内肯定不会有原始量
- ValueBindRef (`->$`): 在追溯情况对被绑定值求值
- ValueBindRef (`->..`): 替换成常量表中被绑定值的绑定者
- ValueBindRef (`->Name`): 求值被绑定值, 并按正常值绑定规则替换成绑定者,
  区别在于这运行在追溯而非求值时
- Closure (闭包): 捕获追溯或求值其声明的值

> [!NOTE]
> 对于常量表查询, 如果没有查询到, 那么将返回其自身,
> 这非常常见, 例如 `read result cell1 0;`
> 这普通的一句里面就有四次未查询到返回自身


求值 (take)
-------------------------------------------------------------------------------
求值将任意的值转换成一个固定的量, 最为常见的就是 DExp,
将其中代码生成后转换为它的句柄.

- Var: 其求值行为就是进行一次追溯, 如果结果不是一个 Var, 那么对其继续求值
- ReprVar: 直接将包裹的原始量作为结果
- DExp: 前面已经有说明了
- ValueBind (值绑定): 这是将一个量绑定到一个值上,
  求值时会把这个值求值为被绑定量,
  然后将绑定量和被绑定量进行绑定表查询到唯一映射量, 最后对这个量进行常量表查询
- ValueBindRef (`->$`): 和被绑定值求值行为相同
- ValueBindRef (`->..`): 和追溯时它的行为相同
- ValueBindRef (`->Name`): 和值绑定求值行为相同
- ClosuredValue: 正常的展开内部值, 但是会先设置闭包捕获的环境,
  详见 [ClosuredValue (闭包值)](#ClosuredValue-闭包值)
- Cmper: 编译错误, 退出编译, 详见 [条件依赖和条件内联](#条件依赖和条件内联)


值绑定 (ValueBind)
-------------------------------------------------------------------------------
这用于往句柄和量的组合上绑定 const

这是在 const 常量系统中经常出现的一个重要的东西,
在前面介绍了 const 可以把一个值绑定到一个量上,
然后使用这个量进行追溯或求值时会替换成操作绑定到这个量上的值,
当然不止一次提到会做一些额外操作不能忽略

而其实, const 还可以将值绑定到*值绑定*上,
流程是先将值绑定的被绑定值进行求值得到一个量(以下称作句柄[^4]),
然后将 句柄 和 值绑定的绑定量 在绑定表查询, 得到它们的映射量.

接着将类似普通的 const 流程, 将值绑定到映射量,
不过这次是在最外层的 Expand 更外一层的作用域中, 当然这影响不大,
因为正常来说你不会得到与映射量相同的量相同产生遮蔽等

这样可以仅传递句柄, 就可以同时传递多个绑定值, 例如以下代码:

```
const myvec.X = 2;
const myvec.Y = 3;
const myvec.Print = (
    print ...X", "...Y;
);

const FooVec = myvec;
print "x: "FooVec.X"\nvec print: ";
take FooVec.Print;
printflush message1;
```
编译为
```
print "x: "
print 2
print "\nvec print: "
print 2
print ", "
print 3
printflush message1
```

在上面那段代码中, 使用到了三个新知识点

1. take: take 是一个常用又简单的语句, 这里用到了一个简单的用法:
   既将给定值直接求值. 和直接写在那用 其它语句 顺带进行求值的方法比较,
   这种可以用在不关心返回句柄的时候使用,
   不然上面代码就会产生用不上的随机生成的 DExp 返回句柄
2. Binder (绑定者): 写做 `..`,  是一种值,
   其求值会展开为当前正在发生的对值绑定求值的被绑定句柄,
   这可以方便的理解为某些语言中的 `this` `self` 等
3. 常量表的绑定者: 常量表并不止被 const 的值, 还包含其它信息,
   在后面有详细介绍

可以看到仅一个 const, 就可以把 X Y Print 的映射关系都传递到了 FooVec,
毕竟求值后都是同一个量


> [!TIP]
> 对于可以被进行 const 的东西, 我们将其称之为 ConstKey,
> 包含了上述提到的 Var 和 ValueBind


Take 语句
-------------------------------------------------------------------------------
在上文中我们使用了 take 语句, 在这里介绍它基本的用法

在上文中, 我们使用了它的一种基本形式, 也就是直接往里面写值,
需要求值但是不关心其返回句柄的时候, 例如如下代码

```
const F = (
    print 2;
);

print "Plan A";
F F;
print "Plan B";
take F F;
```
编译为
```
print "Plan A"
print 2
print 2
__0 __1
print "Plan B"
print 2
print 2
```
可以看出, 使用方案A会在编译结果中得到`__0`和`__1`这两个生成的句柄,
但是我们这次并不需要它们, 所以可以使用 take 语句, 虽然求值, 但是并不管句柄


---
我们有时需要句柄, 但是并不想立即使用, 或者在多处使用只想求值一次的情况,
我们可以使用 take 的另一种形式

```
a, b = 2, 3;
const F = ($ = a + b;);
take Value = F;
add1 = Value + 1;
print "Value: "Value", add1: "add1;
printflush message1;
```
编译为
```
set a 2
set b 3
op add __0 a b
op add add1 __0 1
print "Value: "
print __0
print ", add1: "
print add1
printflush message1
```

`take Value = F;` 其运作原理是, take 先将`F`进行求值, 然后此时拿着得到的句柄,
进行类似 const 的流程, 假设`F`求值的句柄是`X`,
那么此时求值后发生的类似`` const Value = `X`; ``,
`X`使用 ReprVar 因为这并不会对`X`再次追溯


关于 const 时的细节
-------------------------------------------------------------------------------
有一个知识, 就是当你 const 的时候, 并不仅是值被打进了常量表,
其实还有绑定者和标签, 标签是构建时被记录在语句上的, 使用`A`选项也能看到, 例如:

```
const Foo = (
    :foo
    print 1;
    goto :foo;
);
```
构建为
```
const Foo = (
    :foo
    `'print'` 1;
    goto :foo _;
);#*labels: [foo]*#
```
可以从这种形式 const 后面的注释中看到有哪些标签

对绑定者来说, 如果你 const 的值本身追溯的目标就有绑定者,
那么这个绑定者将采用追溯目标的,
否则如果你的 const 绑定到的是一个值绑定, 那么绑定者将会使用值绑定被绑定值的句柄


关于求值时的细节
-------------------------------------------------------------------------------
当你对一个量进行求值且在常量表中追溯到一个值时, 大致会发生以下步骤

1. 设置绑定者为常量表中记录的, 你可以使用`..`进行获取
2. 设置标签重命名为常量表中记录的, 此时定义或使用标签会被重命名,
   这样不会在对一个 DExp 等展开多次的时候, 出现多个同名的内部定义的跳转标签
3. 设置当前展开的名称, 用于调试, 并不需要多关注

标签重命名参考以下代码
```
const Foo = (
    :foo
    goto :foo;
);
take Foo Foo; # 求值两次
```
观察其标签形式(`Li`选项)
```
__0_const_Foo_foo:
    jump __0_const_Foo_foo always 0 0
__1_const_Foo_foo:
    jump __1_const_Foo_foo always 0 0
```
可以看到标签名字并不是`foo`, 而是一长串被重命名的形式,
这样在不同的展开之间定义的标签就不会冲突了
(如果不重命名, 那么就会得到两个foo标签, 那么跳转到foo标签就不知道跳转到哪个了)


编译时运算
===============================================================================
在求值一个值前, 会先尝试一次是否可以进行常量评估, 也就是部分整数的简单运算

以下是一个支持的大致列表:

- 当值是一个 DExp, 且内部仅含有一个 op 或者 set, 且它们的返回值都是 `$`,
  此时会尝试对其参数进行常量评估, 如果所有参数常量评估都成功,
  且自身运算符号也是支持的运算符号, 那么自身也会评估成功, 返回评估结果
  (目前 严格相等、噪声、长度、随机、幅角、幅角差异)不被支持
- 当值是一个 ReprVar, 直接尝试将其内部的量评估为一个普通数字(不能是无穷、非数等)
- 当值是一个量(Var), 且查询常量表找到的符合常量评估, 取常量表内的评估结果
- 当值是一个量(Var), 且不在常量表中, 那么类似 ReprVar, 直接将其评估为一个普通数

例如
```
foo = (1+2+3)*(1*2*3);
```
编译为
```
op mul foo 6 6
```


DExp 必备 - 设置返回句柄 (setres)
-------------------------------------------------------------------------------
setres: 用来设置此语句所在的 DExp 的返回句柄, 例如:

```
print (a: set $ 2; setres b;); # 请不要这么做
```
会编译为
```
set a 2
print b
```

> [!WARNING]
> 以上代码在设置句柄前使用了返回句柄,
> 但是却在使用这个返回句柄后改变了返回句柄
>
> 通常我们是希望返回我们使用的返回句柄的,
> 所以使用 setres 的时候要注意在使用处之前是否有用到返回,
> 不然可能会悄悄的让之前希望使用的句柄作废

最为有用的是你不知道一个值的返回句柄,
但是你想让当前 DExp 额外做某些事的时候保持使用某个未知值的返回句柄, 例如:

```
const BindType = (unused:
    setres _0;
    sensor $.type $ @type;
);
print BindType[(block: getlink $ 0;)].type;
```
编译为
```
getlink block 0
sensor __1 block @type
print __1
```
可以看到, 返回句柄设置成了外部传入的 DExp 的句柄`block`,
且给它附加上`type`的绑定后, 还将它设置成返回句柄返回了

> [!NOTE]
> 用到的`[]`和`_0`都会在之后的[参数系统](#参数系统)中详细讲解


参数系统
===============================================================================
对于每一个正在求值的 Expand, 都可选的可以存储一层参数,
参数是由多个 Var 指向的局部常量组成的, 通常我们可以通过以下几种方法来设置参数

```
const Foo = (
    print @;
);
take["a" "b"] Foo; # 较为古老的方式在 Take 语句上设置
take Foo["c" "d"]; # 较为常用的生成一个用于传参的 DExp 再进行求值, 建议尽快求值

match "e" "f" { @ {} } # 通过匹配语句捕获所有参数来在当前环境中设置
take Foo; # 然后普通的 Take, 使用参数会往外找到首个设置的参数, 就找到了展开前环境的参数
```
编译为
```
print "a"
print "b"
print "c"
print "d"
print "e"
print "f"
```
通常我们会使用后两种方法设置参数, 中间那种最多, match 语句的细节后面章节会讲

关于前两种方法我们可以通过`A`选项查看构建输出, 以探究其具体原理
```
const Foo = (inline 1@{
    `'print'` @;
});
{
    # setArgs "a" "b";
    take __ = Foo;
}
take __ = (__:
    # setArgs "c" "d";
    setres Foo;
);
match "e" "f" {
    @ {}
}
take __ = Foo;
```
首先 Foo 那个`inline 1@`先忽略, 后面的章节会讲

1. `{}` 为一个 Expand, 然后`# setArgs`是一个无法编写的内部语句,
   但是可以构建生成, 用于设置当前 Expand 的参数,
   我们可以看到先使用一个 `{}` 开启了一个 Expand,
   然后使用`# setArgs`语句来设置了这个 Expand 的参数,
   最后再进行了 take, 从而达到被求值的值可以接收到参数的效果
2. 这被称作 **快速 DExp 求值 (Quick DExp Take)**
   可以看到, 它开启了一个 DExp, 利用 DExp 隐含的 Expand 开启了一个 Expand,
   然后设置这个 Expand 的参数, 再使用 setres 语句,
   将我们希望被传参的值进行求值并将返回句柄设置为当前 DExp 的返回句柄

   **从这可以看出, 使用这种形式的时候,
   别在参数里面求值`$`, 不然会得到用于设置参数的这个 DExp 的返回句柄,
   也就是那个没有用的`__`**
3. 这个先忽略, 这会在[匹配与重复块](#匹配与重复块)章节深入描述

> [!TIP]
> 我们可以注意到, `take Foo;` 实际上是展开为了 `take __ = Foo;`,
> 这是随便找了个你肯定不应该使用的变量来接收返回的句柄, 反正你也用不到

---
之前代码 const 中使用的 `@` 符号, 可以让当前环境中的参数在此处展开, 比如:
```
const Foo = (
    print "(" @ ")";
);
take Foo["Hello" "Jack"];
```
编译为
```
print "("
print "Hello"
print "Jack"
print ")"
```
通常我们可以在 Other 语句、参数传递(通常是`[]`)、print 语句中使用

---
`# setArgs` 语句还会进行一种老式的兼容设置, 它同时会进行一系列的 const,
例如之前构建展开中以下代码:

```
{
    # setArgs "a" "b";
    take __ = Foo;
}
```
在之前的版本其实是类似以下这种形式的
```
{
    const _0 = "a";
    const _1 = "b";
    take __ = Foo;
}
```

现在的版本中, `# setArgs` 也同样会设置一遍`_0` `_1`等,
所以也可以使用`_0` `_1`等快捷的使用参数

> [!NOTE]
> 无论是老版本还是现版本, 设置参数或者`_0` `_1`这些时,
> 都不会像直接编写 const 语句那样收集标签
>
> 所以对于要传入被重复展开的地方时, 建议使用 Consted-DExp,
> 在之后的[常用语法糖](#常用语法糖)会讲



匹配与重复块
===============================================================================
在之前介绍了参数系统, 这章来介绍和参数最常搭配的匹配语句和重复块语句

匹配语句, 可以针对参数的多种情况编译不同分支的代码, 并将参数捕获到常量, 例如:
```
const Add = (match @ {
    A B { $ = A + B; }
    Result A B { setres Result; $ = A + B; }
});

print Add[a b];
print Add[x a b];
take Add[c a b];
```
编译为
```
op add __2 a b
print __2
op add x a b
print x
op add c a b
```

这样就非常灵活, 而可以在 match 里面写的匹配都有以下几种:

- `_`: 匹配任意的值
- `A`: 普通的编写一个 Var, 代表匹配任意的值, 并将匹配的值 const 到这个 Var
- `A:[1 2]`: 在方括号中写一些值, 匹配的值必须和这些值的句柄相同,
  其中`A:`部分可省略
- `@`: 在这个位置匹配任意个数的值, 并将匹配到的值进行`# setArgs`,
  每个分支只能存在一个`@`
- 其它匹配前面加上`$`可以方便的将这个匹配到的值进行 setres

match 里面由零至多个匹配后面跟上一个花括号组成, 称作一个分支, 可以有多个分支

而除了普通 match, 还存在一种 const-match,
区别是 match 会将所有参数进行求值后拿其句柄进行匹配,
而 const-match 会将所有参数 const 后拿值去匹配
(这里的 const 类似参数列表, 不会收集标签等)

而 const-match 还多出一些可用的匹配:

- 其它匹配前面加上`*`则使用 take 而不是 const 这个值到要绑定到的常量
- 在方括号匹配里面方括号头部加上`*`则尝试先将待匹配的值进行求值后匹配其句柄
- 在方括号匹配里面方括号头部加上`?`则方括号内输入 op-expr,
  来通过返回0还是1来确定是否匹配

> [!NOTE]
> `$` 和 `*` 同时存在时, `$` 加在 `*` 的前面
>
> 方括号里面使用`*` `?`这些可能会导致反复求值, 不太建议


重复块
-------------------------------------------------------------------------------
重复块可以针对每n个参数, 顺序经过并设置参数(不会设置`_0` `_1`)这类, 例如:

```
match 1 2 3 4 5 6 7 { @ {} } # 小技巧, 利用 match 来模拟 setArgs
inline 3@{
    foo _0 '(' @ ')';
    const X = _0;
}
bar @;
print X;
```
编译为
```
foo 1 ( 1 2 3 )
foo 1 ( 4 5 6 )
foo 1 ( 7 )
bar 1 2 3 4 5 6 7
print 1
```

> [!NOTE]
> 这里的 `foo` 和 `bar` 命令是并不存在的命令, 只是用来直观方便的演示参数情况
>
> 重复块自身包含一个参数作用域, 但是并不包含 Expand 作用域,
> 也不会设置参数时将`_0` `_1`这种进行设置, 比较不同

从编译结果可以看出它的工作原理, `@`符号前面写数量, 数量不足时**依旧会运行**,
如果数量省略不写则默认为`1`

通常重复块也配合着 match 一起使用, 来取出参数内容, 或者仅关心参数足够时的情况


常用语法糖
===============================================================================
在这里介绍一些常用的语法糖, 语法糖大致表示用更简洁或更方便的方式写出来,
但是构建后基本是一样的结果的语法, 通常是为了方便编写而设计的

在这章中会介绍大部分语法糖的常用用例, 读者可以以此类推

用例上半部分是原始形式, 下半部分是语法糖形式

> [!NOTE]
> `___0` `___1` 这种格式通常是构建期分配的临时量, 通常并不会在结果中看到
>
> `__0` `__1` 这种通常是编译期分配的临时量, 经常在 DExp 的句柄看到
>
> 这两种形式都不要手动写, 以下示例只是为了演示语法糖

## 单分支匹配语法
```
match a @ b {
    X @ Y { print X @ Y; }
}
match a @ b => X @ Y { print X @ Y; }
```

```
const match a @ b {
    X @ Y { print X @ Y; }
}
const match a @ b => X @ Y { print X @ Y; }
```


## Take 省略返回句柄
```
take __ = Value;
take Value;
```


## 重复块匹配语法
```
inline 1@ { const match @ => V {
    print V;
} }
inline@ V {
    print V;
}
```

```
inline 2@ { const match @ => A B {
    x = A + B;
} }
inline@ A B {
    x = A + B;
}
```


## op-expr
对, 这个也是语法糖, 之前已经介绍过许多了

```
{ a = 1; b = 2; }
a, b = 1, 2;
```

```
{ take ___0 = a; ___0 = 1; b = ___0; }
a, b = 1;
```


## op-expr self op
```
{ take ___0 = x; ___0 = ___0 + 1; }
x += 1;
```

```
{ take ___0 = x; ___0 = min(___0, 1); }
x min= 1;
```

```
{
    { take ___0 = x; ___0 = min(___0, 1); }
    { take ___1 = y; ___1 = min(___1, 2); }
}
x, y min= 1, 2;
```

```
{
    take ___0 = 1;
    { take ___1 = x; ___1 = min(___1, ___0); }
    { take ___2 = y; ___2 = min(___2, ___0); }
}
x, y min= 1;
```


## Value Inc and Dec
```
x += `1`;
x++;
```

```
x -= `1`;
x--;
```


## if elif else skip while do-while gwhile switch break continue
这些也是语法糖, 之前介绍过了


## Quick DExp Take Reference
```
const Foo = (2: print _0;);

const X = Foo[3]->$;
const Y = Foo->[3];
```


## Consted DExp
```
const Do = (take _0 _0;);

take Do[(__:
    const ___0 = (:x goto :x;);
    setres ___0;
)];
take Do[const(:x goto :x;)];
```

> [!NOTE]
> 使用这种方式是因为, 传参过程中, 参数不会被记录标签等, 需要经过一下 const,
> 否则标签不会被重命名, 重复展开就会炸, 所以需要这个语法糖, 使其好看一些


## Statement like value op-expr
```
print ($ = a+b+c;);
print (?a+b+c);
```

```
print ($ = a;);
print (?a);
```

```
print (x: $ = a;);
print (?x: a);
```

```
print ($ = (__: setres a; $ += 1;);); # 这里需要额外赋值一次
print ($ = ++a;);
print (?++a);
```

> [!NOTE]
> `(?++a)` 这种方式非常低效, 在`++`这类发明出来之前, `(?)`就已经够用了,
> 详见接下来的`(*)`


## Value like value op-expr
```
print ($ = a+b+c;);
print (*a+b+c);
```

```
print a;
print (*a);
```

```
print ($ = a++;);
print (*a++);
```

```
print (__: setres a; $ += 1;);
print (*++a);
```
这种形式的 op-expr 展开为的是值形式, 而不是语句形式, 这种情况可以有更好的结果


## Multi print
```
inline {
    `'print'` 1;
    `'print'` 2;
    inline@{ `'print'` @; }
    `'print'` 3;
} # 使用'print'避开关键字
print 1 2 @ 3;
```


## Quick Take
```
take Foo[1 2 3 @ 4];
Foo! 1 2 3 @ 4;
```


## Take like value op-expr
```
take A=(*a+b) B=(*c/d);
take*A, B = a+b, c/d;
```


## Tmp Handle in Take
```
take+A+B Foo[A B] +C;
take A=() B=() Foo[A B] C=();
```


## Cmp Deps Quick Take
```
const C = goto({
    inline { foo; }
    # setArgs a b
} => _0 < _1);
const C = goto({ foo; }=>[a b] _0 < _1);
```

```
const C = goto({
    inline {}
    # setArgs a b
} => _0 < _1);
const C = goto(=>[a b] _0 < _1);
```

这在之后会讲到


## Packed DExp like
一些 DExp 在语法上不能被直接使用, 需要使用包裹语法`(%)`

这算是语法设计的妥协, 也可以提醒过长的 DExp 后面可能接着其它东西

```
print ().x; # syntax error
print (%()).x; # passed
```

```
print (%(v: $.x = 2;)).x;
print (%v: $.x = 2;%).x;
```

各种高级的值
===============================================================================

- 比较者 (Cmper): 用于条件内联, 详见 [条件依赖和条件内联](#条件依赖和条件内联)
- 闭包值 (ClosuredValue): 用于追溯时捕获环境, 详见 [ClosuredValue (闭包值)](#ClosuredValue-闭包值)


ClosuredValue (闭包值)
===============================================================================
这个值可以在追溯时在内部将一些追溯处的值进行提前绑定、求值,
然后在自身求值前将提前绑定、求值的值在当前环境中使用

可以以以下几种形式进行捕获:

- 求值捕获: `A:Value` 相当于 `take Closure.A = Value;`
- 追溯捕获: `&A:Value` 相当于 `const Closure.A = Value;`
- 参数捕获: `@` 相当于 `const Closure._0 = _0; const Closure._1 = _1; ...`,
  捕获个数为参数个数, 只能同时编写一个, 写在求值和追溯捕获后面, 标签捕获前面
- 标签捕获: `| :a :b`, 可以使用捕获时的标签重命名, 方便一些灵活的跳转

以上的 `Closure` 是闭包携带的句柄, 闭包包含一系列捕获和一个值,
值也是绑定在闭包句柄上的, 也就是值在求值时可以改变自己捕获的值,
给句柄上相对应的绑定进行 take 或 const 就行了

> [!TIP]
> 对于求值捕获和追溯捕获, 当名称相同时可以简写, 算是一个语法糖
> - `A:A` -> `A`
> - `&A:A` -> `&A`

```
const N = 2;
const F = ([&N](
    print N;
));
const N = 3;
print "split";
take F;
```
编译为
```
print "split"
print 2
```

可以看到, 闭包的确没有在追溯时展开, 捕获的`N`也没受外部`const N = 3;`所影响,
因为闭包在对内部的值求值前, 先进行了类似`const N = Closure.N;`的操作,
然后再求值内部包含的值, 类似`setres Closure.__Value;`


参数捕获
-------------------------------------------------------------------------------
参数捕获可以让闭包捕获追溯处的参数, 并在内部值求值前设置参数(包括 `_0` `_1`)等

```
const Builder = (
    const $.F = ([@](
        print @ _0;
    ));
);
print "split";
const Clos = Builder[a b]->F;
take Clos[c d];
```
编译为
```
print "split"
print a
print b
print a
```

可以从编译结果看出, 它设置了参数, 也设置了老式参数, 并没有出现 `c d`


标签捕获
-------------------------------------------------------------------------------
标签捕获可以捕获捕获处重命名后的标签,
主要可以方便的从 DExp 外面跳进展开过的 DExp 里面,
不至于获取不到内部重命名后的标签

```
print "start";
const Builder = (
    :x
    comecode;
    const $.Back = ([| :x](goto :x;));
);
const Back = Builder[]->Back;
print "split";
take Back[];
end;
```
编译为
```
print "start"
comecode
print "split"
jump 1 always 0 0
end
```

可以看到, 成功的跳进了 DExp 里面, 我们可以看看不使用闭包为什么样子:

```
print "start";
const Builder = (
    :x
    comecode;
    const $.Back = (goto :x;);
);
const Back = Builder[]->Back;
print "split";
take Back[];
end;
```
编译为标签形式
```
    print "start"
__0_const_Builder_x:
    comecode
    print "split"
    jump x always 0 0
    end
```
明显能看到jump的标签和重命名后的标签不一致, 这样正常编译就会失败


一些语句的扩展用法
===============================================================================
一些语句有一些实用的扩展用法, 在这章进行简单介绍


关于有序整数分支结构的穿透 (select switch)
-------------------------------------------------------------------------------
这些结构中, 有一个很好用的操作, 可以让某个 case 执行完接着执行另一个 case

对于 select:
```
select n {
    print 0; # 继续执行
    { print 1; end; } # 结束执行
    print 2;
}
```
编译为
```
op mul __0 n 2
op add @counter @counter __0
print 0
jump 4 always 0 0
print 1
end
print 2
```
从结果很容易看出, 当`n`为`0`时, 会打印 `01`, 这执行了多个 case, 也就是所谓的穿透


---
对于 switch, 它将代码简单的编译为 select, 这会导致一个问题,
也就是想在 switch 中应用穿透时, 是按大小顺序穿透的, 例如:

```
switch n {
case 1: print 1;
case 0: print 0;
}
```
构建为
```
select n {
    {
        `'print'` 0;
    }
    {
        `'print'` 1;
    }
}
```
我们从代码顺序来看, 应该是 case 1 的代码执行完毕后, 穿透到 case 0,
但是构建结果标明实际上是 case 0 在前面,
所以 switch 按顺序直接构建到 select 有的时候使穿透并不是那么方便

如果需要按编写顺序穿透的可以参考
[#关于自由序整数分支结构的穿透 (gswitch)](#关于自由序整数分支结构的穿透-gswitch)


关于自由序整数分支结构的穿透 (gswitch)
-------------------------------------------------------------------------------
在前面讲解了 select 和 switch 这种有序整数分支结构,
在这一章讲解自由序整数分支结构

使用 gswitch 可以使 case 代码按编写顺序自由的编排,
因为 gswitch 使用的是跳转表形式, case 代码部分不受 select 的限制

将前面的例子拿过来对比一下, 例如:

```
switch n {
case 1: print 1;
case 0: print 0;
}

print "split";

gswitch n {
case 1: print 1;
case 0: print 0;
}
```
编译为
```
op add @counter @counter n
print 0
print 1
print "split"
op add @counter @counter n
jump 8 always 0 0
jump 7 always 0 0
print 1
print 0
```
可以看到, 这下 case 的代码按编写顺序排布了,
不过代价是 gswitch 总是会生成一张跳转表, 来完成这个功能,
在有些时候行数可能比 switch 更多


switch catch
-------------------------------------------------------------------------------
普通的 switch 语句支持将 未命中、下越界、上越界进行捕获,
可以在 switch 头部附加任意个捕获语句, 展开为 switch 前的if

- `<` 下越界
- `!` 未命中
- `>` 上越界
- CmpTree 自定义条件

以下是两个 switch 的对比

```
switch n {
    break;
case 0: print 0;
case 3: print 3;
}

switch n {
    break;
case !: stop;
case 0: print 0;
case 3: print 3;
}
```
构建为
```
{
    select n {
        {
            `'print'` 0;
            goto :___0 _;
        }
        {} # ignore line
        {
            goto :___0 _;
        }
        {
            `'print'` 3;
            goto :___0 _;
        }
    }
    :___0
}
{
    take ___0 = n;
    {
        {
            goto :___3 _;
            :___2
            {
                stop;
            }
            :___3
        }
    }
    select ___0 {
        {
            `'print'` 0;
            goto :___1 _;
        }
        {} # ignore line
        goto :___2 _;
        {
            `'print'` 3;
            goto :___1 _;
        }
    }
    :___1
}
```
可以看到, 如果使用了未命中捕获,
会跳到 switch 头部的一个块中, 并运行捕获代码, 且不会被附加 switch-append

如果没使用捕获, 则会在连续的未命中块中使用 switch-append

如果同时使用多个捕获的话, 将会在 switch 头部对于每个捕获 case 都生成一个块

```
switch n {
    break;
case <!: stop;
case >: end;
case (a < 2): printflush message1;
case 0: print 0;
case 2: print 2;
}
```
构建为
```
{
    take ___0 = n;
    {
        {
            goto :___2 ___0 >= `0`;
            :___1
            {
                stop;
            }
            :___2
        }
        {
            goto :___3 ___0 <= `2`;
            {
                end;
            }
            :___3
        }
        {
            goto :___4 a >= 2;
            {
                printflush message1;
            }
            :___4
        }
    }
    select ___0 {
        {
            `'print'` 0;
            goto :___0 _;
        }
        goto :___1 _;
        {
            `'print'` 2;
            goto :___0 _;
        }
    }
    :___0
}
```

可以看到, 在头部生成了三个块, `!` 捕获也顺带使用了 `<` 捕获的条件,
而不是一个无条件跳过, 避免浪费


gswitch catch
-------------------------------------------------------------------------------
类似于 switch, gswitch 也有 catch, 不过组合顺序是固定的, 且不用写在头部

组合顺序为 `< ! >`, 并且后面还可以跟一个 ConstKey,
用于将 gswitch 使用的跳转编号进行 const, 为了方便而设计

与 [switch-catch](#switch-catch) 不同的是, 它并不加在头部,
而是和其它普通 case 加在一起, 且也会应用 append

```
gswitch (?x: n//2) {
    end;
case >: print "overflow";
case 0: print 0;
case*<: # 不使用 append, 按正常序穿透到 case 1
case 1: print 1;
}
```
编译为
```
op idiv x n 2
jump 10 lessThan x 0
jump 6 greaterThan x 1
op add @counter @counter x
jump 8 always 0 0
jump 10 always 0 0
print "overflow"
end
print 0
end
print 1
end
```
可以看到, 越界检查还是加在 `@counter` 的跳转前,
但是处理代码按编写顺序加在了 case 之中, 这可以方便的穿透等

> [!NOTE]
> gswitch-catch 目前并不具有 switch-catch 的条件捕获语句, 或许未来会加为语法糖,
> 目前可以手动编写标签在gswitch前使用goto跳入


gswitch guard
-------------------------------------------------------------------------------
gswitch 的普通 case 支持守卫语句, 可以对于同一个编号在不同条件下跳转不同的分支

```
gswitch id {
    end;
case 2 if ty == foo: print foo;
case*1 if ty == bar: print bar;
case 1: print bar1;
}
```
编译为
```
__3:
    op mul __1 id 2
    op add @counter @counter __1
    jump __3 always 0 0
    jump __4 always 0 0
__4:
    jump __1 equal ty bar
    jump __2 always 0 0
    jump __0 equal ty foo
    jump __3 always 0 0
__0:
    print foo
    end
__1:
    print bar
__2:
    print bar1
    end
```

观察上述代码, 可以得出下面两条提示

> [!WARNING]
> gswitch 的跳转表结构本身, 是使用 select 生成的,
> 也就是说当使用了守卫后, 不同编号间守卫的顺序依旧是编号顺序而不是编写顺序,
> 且守卫总是使 select 中最大行数至少达到两行,
> 从而需要多一行计算`@counter`增量的运行开销

> [!NOTE]
> 如果对于一个编号, 其所有 case 都具有守卫,
> 那么守卫条件不成立时会拥有一个隐含的指向未命中的跳转


gswitch multi Var
-------------------------------------------------------------------------------
对于 gswitch, 支持的多个数使用一个分支的情况, 和 switch 的处理方式不同, 例如:

```
switch n {
case 0 1: print 0 1;
}
end;
gswitch n {
case 0 1: print 0 1;
}
```
编译为
```
op mul __0 n 2
op add @counter @counter __0
print 0
print 1
print 0
print 1
end
op add @counter @counter n
jump 10 always 0 0
jump 10 always 0 0
print 0
print 1
```
可以看到, gswitch 因为始终使用跳转表形式,
所以可以很自然的将多个jump指向同一个case,
而 switch 因为要编译成 select 所以很不方便做这种事


gswitch use const Var
-------------------------------------------------------------------------------
gswitch 使用值来当 case 的编号, 且可以参与常量系统, 这拥有很高的灵活度, 例如:

```
match 0 2 => @ {}
const One = 1;
gswitch n {
    end;
case One: print 1;
case @: print 0 2;
}
```
编译为
```
op add @counter @counter n
jump 6 always 0 0
jump 4 always 0 0
jump 6 always 0 0
print 1
end
print 0
print 2
end
```


gswitch ignore append
-------------------------------------------------------------------------------
switch 和 gswitch 不是拥有一个 append 扩展用法吗? 可以把一些行附加在每个块后面

而这个扩展用于在某些 case 中不应用 append, 在 case 后加星号即可, 例如:

```
gswitch n {
    end;
case : print 0;
case : print 1;
case*: print 2;
}
```
编译为
```
op add @counter @counter n
jump 4 always 0 0
jump 6 always 0 0
jump 8 always 0 0
print 0
end
print 1
end
print 2
```
可以看到, 最后的 `print 2` 并没有被附加 `end`


strictNotEqual extend
-------------------------------------------------------------------------------
逻辑语言有 `strictEqual` 严格相等运算, 在比较运算和 op 中都存在,
但是没有严格不等运算, Bang 在 CmpAtom 中扩展了一个扩展比较,
而在 op 中扩展为了语法糖

在 op 中为语法糖导致可能在内联中产生多余代码,
不过推荐是用 Cmper 直接输入 CmpTree 用于内联, 所以没必要改了

```
op strictNotEqual x a b;
op x a !== b;
```
编译为
```
op strictEqual __0 a b
op equal x __0 false
op strictEqual __1 a b
op equal x __1 false
```


内建函数
===============================================================================
内建函数用于处理一些重要, 但不适合做进语法, 且使用频率也较少的功能

内建函数同样是一个值, 也可以被求值,
它们在出错时会引发报错输出在错误输出(但不会停止编译)

它们使用参数系统进行输入, 不支持`_0` `_1`这种方式

内建函数统一使用值绑定绑定在`` `Builtin` ``上面, 比较实用的例如:

```
take Foo = ();
Builtin.Info! "test:"; # 用来在日志输出传入的量, 可用于调试
Builtin.Info! Foo;
print Foo;
```

具体列表详见 [builtin-functions](./builtin_functions.mdtlbl)


条件依赖和条件内联
===============================================================================
在 [复合条件 (CmpTree)](#复合条件-CmpTree) 一章中,
有说到 `({print 2;} => a < b)` 这种写法, 可以在使用一个条件前,
插入一些代码, 主要是为了固定内联某个值时方便引用到量或给需要内联的传参等,

在比较时, 如果用的是 `==` 或 `!=`, 且一方为`false` 或 `0`的时候,
如果另一方再满足:

- 只包含一条比较运算且匿名返回值的 op 的 DExp, 例如 `(op $ a < b;)`
- 一个 Cmper, 例如 `goto(a < b)`

如果满足, 那么条件将被内联, 此时如果是类似`? == false`时, 条件将被先反转.

```
break (op $ a < b;) != false;
break (op $ a < b;) == false;
break (op $ a < b;) != 0;
break (op $ a < b;) == 0;
break (op $ a < b;);
break !(op $ a < b;);
break goto(a < b) != false;
break goto(a < b) == false;
break goto(a < b) != 0;
break goto(a < b) == 0;
break goto(a < b);
break !goto(a < b);
```
编译为
```
jump 0 lessThan a b
jump 0 greaterThanEq a b
jump 0 lessThan a b
jump 0 greaterThanEq a b
jump 0 lessThan a b
jump 0 greaterThanEq a b
jump 0 lessThan a b
jump 0 greaterThanEq a b
jump 0 lessThan a b
jump 0 greaterThanEq a b
jump 0 lessThan a b
jump 0 greaterThanEq a b
```
可以看到, 它们都被内联了


```
break goto(a < b || c < d);
print "split";
break !goto(a < b || c < d);
end;
```
编译为
```
jump 0 lessThan a b
jump 0 lessThan c d
print "split"
jump 5 lessThan a b
jump 0 greaterThanEq c d
end
```
可以看到, 对于 Cmper, 可以存储更复杂的比较条件

> [!WARNING]
> Cmper 被求值时, 会直接触发严重报错退出, 因为 Cmper 是被设计为仅内联使用的

同时, 内联也支持简单的常量交互,
它可以使用 Var 在常量表里找到 DExp、Cmper、`false`、`0` 等,
对于 `false` 或 `0` 还支持空 DExp 模拟二层追溯等

```
const F = false;
const Cmp = goto(a < b);
break Cmp != F;
```
编译为
```
jump 0 lessThan a b
```

同时你还可以利用 CmpDeps 来对 Cmper 进行老式形式传参, 使其更加灵活, 例如:

```
const Less = goto(_0 < _1);
break (=>[1 2] Less);
```
编译为
```
jump 0 lessThan 1 2
```


关于一些命名的解释
===============================================================================
这里讲解一些奇特命名的可能解释,
有些命名是意思都没想好但是先随便用了一个英文缩写, 再凑出来合理的解释的

- DExp <- D-Expression -> Dependency-Expression or Deep-Expression
- Var <- Variable
- gswitch <- goto-switch
- gwhile <- goto-while
- ReprVar -> RepresentationVariable
- take <- take-DExp-handle
- Expand <- ExpandedLines
- Statement <- LogicLine


<!-- vim::sw=8:ts=8:sts=8
-->
