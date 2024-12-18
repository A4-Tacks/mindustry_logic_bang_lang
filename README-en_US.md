# Introduction
This is the compiler for language `MindustryLogicBangLang`,
compilation target language is the `LogicLang` in game [`Mindustry`]

**This is the English version, which may not be updated in a timely manner**

`LogicLang` here refers to the form of writing a language similar to assembly language in a building called logic-processor in the [`Mindustry`] game and serializing this language

[`Mindustry`]: https://github.com/Anuken/Mindustry

# The advantages of this language
1. ### **Process Control**
   In `mindustry_logic_bang` language,
   use statement such as `if` `elif` `else` `while` `do_while` `switch` ...
   to complete process control

   ---
   In `LogicLang`,
   the only way to control a process is through `jump`, which means `goto`,
   alternatively, set `@counter`, which is the location that the runtime decides to jump to,
   both methods have poor readability.

2. ### **Code Reuse**
   In `mindustry_logic_bang` language,
   we can use `const-DExp` and `take`,
   their behavior is similar to macro expansion,
   which can achieve zero overhead code reuse.<br/>
   This language is designed as a zero cost language,
   where you can accomplish many things without any cost,
   rather than making a trade-off between programming efficiency and running efficiency

   And it can achieve great flexibility by utilizing handle pattern matching, conditional compilation, and macro repetition.
   It can also be combined with binding leakage and compiling calculation to achieve similar effects to structure methods, combining types, etc

   ---
   In `LogicLang`,
   code reuse is not very strong either.
   If you manually encapsulate it as a function, you need to accept:
   1. Set return line address
   2. Jump to function head
   3. In function tail set `@counter` goto the return line

   We need to accept at least three full lines of overhead,
   which is completely unacceptable when writing small and fast functions.
   And more complex scenarios require passing in parameters or even returning values,
   which makes it even more expensive to achieve.

3. ### **Conditional statements**
   In `mindustry_logic_bang` language,
   we can use symbols such as `<=` and `>=` for comparison,
   and you can use `&&` `||` `!` to organize complex conditions.

   ---
   In `LogicLang`,
   if we are in the game, we can use the built-in editor,
   and we can choose symbols such as `<=` and `>=`.<br/>
   But if we manually edit the `LogicLang`,
   we will see `lessThanEq` and `greaterThanEq`,
   which is very inconvenient to edit.<br/>
   And it is difficult to organize complex conditions.
   We all know that in `LogicLang`, `jump` is used, while `jump` is a single condition,
   Therefore, we need to manually write short-circuit logic to jump to various designated positions,
   which is too scary<br/>
   (Due to the dense `jump` jumper lines, many complex `LogicLang` program are often referred to as 'spider caves')

4. ### **Operation**
   In `mindustry_logic_bang`,
   We can use `DExp` to nest statements into one line,
   multiple operations can be completed within one line.

   And it has a simple calculation method like OpExpr, such as `print (?a+b*c+log(x));`

   The generated intermediate variables are automatically named by the compiler,
   of course, you can manually specify this variable to achieve zero overhead in scenarios where you need to use this intermediate variable later

   If you manually write `LogicLang` instead of using the built-in editor in the game,
   So for commonly used operations, serialization names such as `add` and `idiv` should still be used,
   The Bang language assigns operational symbols to these commonly used operations,
   which can improve the writing experience.

   ---
   In `LogicLang`, each row can only have one `op` for single step operations,
   which often leads to many row operations and is very annoying,
   And also pay attention to the complex relationships between intermediate variables

5. ### **Learning costs**
   **Note: To learn this language, one must first be familiar with `LogicLang`**

   This language does not contain much content and provides an example for most grammars,
   Learning by [examples] can quickly master this language

   And in [`examples/std/`], there are some well written `const-DExp`,
   It can help you know how to write `const-DExp` in a standardized manner

   [examples]: ./examples/README.md
   [`examples/std/`]: ./examples/std/

6. ### **Special Statements**
   For some commonly used special statements, such as `set` and `print`,
   they are specifically processed

   Examples:

   | `BangLang`     | `LogicLang`             |
   | -------------- | ----------------------- |
   | `set a 2;`     | `set a 2`               |
   | `a b = 1 2;`   | `set a 1`<br/>`set b 2` |
   | `print 1 2;`   | `print 1`<br/>`print 2` |
   | `op i i + 1;`  | `op add i i 1`          |
   | `op + i i 1;`  | `op add i i 1`          |

   So there's no need to write many lines of `print` to print anymore,
   it can be placed in one or two lines.

# This is an example of the code and compilation output
**BangLang Source Code**:
```
id, count = 0;

while id < @unitCount {
    lookup unit unit_type id;
    const Bind = (@unit: ubind unit_type;);

    :restart # restart unit count

    skip Bind === null {
        # binded a nonnull unit
        first icount = @unit 1;

        while Bind != first {
            # if the first unit dies, restart this type of counting
            goto :restart (sensor $ first @dead;);
            icount++;
        }
        # Accumulate the number of units to the total number of units
        count += icount;

        # Print units with units count not equal to zero
        print unit_type ": " icount "\n";
    }

    id++; # plus units type id
}

print "unit total: " count;
printflush message1;
```

