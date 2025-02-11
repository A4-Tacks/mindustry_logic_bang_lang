# Introduction
Bang language was born to quickly encapsulate and abstract logical languages while maintaining zero overhead

The overall language is an extension based on the style of logical language itself,
which may be a bit strange and different from most languages

The core of language design lies in the manipulation of code, values, and constants during compilation,
which allows for flexible completion of most requirements

The most basic ability is to avoid using line number jumps and label jumps everywhere.
It is possible to convert statements such as `if` and `while` into `goto` during building without having to manually write them

> [!WARNING]
> This tutorial is currently being translated,
> and there are chapters that have not been fully translated yet


Basic Elements
===============================================================================
Bang language is mainly based on two basic elements:

1. Value, which can quickly perform three types of operations: assign, follow, and take,
   including many types.
   The most basic one is Var, which will be discussed in the following text
2. Statement [^1], as a basic element converted into logical lines at compile time,
   can most commonly be composed of multiple Value to form a statement,
   just like writing it directly in a logical language

[^1]: This is also known as LogicLine, but its function is no longer suitable for using this name

The most basic Statement, which is composed of multiple Value as mentioned earlier,
usually starts with Var:

```
read foo cell1 15;
```

For example, the above code consists of four Value, each of which is composed of Var,
namely the four logical variables `read`, `foo`, `cell1`, and `15`

> [!TIP]
> `read` and `15` are also classified as 'logical variables',
> although they are not used as variables in logic,
> they play the same role in logical language, that is,
> the basic elements that make up a line in logical language
>
> And because commands and variables look the same from the text,
> Bang treats them as the same type

> [!NOTE]
> Semicolons are necessary,
> Bang as a language whose syntax is independent of whitespace characters,
> does not contain whitespace characters in its syntax,
> so it is best to have a clear delimiter to separate them


Basic Value Introduction
===============================================================================
There are many types of Value,
and here we will briefly introduce the more basic and commonly used ones


Var
-------------------------------------------------------------------------------
Var refers to all logical-values in a logical language,
which are essentially the parts of logic that can be used as literals, such as:

- number: `1` `1.25` `0x1f` `0x-3e`

- string: `"test"`
  For the back slash that is not strictly processed in logical languages,
  Reverse slash escape in Bang's string will be more strict and convenient.
  You can use reverse slash escape to escape the reverse slash itself and square brackets

  For details, please refer to [Multi line String](./mult_line_string.mdtlbl)

- logical variable: `foo` `a-b` `@copper` `true` `null` `let's`

> [!IMPORTANT]
> The above logical variables are not entirely applicable in Bang,
> such as `a-b` and `let's`.
>
> Logical variables are too free, and except for a few disallowed characters,
> the remaining characters can be pieced together to form variables
>
> If Bang were to design entirely using the syntax of logical languages, it would be very inconvenient.
>
> Therefore, Bang has reduced the syntax of logical variables and used unicode-xid according to the common programming language form,
> so it can support variable names in multiple languages
>
> A normal variable is composed of one (xid-start or underline) and multiple xid-continue,
> e.g `foo_bar` `i` `x2` `你好` `_x`, And incorrect usage, for example: `2x` `a-b`
>
> If the `@` character is added before it, the following part will be similar to a normal variable,
> but the part of xid-continue allows for extra dashes (`-`),
>
> Applicable to some commonly used environment variables (built-in variables) in logical languages,
> such as: `@overflow-gate`

Common numerical forms:

- integer or float: `123` `1_000_000` `1.29` `1e4` `-6`
- hex or binary: `0x1f` `0b1001` `0x-2`

> [!NOTE]
> Attention, Bang supports adding underscores to numbers to increase readability, such as `1_000_000`.
> After compilation, underscores will be ignored, and the above numbers will be directly compiled as `1000000`
>
> The rest of the syntax is based on the syntax supported by the logical language itself, such as `1e4`,
> so the decimal form Scientific notation not supported by the logical language is not supported,
> such as `1.2e3`

But obviously, the above three methods cannot meet all the requirements,
so Bang also created an additional universal format:

Any character enclosed in a single quotation mark, excluding whitespace and single quotation marks,
will form a Var, where double quotation marks represent single quotation marks,
as logical languages themselves do not allow variables to be composed of double quotation marks

So variables in any logical language can be expressed using this syntax,
such as the unsupported format in the previous example:

```
set a 'a-b';
set b 'let"s';
```
Compile to:
```
set a a-b
set b let's
```

> [!WARNING]
> Be careful not to use special characters, although Bang supports these characters,
> they have other meanings in the logic, so when compiled into a logic language,
> the logic processor cannot parse them
>
> For example, `#` is also a comment in logical languages,
> while `;` is also used to separate statements in logical languages


DExp
-------------------------------------------------------------------------------
This is a type of Value that represents a Var,
but the validity of this Var depends on some statements

For example, the returned Var is a logical variable,
and it depends on a certain statement to assign a value to it, making this logical variable valid

You can manually specify the Var to return, for example, the following code:

```
set a 1;
set b 2;
print (foo:
    foo = a+b;
);
```
Compile to:
```
set a 1
set b 2
op add foo a b
print foo
```

As can be seen, DExp, as a value,
always compiles all the statements contained in it before returning its own Var when taking

**Taking any Value will always return a Var**