**The above code will be compiled as**:

```
set id 0
set count id
jump 22 greaterThanEq id @unitCount
lookup unit unit_type id
ubind unit_type
jump 20 strictEqual @unit null
set first @unit
set icount 1
ubind unit_type
jump 15 equal @unit first
sensor __0 first @dead
jump 4 notEqual __0 false
op add icount icount 1
ubind unit_type
jump 10 notEqual @unit first
op add count count icount
print unit_type
print ": "
print icount
print "\n"
op add id id 1
jump 3 lessThan id @unitCount
print "unit total: "
print count
printflush message1
```

# Project Build
Building this project will be relatively slow due to the following reasons:
1. Compile using `rustc`, which is slightly slower compared to `gcc` and `clang`
2. Using the large syntax analysis framework 'lalrpop', generated nearly 600000 lines of code and works together with 'rustc' to make compilation very slower

You can first check the Releases to see if there is a built program,
and if it does not exist or cannot be used, try building it yourself

## Build Method
Firstly, install the `rust` toolchain, as shown in <https://www.rust-lang.org/tools/install><br/>
Please ensure that the toolchain you are using is a `stable` version.

The following construction requires updating the index and obtaining dependencies from `crates-io`.
You should have a suitable network environment or configured image sources, etc

Switch the working directory to the project path (usually the directory generated from your `git clone`)

```shell
# By executing this, you can obtain the compiled binary file under target/release
cargo build --release
# Execute this command and you can directly use it in your shell
# (assuming you have already configured the `cargo` related environment)
cargo install --path .
```

# Editor support
Provided basic support for some editors
- [**Vim**]\:
  This is an editor active on platforms such as Unix and Linux, although relatively niche<br/>
  Basic syntax highlighting and folding, as well as indentation rules, have been configured for it<br/>
  And if you are using `coc-snippets` or `Ultisnips` (untested),
  You can enjoy some configuration code snippets,
  such as `set` process control syntax `op` and `iop`, etc

- [**MT-Manager**]\:
  This is an Android file manager with a text editor that supports custom highlighting,
  It has been configured with basic syntax highlighting.

- [**VSCode**]\:
  This is a cross platform editor,
  Provided syntax support for Bang language by [westernat] in this editor

- [**BlocklyEditor**]\:
  This is a graphical code editor framework that implements an editor for Bang language.

  Having two branches in Chinese and English

`LSP` is currently not implemented and there is no need to implement it. The logic language is so messy, and this function cannot be used much

[**Vim**]: https://github.com/vim/vim
[**MT-Manager**]: https://mt2.cn/
[**VSCode**]: https://code.visualstudio.com/
[westernat]: https://github.com/westernat
[**BlocklyEditor**]: https://github.com/A4-Tacks/blockly_for_mindustry_logic_bang_lang

# Performance
Even if you input thousands of lines of code,
it is basically completed in an instant,
without worrying about any performance

# Errors
There is basically no error location information generated,
and the entire error mechanism is very poor However,
error reporting is not common. Usually,
we can still find the source of the error through a small amount of error information.

Fortunately, you won't encounter those terrifying advanced errors when using more basic functions

# How To Use
Let's first explain that the file name of this sample program is `mindustry_logic_bang_lang`,
Because the name may differ due to platform reasons or personal renaming,
For example, on Windows, there will be a `exe` suffix

This compiler reads input from the input stream and outputs it to the output stream (stdout) or stderr,
And we can use the redirection function of the shell to take the file as the input stream and output the output stream to another file

Here is an example:

```shell
mindustry_logic_bang_lang cl < my_source.mdtlbl > out.logic
```

In this example, we used syntax that is common to almost all shells, such as `<` and `>`

- The parameter `c` represents compiling the input `BangLang` into `LogicLang`
  and parameter `l` run lints
- Following `<` is a file, which is used as standard input for the program
- `>` followed by a file and used as program standard output,
  which means that the standard output is overwritten into this file

If you sometimes need to visually see the expanded form of the label,
you can change the `c` parameter to the `Li` parameter,
which will become a logically importable form with labels
It will just throw away some jump optimizations.

If your file name or its path contains spaces or special characters,
you may need to wrap it in single or double quotation marks.

Other compilation options can view their help without passing in any parameters:

```shell
mindustry_logic_bang_lang
```

# Comparison with other compilers
In addition to Bang's compiler, there are many useful compilers that can compile easy to write languages into `LogicLang`, such as:

- [mindcode](https://github.com/cardillan/mindcode)
- [mlogjs](https://github.com/mlogjs/mlogjs)

A simple example for comparison

1. **Bang**:
   [code-and-compiled](./examples/pascals_triangle.mdtlbl)
2. **mlogjs**:
   [code](https://github.com/mlogjs/mlogjs/blob/e17c84769a14c59ae0607db3c71db31d52ea8ad8/compiler/test/examples/pascals_triangle.js)
   [compiled](https://github.com/mlogjs/mlogjs/blob/e17c84769a14c59ae0607db3c71db31d52ea8ad8/compiler/test/examples/pascals_triangle.mlog)
3. **mindcode**:
   [code](./examples/pascals_triangle.mnd)
   *Currently not compiled*