ResultHandle[^4]
-------------------------------------------------------------------------------
This is a Value used internally in DExp to take the Handle returned for the current DExp

This is usually convenient for assigning values to it, written as a dollar sign

Let's take the previous example of DExp as an example:

```
set a 1;
set b 2;
print (
    $ = a+b;
);
```
Unlike in the previous example, this time we did not manually specify the return Var,
so the compiler will randomly generate a Var to represent the return handle of this DExp

But we don't know what the randomly generated Var is called anymore,
so we need to use ResultHandle[^4] to use it

Compile to:

```
set a 1
set b 2
op add __0 a b
print __0
```

As can be seen, a logical variable named `__0` has been generated to represent the return handle of this DExp

> [!NOTE]
> Try not to use double underscores in variables,
> as this is an internal convention of the compiler and manual use may cause conflicts


ReprVar
-------------------------------------------------------------------------------
In the subsequent constant system, it will be explained in detail,
written as Var wrapped in a layer of anti quotation marks

Example:
```
`read` result cell1 0;
```


ValueBind
-------------------------------------------------------------------------------
Used to bind a Var to a handle,
of a Value and generate a unique correspondence between these two Vars and another randomly generated Var

As long as the binded Var and the binded handle are consistent in two uses,
the handle obtained by taking ValueBind will also be consistent (only in a single compilation)

Example:
```
foo = 2;
foo.x = 3;
foo.y = 4;

print foo", "foo.x", "foo.y;
```
Compile to:
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

This is mainly widely used in constant systems,
where Value can be passed during the taking process instead of just handles,
and multiple Value can be easily passed at once


[^4]: Handle, usually referring to the Var generated after the Value is taken


Common Control Flow
===============================================================================
In Bang, there are many convenient control statements that can be used, such as `if` `while`, etc

> [!TIP]
> Of course, `jump` in logical language can also be used, which is called `goto` in bang,
> The label has also been changed from ending with a colon to beginning with a colon

```
set a 1;
:x
set b 2;
goto :x a < b;
set unreachable 3;
```
Compile to:
```
set a 1
set b 2
jump 1 lessThan a b
set unreachable 3
```
It can be seen that a simple `goto` is directly compiled into the same `jump`,
and most of the control flow statements to be introduced later need to be built as `goto`


Conditional Statement (if elif else skip)
-------------------------------------------------------------------------------
- `if`: When the conditions are met, execute the code
- `else`: When the `if` condition is not met, execute the code
- `elif`: Like `else if`, but with a slightly different structure

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
Compile to:
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

There is also a `skip` statement, which is similar in usage to `if`,
but skips if the condition is met, and `else` is not allowed


Loop Statement (while gwhile do-while)
-------------------------------------------------------------------------------
Loop, usually used to repeatedly execute a piece of code if a certain condition is met

- while: Repeating the execution of a certain code segment until the condition is no longer met
- gwhile: Like the while, but executing an extra line when entering the loop,
  the advantage is that only one condition is generated,
  and complex conditions can make the code shorter
- do-while: Like the while, but always executed at least once, it is a goto that jumps back


```
print "while";
while i < 2 { print 1; }
print "gwhile";
gwhile i < 2 { print 1; }
print "do-while";
do { print 1; } while i < 2;
end;
```
Build[^3] to:
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


[^3]: Bang's workflow is divided into two stages, Build time and Compile time

      Expand some simple fixed things during construction,
      such as if while, and collect some label and label bindings

      Handling more complex things during compilation,
      such as constants, scopes, argument systems, follow, take, etc

      Usually, the `c` option is used to build and compile at once,
      or the `A` option is used to observe the details of the build phase


Control Flow within the Loop (break continue)
-------------------------------------------------------------------------------
In some statements (while gwhile do-while select switch gswitch),
you can use the `break` or `continue` statements to directly jump out of the loop or jump to a new round of loop

```
i = 0; do {
    if (op mod $ i 2;) == 0 {
        op add j j 1;
        break i > 6;
    }
    op add i i 1;
} while i < 10;
```
Compile to labeled:
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
> In the `select` `switch` and `gswitch`, `continue` is to jump to its front
>
> When not in the loop, `break` jumps to the end of the entire code,
> `continue` to jump to the beginning of the entire code,
> and at this point, there is basically no difference between the two


Control Block
-------------------------------------------------------------------------------
You can make the `continue` and `break` in the control block point to the head and end of the control block,
or add an bang (`!`) mark to reverse the meaning.
If only one is added, it will not affect the other

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
Build to:
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

Integer Branch Statement (select switch gswitch)
-------------------------------------------------------------------------------
This type of statement dynamically selects the i-th block of code (starting from block 0) using an integer,
and the principle depends on `@counter`[^2]

> [!NOTE]
> Do not enter non integers or numbers less than 0, or numbers not less than the number of code blocks
>
> Each block of code is composed of one statement.
> If you want to input multiple statements,
> you can use one block, such as `{print 1; break;}`

`select` will be compiled into one of two mature schemes,
with the difference being the amount of code between the two

One approach requires filling in aligned statement blocks,
while the other requires constructing a jump table,
`select` will compile into a solution with fewer logical lines

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
Compile to labeled:
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
From the results, it can be seen that a jump table and an alignment block were generated

> [!TIP]
> If no jump is manually added at the end of the internal block,
> the subsequent blocks will continue to execute

Usually we can use a more convenient format, such as `switch`,
which will be build as `select`

Switch uses `case` to separate each block of code, and can specify the block number

It can also attach the same block to the end of each block, called switch-append


```
switch i {
    break;
case: print 0;
case: print 1;
case: print 2 2;
}
```
Build to:
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
From the expanded code,
it can be seen that the `goto` built by `break` is appended to the end of each `case`

And `switch` can directly specify the block number,
and the empty part will be automatically filled with empty blocks:
```
switch i {
    break;
case 2:
    print 2;
case 4:
    print 4;
}
```
Build to:
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
As can be seen, for continuous empty blocks, append will also be performed

`gswitch` is like `switch`, but it always compiles into a jump table form,
which can reasonably have more flexibility

[^2]: This is a program counter in logic languages,
      used to indicate the line number to be executed after a certain line is completed.

      It can be changed on a certain line to adjust which code to execute next, with high flexibility


Simple Comparison (CmpAtom)
-------------------------------------------------------------------------------
It mainly consists of conditions that can be expressed in a single logical `jump` statement,
appearing as the smallest element in the composite condition.

The operator forms are listed below:

- `_`: Unconditionally always true
- Never: Unconditionally never true
- `<` `>` `<=` `>=`: lessThan, greaterThan, lessThanEq and greaterThanEq
- `==` `!=`: equal and notEqual
- `===` `!==`: strictEqual and strictNotEqual

> [!NOTE]
> `!==` It is an operator that is an additional extension of Bang and does not exist in logical languages.
> If it is ultimately used to generate logical code,
> an additional op statement will be used and its result will be reversed
>
> Never is an operator that is an additional extension of Bang,
> and it does not even have a syntax for simple comparison.
> It is usually obtained by reversing `_` through composite conditions, such as `!_`


Complex Comparison (CmpTree)
-------------------------------------------------------------------------------
A simple comparison often cannot meet the requirements,
so complex comparison can be used to associate multiple simple comparisons

Complex conditions are usually organized using the following operations:

| Example                   | Priority    | combination | Name    |
| ---                       | ---         | ---         | ---     |
| `!a < b`                  | 4           | Right       | CmpNot  |
| `a && b`                  | 3           | Left        | CmpAnd  |
| `a \|\| b`                | 2           | Left        | CmpOr   |
| `({print 2;} => a < b)`   | 1           | Right       | CmpDeps |

You can also use parentheses to avoid priority: `(a < 2 || b < 2) && c < 2`

> [!NOTE]
> CmpDeps can compile certain statements before comparing a certain condition,
> similar to DExp, but it requires parentheses in many places
>
> `!` The operation does not actually exist,
> it uses boolean transformations to invert internal conditions until it reaches a simple condition and ends
>
> The operations of `&&` and ` | | ` are short-circuit that is:
>
> - `a && b` When a is false, it is directly false and b will not be calculated
> - `a || b` When a is true, it is directly true and b will not be calculated
>
> Reasonable use of this short-circuit characteristic can bring many conveniences


Simple Statement
===============================================================================

- noop: A statement in logic that cannot be manually typed and is displayed as "Invalid",
  Even statements that fail to parse will produce it
- op: Operational statements compatible with logical language styles
- print: Compatible with logical languages, but supports multiple arguments.
  e.g `print "foo: "foo"\n";`
- Expand: commonly referred to as a block,
  It can contain multiple statements, usually used after loops, such as:
  ```
  {
      print 1;
      print 2;
  }
  ```
- Inline Block: Like the Expand, but without scope, such as `inline {}`,
- Label: Used for the goto label, written as a colon followed by a Var, such as `:foo`
- Other: A logical language statement composed of multiple Values,
  such as `read result cell1 0;`

  Will take each value that makes up the statement and then use handles to form the statement


OP and Comparison Styles Compatible
-------------------------------------------------------------------------------
For CmpTree and op statements, compatible with logical language styles

For example, each of the following `skip` is the same and can be compiled

```
skip a < b print 2;
skip < a b print 2;
skip a lessThan b print 2;
skip lessThan a b print 2;
```

Similarly, the following `op` is also the same and can be compiled
```
op add a a 1;
op a a add 1;
op + a a 1;
op a a + 1;
```

```
op floor r n 0; # `0` that is not used in unary operations will not be taken
op r floor n 0;
op floor r n;
op r floor n;
```

Although this design has little practical effect and may be removed in the future,
some people may like it


Simplified Operations - Operational Expressions (op-expr)
-------------------------------------------------------------------------------
Generate a series of nested op wrapped in DExp using readable expressions

```
i, x = 2, abs(a-b) + sqrt(a)*2;
```
If there is no op-expr, we would need to write the following code
```
{
    `set` i 2;
    op x (op $ abs (op $ a - b;);) + (op $ (op $ sqrt a;) * 2;);
}
```

At the same time, if-else is also provided,
please refer to [op-expr](./op_expr.mdtlbl) for details

> [!NOTE]
> The `||` and `&&` operation priorities provided by op-expr are similar to CmpTree,
> but do not have short-circuit characteristics
>
> op-expr `||` and `&&` are implemented using `+` and `land`
> for the convenience of logical operations

> [!TIP]
> op-expr results comma can ignore


About Comments
===============================================================================
Bang's comments are extended on the basis of logical language

But a new syntax has also been added,
where content from the beginning of `#*` until `*#` will be ignored and can be used across multiple lines without the need to add comment characters to each line

Of course, for the sake of habit or style, `* ` is often added to the beginning of the line

```
# This is a inline comment
set a not_a_comment;
#* This is a multi-line comment
In multi-line comment
* In multi-line comment
*# set b not_a_comment;
set c not_a_comment;
```
Compile to:
```
set a not_a_comment
set b not_a_comment
set c not_a_comment
```

> The annotation style of logical language,
> using the `#` character for annotation,
> will ignore the content from the `#` character until the end of the line


Advanced Beginner - Constant System
===============================================================================
Bang language provides a very powerful constant system for implementing metaprogramming,
allowing for flexible manipulation of code to meet the needs of most logic languages

Here is a core statement: `const`

The most basic usage of the `const` statement is to const a value onto a Var,
which will then take effect within the current Expand and its child Expands (referred to as the scope)

When following or taking with a Var of the same name within the scope,
replace the current Var with its corresponding const value

> [!WARNING]
> DExp also contains an implicit layer of Expand, please do not ignore it

```
const A = 2;
print A;
```
Compile to:
```
print 2
```
It can be understood as directly replacing the const values within the scope

But before replacing it, some additional operations will be carried out,
which will be explained in detail later

If a Var is duplicated const in the same Expand, the old Var will be overwritten, for example:
```
const A = 2;
const A = 3;
print A;
```
Compile to:
```
print 3
```


Shadow
-------------------------------------------------------------------------------
If const is performed in a sub Expand,
the const outside the sub Expand will be shadowed within its scope

Whether it's take or follow, it always retrieves the const value of the innermost and innermost layers.

However, after leaving the inner layer Expand, the const value in the outer layer Expand does not change, but is shadowed by the Var of the same name in the inner layer

```
const A = 2;
{
    const A = 3;
    print A;
}
print A;
```
Compile to:
```
print 3
print 2
```

It can be seen that when a shadow occurs in the child Expand,
the const Value in the parent Expand is not overwritten and can still be used after the child Expand ends


Follow
-------------------------------------------------------------------------------
When performing const on a value, it will be followed **once**,
and for the most common Var, its follow is to query const

Example:
```
const A = 1;
const B = A;
const A = 2;
print B;
```
Compile to:
```
print 1
```
Instead of (*in the old version, it really was*)
```
print 2
```

This also constitutes the basic assignment ability of Values

This is a rough follow table, and values not listed will not be followed:

- Var: Query constant table
- ReprVar: Unpack ReprVar, e.g `` `X` `` into `X`,
  And because its follow and take results are both Var,
  there will definitely not be ReprVar in the constant table
- ValueBindRef (`X->$`): Take X, But when following
- ValueBindRef (`X->..`): Follow X,  The Binder that obtains the follow result of X
- ValueBindRef (`X->Name`): Obtain a value similar to ValueBind,
  but do not take the value because it is being followed
- Closure: Capture the environment, including take, follow, arguments and label renaming.
  Then follow the inner values

> [!NOTE]
> For constant table queries using Var, if the query does not hit, Var itself will be returned
>
> For example, `read result cell1 0;` is composed of four Vars that were not hit in the query


Take
-------------------------------------------------------------------------------
Take converts any value into a Var,
the most common being DExp, which compiles the included statements to obtain its handle

- Var: Follow, if the result is not Var, continue taking the result
- ReprVar: Unpack ReprVar
- DExp: It has already been explained earlier
- ValueBind: After taking the binded value, use two Vars to query the binding table,
  and then take the Var found by the query
- ValueBindRef (`X->$`): Equivalent to take X
- ValueBindRef (`X->..`): Equivalent to following it
- ValueBindRef (`X->Name`): Equivalent to take ValueBind `X.Name`
- ClosuredValue: First set the capture environment, then take the inner value
  see [ClosuredValue (闭包值)](#ClosuredValue-闭包值) for details
- Cmper: Compile error, see [条件依赖和条件内联](#条件依赖和条件内联) for details


ValueBind
-------------------------------------------------------------------------------
You can const the value onto ValueBind

It's not much different from Var,
just const the value to the handle of ValueBind, but the scope is global

By simply passing the handle, multiple ValueBinds can be passed,
and multiple const values can be passed

Example:

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
Compile to:
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

1. Take Statement: When you need to take some values but don't care about the handle,
   you can use the take statement
2. Binder: Write as `..`,
   It is a type of Value that expands to the handle to which the innermost value is binded

   Similar to `self` or `this` in other languages
3. 常量表中值的绑定者: 常量表并不止被 const 的值, 还包含重命名标签和绑定者,
   在后面有详细介绍

可以看到仅使用一个 const, 就可以把 X Y Print 的映射关系都传递到了 FooVec,
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
Compile to:
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
但是我们这次并不需要它们, 所以可以使用 take 语句, 虽然求值, 但是无视其句柄


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
Compile to:
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

`take Value = F;` 其运作原理是, take 先将`F`进行求值, 然后此时将得到的句柄,
进行类似 const 的流程

假设`F`求值的句柄是`X`, 那么此时求值后发生的类似`` const Value = `X`; ``,
`X`使用 ReprVar 因为这并不会对`X`再次追溯


关于 const 时的细节
-------------------------------------------------------------------------------
有一个知识, 就是当你 const 的时候, 并不仅是值被打进了常量表,
其实还有绑定者和标签, 标签是构建时被记录在 const 语句上的,
使用`A`选项就能看到, 例如:

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
当你对一个量进行求值且在常量表中查找到一个值时, 大致会发生以下步骤

1. 设置绑定者为常量表中记录的, 你可以在值里面使用`..`进行获取, 类似`$`
2. 设置标签重命名为常量表中记录的, 此时定义或使用标签会被重命名,
   这样不会在对一个 DExp 等展开多次的时候, 出现多个同名的内部定义的被跳转标签
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
Compile to:
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
Compile to:
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
Compile to:
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

   **从上文可以看出, 使用这种形式的时候,
   别在参数里面求值`$`, 不然会得到用于设置参数的这个 DExp 的返回句柄,
   也就是那个没有用到的`__`**
3. 这个先忽略, 这会在[匹配与重复块](#匹配与重复块)章节深入描述

> [!TIP]
> 我们可以注意到, `take Foo;` 实际上是展开为了 `take __ = Foo;`,
> 这是随便找了个你肯定不应该使用的变量来接收返回的句柄, 反正你也用不到

---
之前代码 const 中使用的 `@` 字符, 可以让当前环境中的参数在此处展开, 比如:
```
const Foo = (
    print "(" @ ")";
);
take Foo["Hello" "Jack"];
```
Compile to:
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
Compile to:
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
Compile to:
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

从编译结果可以看出它的工作原理, `@`字符前面写数量, 数量不足时**依旧会运行**,
如果数量省略不写则默认为`1`

> [!WARNING]
> 重复块的重复数量可以为 0, 那么它将无限的重复,
> 此时需要使用内建函数 StopRepeat 来终止下一轮重复
>
> 如果重复数量为 0 且为硬重复的情况下, 也就是 `inline 0@{}` 时,
> 不仅无限重复, 还不会把参数清空, 可以在下一轮重复时获得上一轮设置的参数

通常重复块也配合着 match 一起使用, 来取出参数内容, 或者仅关心参数足够时的情况

> [!TIP]
> 使用了`*`这种标注的重复块如 `inline*0@{}` 被称作软重复,
> 如果没有使用则被称作硬重复
>
> 软重复将不再输入一个固定的数字, 而是输入一个值做重复数量,
> 既然是值那么就可以参与常量系统, 这可以用来做一些灵活的设计

```
const C = 2;
match 1 2 3 4 5 6 7 { @ {} }
inline*C@{
    foo @;
}
```
Compile to:
```
foo 1 2
foo 3 4
foo 5 6
foo 7
```

> [!NOTE]
> 重复块从常量系统中获取的重复次数是对值使用常量评估而不是求值进行的,
> 所以对于一些值你可能需要先求值再给重复块使用


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


## op-expr results ignored comma
```
a, b, c = 1, 2, 3;
a b c = 1, 2, 3;
```

```
a, b, c = 1;
a b c = 1;
```


## Param Comma Postfix Compatible
```
Foo! a b c;
Foo! a, b, c;
```

```
Foo! a b c;
Foo! a, b, c,;
```

```
take Foo[a b c];
take Foo[a, b, c];
```

```
take Foo[a b c];
take Foo[a, b, c,];
```

```
take Foo[a @ b c];
take Foo[a, @, b, c,];
```

```
take Foo[
    a b
    c d
];
take Foo[
    a b,
    c d,
];
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


## Quick DExp Take
```
foo (__:
    # setArgs 1 2;
    setres Foo;
);
foo Foo[1 2];
```


## Bang Take
```
take Foo[1 2 3 @ 4];
Foo! 1 2 3 @ 4;
```


## Take Param Prefix Reference
```
const Foo = (2: print _0;);

const X = Foo[3->$];
const X = Foo[*3];
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


## Packed Statement Inc and Dec
```
inline {
    take ___0 = i;
    read num cell1 ___0;
    op ___0 ___0 + `1`;
}
read num cell1 i++;
```

```
inline {
    take ___0 = i;
    read num cell1 ___0;
    op ___0 ___0 - `1`;
}
read num cell1 i--;
```

```
inline {
    take ___0 = i;
    Foo! ___0:
    op ___0 ___0 - `1`;
}
Foo! i--;
```

用于在一些特定的情况将内部的后缀自增自减添加至当前行之后 (**和op-expr没有关系**)


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
- 标签捕获: `| :a :b`, 可以使用捕获时的标签重命名,
  方便一些灵活的跳转, 比如从更内层或外层的 DExp 准确跳到捕获处的标签

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
Compile to:
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
Compile to:
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
Compile to:
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
Compile to labeled:
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
Compile to:
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
[关于自由序整数分支结构的穿透 (gswitch)](#关于自由序整数分支结构的穿透-gswitch)


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
Compile to:
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
Compile to:
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
Compile to:
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
Compile to:
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
Compile to:
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
Compile to:
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
Compile to:
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
在 [Complex Comparison (CmpTree)](#Complex-Comparison-CmpTree) 一章中,
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
Compile to:
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
Compile to:
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
Compile to:
```
jump 0 lessThan a b
```

同时你还可以利用 CmpDeps 来对 Cmper 进行老式形式传参, 使其更加灵活, 例如:

```
const Less = goto(_0 < _1);
break (=>[1 2] Less);
```
Compile to:
```
jump 0 lessThan 1 2
```


关于 Bang 编程技巧
===============================================================================
在进行 Bang 的复杂编程时, 容易遇到各种坑,
可以使用一些技巧来避开一些坑, 或增加调试效率


快速参数卫生性
-------------------------------------------------------------------------------
对于有些 DExp, 我们希望它接收到的参数或部分参数始终被求值,
那么最好在其它常量创建前先将参数进行求值

由于 match 直接对输入的参数全部求值, 所以尽量用 match,
不够用了再考虑 const-match

例子:
```
const Foo = (match @ => A {
    print 1 A;
});
const A = 2;
Foo! (
    setres A;
);
```
Compile to:
```
print 1
print 2
```

可以看到, 一切正常, 以下是反例:
```
const Foo = (const match @ => A {
    print 1 A;
});
const A = 2;
Foo! (
    setres A;
);
```
尝试编译, 无限展开超出限制

这是因为我们预期传入的 DExp 中使用的 A 是调用处 const 的那个,
但是传进 Foo 后, 先使用 const-match 在内部创建了一个 A, 然后再求值 A

此时传入的 DExp 从环境中获取 A 时获取到了它自己, 所以就无限展开了


常量引用闭包
-------------------------------------------------------------------------------
在上一章的例子中, 我们产生了无限展开, 这是因为 Foo 没有立即将参数进行求值,
而是先进行了一些 const, 再对参数进行求值, 造成了隐患

但是有时候我们确实需要非立即的求值, 或者是将值传递并不求值等,
那么我们在调用这一类 DExp 的时候,
可以为了保险起见, 如有引用外部的常量, 就使用闭包

以上一章反例举例, 我们在传入的 DExp 外面包裹一层闭包:
```
const Foo = (const match @ => A {
    print 1 A;
});
const A = 2;
Foo! ([&A](
    setres A;
));
```
Compile to:
```
print 1
print 2
```
可以看到, 结果很正常, 也符合预期


传参时提前求值
-------------------------------------------------------------------------------
接着上一章, 对于有时候, 我们调用 DExp 时不一定完全需要它被延迟求值等,
那么我们可以直接在调用时利用绑定引用在传参追溯时就将其求值
```
const Foo = (const match @ => A {
    print 1 A;
});
const A = 2;
Foo! *(
    setres A;
);
```
Compile to:
```
print 1
print 2
```
可以看到, 结果也是非常的正常,
如果在传入的 DExp 加上些标注, 那么就会发现在传参时就已经将其求值完成了:
```
const Foo = (y; const match @ => A {
    print 1 A;
});
const A = 2;
Foo! *(
    x;
    setres A;
);
```
Compile to:
```
x
y
print 1
print 2
```


使用内建函数方便调试
-------------------------------------------------------------------------------
经常我们在编写复杂的常量操作时, 出现了问题可能很难排查,
此时可以借助一些内建函数来帮助调试

- Info: 将传入的一个量以普通日志形式打印
- Err: 将传入的一个量以错误日志形式打印
- Debug: 将传入的值以类似 A 编译选项的形式打印其接近源码的形式
- ExpandStack: 打印常量求值调用栈

还有一些是操作一些功能, 让代码输出更方便查找问题的

- BindSep: 使用传入的量来分隔绑定量, 例如 `Builtin.BindSep['.']`, 注意别冲突
- MissesMatch: 如果 match 或 const-match 语句没有匹配任何一个分支,
  那么将会打印日志


以赋值返回的灵活 DExp
-------------------------------------------------------------------------------
通常, 如果一个 DExp 能够以给参数赋值的形式返回值, 而不是将自身句柄当值返回,
那么在大多数情况下这类 DExp 在动态场景将更加灵活, 且容易用出更好的性能, 例如:

```
const Foo = (match @ => $R {
    R = a + b;
});
x = 2;
if cond {
    Foo! x;
}
print x;
```
Compile to:
```
set x 2
jump 3 equal cond false
op add x a b
print x
```

但是如果是这么写的, 例如:
```
const Foo = (match @ => {
    $ = a + b;
});
x = 2;
if cond {
    x = Foo[];
}
print x;
```
Compile to:
```
set x 2
jump 4 equal cond false
op add __0 a b
set x __0
print x
```

可以看出, 这种设计在较为动态的场景就可能产生无用的赋值

> [!NOTE]
> 这种以赋值方式返回的 DExp, 虽说在动态的场景有优势,
> 但是在常量评估为主的时候就有劣势了, 所以有时会写两个分支,
> 一个用于返回赋值, 一个用于常量评估的 setres


利用值绑定返回内部值
-------------------------------------------------------------------------------
因为求值只能得到量, 即值的句柄. 为了在求值时返回值,
可以使用 const 将值绑定在句柄上, 在求值后获取句柄上的绑定值来完成返回值, 例如:

```
const Foo = (
    print "foo";
    const $.Value = (print "test";);
);
print "start";
const Value = Foo->Value;
print "split";
take Value;
```
Compile to:
```
print "start"
print "foo"
print "split"
print "test"
```

> [!NOTE]
> 最好绑定到的句柄是匿名的, 或者尽量不要让不同类的绑定在同一个句柄上,
> 不然各种设计在同一个句柄绑定的多了可能会冲突

> [!WARNING]
> 不要制造极端大量的值绑定, 因为并不会自动清理值绑定,
> 所以会同时存在越来越多的值绑定, 在一些极端设计中可能很浪费内存


灵活编译选项
-------------------------------------------------------------------------------
Bang 编译器有着不同的编译选项, 最普通常用的选项就是 `c`,
即为 Bang 直接编译到逻辑, 但是还有其它可用的选项, 对调试有着帮助

- `l`: lint 功能, 可以对部分逻辑语言进行简单的检查, 如赋值未使用的变量、参数错误等
- `A`: 可以将 Bang 源码以完全脱糖的形式输出, 可以用来理解一些奇怪的语法.
  无法写出来的形式将以注释输出, 例如`# setArgs;`
- `L`: Label 功能, 可以将 Bang 编译为标签的形式.\
  不同于 `t` 选项只支持数字标签, `L` 选项支持原本的标签形式,
  所以它可以用来观察常量求值对标签的重命名
- `r`: 用于快速的将逻辑简单的转换成 Bang,
  但是并不会尝试将 jump 转换成 while 等, 这可以快速的将 Bang 嵌入现有的逻辑

> [!TIP]
> 编译选项可以连续使用多个, 可以将上一个选项的输出输入下一个选项,
> 例如最常用的 `cl` 选项

> [!NOTE]
> `L`、`Li`选项虽然很方便观察标签, 但是普通的`c`选项会进行跳转跟踪等,
> 而`L`选项不会, 这可能造成`else if`等一些情况的轻微性能下降,
> 非调试等需求不建议使用


行数超限问题 - 函数结构
-------------------------------------------------------------------------------
Bang 中, 在编写中大型工程时, 很可能由于大量展开大型 DExp,
导致行数经常超过逻辑最大行数一千行, 可以尝试应用函数结构来缩减

关于快速生成非递归函数结构的 DExp: [function](./std/function.mdtlbl)

函数结构的缺点也很明显:
1. 传参、跳转、跳回都需要额外的行数进行, 会拖慢速度
2. 由于是逻辑语言的值传递, 很难传递 Bang 的一些复杂值关系, 例如值绑定

所以, 应用函数结构限制较大, 且对性能有影响, 最好在代码行数过多时再考虑

> [!TIP]
> 在性能方面, 考虑可以不在常量传参时给参数变量赋值,
> 而是在计算时就直接使用参数变量进行计算,
> 那样调用函数时就只需跳转不用对参数赋值了


动态递归问题 - 利用现有模板设施快速生成函数
-------------------------------------------------------------------------------
我们有的时候需要动态的使用逻辑中的函数结构进行递归,
此时可以利用现有示例中的模板来快速的构建一个函数结构

配合栈模板方便的构造函数栈来避免递归过程中对全局变量的覆盖

以下是一个递归计算 fib 的函数示例:

```
Builtin.BindSep! '.';
Builtin.MissesMatch! 1;

# include std/{function,stack,for_each}

NewStack! cell1;

const Fib = Function[n (
    take N = ...n;
    if N <= 1 {
        ...result = 1; # 利用一个固定变量来返回值
        ...Return!; # 直接返回
    }
    cell1.Push! ..->ret_counter;
    cell1.Push! N; # 使用栈来存储n防止被覆盖
    take A = ...Call[(N:$ -= 1;)]; # 利用直接操作参数本身避免中间赋值
    cell1.Read! N;
    cell1.Write! A; # 不需要存储n了, 但是需要存储第一次递归的结果了, 直接用原位
    ...result = ...Call[(N:$ -= 2;)] + cell1.Pop[$]; # 新的递归结果加上栈中的

    cell1.Pop! ..->ret_counter; # 将需要返回到的行从栈中弹出
)]->Call;

printflush message1; # clear text buffer
printflush message1; # clear message1
For! i in 0....9 (
    print "fib("i")="Fib[i]"\n";
);
printflush message1;
wait 8;
```

> [!WARNING]
> 最后使用`Pop!`弹栈到`..->ret_counter`的代码, 不能直接赋值到`@counter`,
> 因为栈设计是指向栈顶, 所以读取后还需要挪动栈顶, 如果直接读取到`@counter`,
> 那么将直接跳走, 不再有挪动栈顶的机会, 导致递归爆栈

> [!NOTE]
> 当调用 Call 时, 如果传入的参数和使用的参数句柄相同,
> 则不会再次赋值, 也就是产生如 `set __10.n __10.n` 这种代码,
> 这是刻意在模板中判断相等造成的, 可以利用其来优化

以下是用到的模板, 复制其主要 const 部分粘贴到代码头部即可

- [function](./std/function.mdtlbl)
- [stack](./std/stack.mdtlbl)
- [for-each](./std/for_each.mdtlbl)


多逻辑编译
-------------------------------------------------------------------------------
有时随着结构的复杂, 不管是行数超限, 还是为了提升计算速度,
我们都需要使用多个逻辑而不是全部在单个逻辑内完成,
本章简单介绍关于 Bang 在多逻辑块编程时的惯用方法

这是一个很简单的小技巧, 可以简单的将每个逻辑块的代码放到一个 DExp 中,
要给哪个逻辑块编译就对哪个 DExp 求值, 例如:

```
const W = (write 8 cell1 0;);
const R = (read n cell1 0; print n; printflush message1;);

take R;
#take W;
```

这样想给哪个逻辑块编译, 就取消哪个 take 的注释即可,
可以较为方便的手动给多逻辑编译


避免常量评估为数字
-------------------------------------------------------------------------------
有的时候我们需要传入的量是一个可修改的量,
但是由于常量评估, 经常被计算成一个数字量, 这就会造成问题, 例如:
```
const Inc = (match @ => I {
    do { print I; } while (*++I) < 10;
});
take Inc[(?2*3)];
```
Compile to:
```
print 6
op add 6 6 1
jump 0 lessThan 6 10
```
可以看到, 这明显是不正常的行为, 这时我们就需要避开常量评估,
方法有许多, 以下是较为方便的:

- `(?x:2*3)`: 有了明确指定的返回句柄, 就不会被评估为一个数字量了
- `({$=2*3})`: 使用块包裹来绕开常量评估的固定形式
- `(?(6:))`: 使用指定了返回句柄的空 DExp 来绕开常量评估的固定形式

> [!TIP]
> 对于 `op add 6 6 1` 这种给数字赋值的语句, 大部分会被`l`选项检测到,
> 能避免大部分此类问题


利用命名风格区分常量和普通量
-------------------------------------------------------------------------------
因为普通逻辑语句都由一系列量组成,
所以如果这些量被用做了常量, 很可能造成很大的混乱,
所以我们使用命名风格来让常量和普通量区分开

通常常量被命名为 **大坨峰**[^5] 或 **大蛇形**[^5] 格式,
而逻辑中基本没有任何此类格式的量,
所以就不必担心编写逻辑语句时使用到了常量或需要频繁使用 ReprVar

> [!NOTE]
> 因为 Bang 支持使用中文及各种乱七八糟的字符作为量,
> 且经常用于和现有逻辑变量对接, 或用于逻辑内部本地化命名,
> 所以对于此类量一致认为基本不是常量


[^5]: 两种命名风格, 适用范围是拉丁字母等

      大坨峰(UpperCamelCase)指的是使用大写字母来分隔每个词, 且首个字母大写,
      例如 `is a foo` 为 `IsAFoo`

      大蛇形(UPPER_SNAKE_CASE)指的是字母全部大写, 使用下划线分隔每个词,
      例如 `is a foo` 为 `IS_A_FOO`


利用无限重复块避免深层递归
-------------------------------------------------------------------------------
在 [重复块](#重复块) 一章中, 有说到重复块无限重复的情况,
可以利用它来替换掉一些用递归解决可能远超递归层数限制的问题

例如如果想给一千以内整数附加上一个 pi 绑定量:

```
const Each = (match @ {
    [1000] {}
    N {
        take*N.pi = N*3.1415926535897932;
        take Each[(*N+1)];
    }
});
Each! 0;
print 8.pi;
```
编译上述代码, 产生递归超限, 我们可以利用无限重复块将其改为:
```
{
    take N = 0;
    inline 0@{
        match N { [1000] { Builtin.StopRepeat!; } N {
            take*N.pi = N*3.1415926535897932;
            take*N = N+1;
        } }
    }
}
print 8.pi;
```
Compile to:
```
print 25.132741228718345
```


关于优化
===============================================================================
Bang 语言从理论上来说, 是不存在任何优化的,
不过 Bang 的中间 tag 码会对无条件跳转链进行简单的追踪, 算是仅有的一丁点优化吧

剩下的一些东西虽然不是优化, 但是经过精心使用可以达到类似优化的效果, 例如:

- op-expr 使用常量评估的优秀实践, 如`(n*2)*3`改成`n*(2*3)`,
  后者会将`(2*3)`进行常量评估使运算少一步.

  而`(n*2)`无法被常量评估, 所以`(n*2)*3`也无法被常量评估, 将会把整个展开

- 对于不同的情况抽象出不同的 DExp 参数组成, 使用时使用不同的参数形式,
  避免大部分封装抽象产生的开销 (但是可能会让抽象变得较为复杂)

  一个较为常见的例子就是针对常量评估和为外部赋值返回写两套代码

- 手动循环展开, 如使用重复块等,
  因为 Bang 并没有优化, 所以常数次循环依旧会原样输出到逻辑, 此时可以换成重复块,
  也可以简单封装一下使其方便使用, 例如 [for-each](./std/for_each.mdtlbl)

- 内联 Cmper, 对于使用值接收一个跳转条件的地方, 最好使用 Cmper 去进行内联,
  而不是使用 op, 因为被内联的 op 并不具备短路特性, 可能会损失性能

所以 Bang 以高度灵活性让人能编写出高性能抽象和封装, 虽说可能抽象的很复杂


Regarding some Naming Explanations
===============================================================================
Here are some possible explanations for peculiar names.

Some names may not have a clear meaning but use a random English abbreviation first,
and then come up with a reasonable explanation

- DExp <- D-Expression -> Dependency-Expression or Deep-Expression
- Var <- Variable
- gswitch <- goto-switch
- gwhile <- goto-while
- ReprVar -> RepresentationVariable
- take <- take-handle
- Expand <- ExpandedLines
- Statement <- LogicLine


<!-- vim::sw=8:ts=8:sts=8
-->
