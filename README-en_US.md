# Introduction
This is the compiler for language `MindustryLogicBangLang`, compilation target language is the `LogicLang` in game [`Mindustry`], Referred to as Bang in the following text

**This is the English version, which may not be updated in a timely manner**

`LogicLang` here refers to the form of writing a language similar to assembly language in a building called logic-processor in the [`Mindustry`] game and serializing this language, Referred to as MLog in the following text

[`Mindustry`]: https://github.com/Anuken/Mindustry

The form of MLog itself is similar to assembly, using conditional jumps or assigning values to program counters to create control flow.
However, this method can only be manually written, which can be very tedious and tiring.
Function parameter passing also requires manual writing of repetitive and cumbersome code, which seriously slows down execution speed

In addition, MLog can only use a single operation instruction to perform calculations, so writing complex formulas can be very lengthy

Bang extends and improves based on the style of MLog itself, with a focus on zero-cost metaprogramming, emphasizing static encapsulation abstraction, value passing, structured control flow unfolding, compile time evaluation, and more
We also added op-expr, which can quickly expand familiar expressions in common language styles into multiple operation instructions

**Example**:
```
i = 0; do {
    x, y = cos(i)*r, sin(i)*r;
} while (*++i) < 360;
```
**Compile to**:
```
set i 0
op cos __0 i 0
op mul x __0 r
op sin __1 i 0
op mul y __1 r
op add i i 1
jump 1 lessThan i 360
```

And as long as the foundation is good enough,
the vast majority of MLog code can be rewritten in Bang without any performance loss,
making it easy to read, modify, abstract, and add/delete codes

Bang provides a flexible large constant system that,
when combined with compile-time arithmetic, DExp code passing, parameter system,
and conditional compilation based on branch matching, can simulate overloading,
compile-time data structures, simulate object-oriented features, chain calls, and more


# Learn
Please refer to the example [README](./examples/README-en_US.md),
Or other instances of [example directory](./examples)


# Advanced use
Various ready-made tools can be used to conveniently solve the requirements

Copy and paste the tool code into your code to use it

- [For Loop](./examples/std/for_each.mdtlbl)
  ```
  For! i in 1..6 (
      print i;
  );
  ```

- [Loop Unrolling](./examples/std/count_loop.mdtlbl)
  ```
  i = 13;
  CountLoop! i 5 const(
      print "x";
  );
  ```

- [Packed Stack](./examples/std/stack.mdtlbl)
  ```
  NewStack! cell1;
  cell1.Push! 1 2 3;
  print cell1.Read[a b c];
  print b c;
  cell1.Write! a b c;
  cell1.Pop!;
  cell1.Pop! x;
  print x;
  ```

- [Sync From Memory](./examples/std/mempack.mdtlbl)
  ```
  MemPack! cell1 0, num foo;

  num.Store! 2;
  foo.Write! 3;

  print num", "foo.Load[];
  ```

- [Benchmark](./examples/std/timeit.mdtlbl)
  ```
  TimeIt! 100 # testing rounds
      (case1:
          _x = "a"+"b"; # 1 lines
      )
      (case2:
          _x = (?"a"+"b"); # 2 lines
      )
      (case3:
          noop;
          noop;
          _x = (?"a"+"b"); # 4 lines
      )
  ;
  printflush message1;
  stop;
  ```

- [Function](./examples/std/function.mdtlbl)
  ```
  const Foo = Function[a b (match @ => A B {
      ...result = A + B;
  })]->Call;
  print Foo[2 3];
  ```

  If you have the ability to modify tool code,
  you can make parameters global variables for direct use,
  or constants that can be automatically replaced within the function body


# How To Install
Releases provide pre built products,
consider downloading the binary files of your platform from them first,
If there are other requirements, consider building it yourself,
refer to below [Project Build](#Project Build)


# Project Build
Building this project will be relatively slow due to the following reasons:
1. Compile using `rustc`, which is slightly slower compared to `gcc` and `clang`
2. Using the large syntax analysis framework 'lalrpop', generated over 320000 lines of code and works together with 'rustc' to make compilation very slower

You can first check the [Releases] to see if there is a built program,
and if it does not exist or cannot be used, try building it yourself

[Releases]: https://github.com/A4-Tacks/mindustry_logic_bang_lang/releases

## Build Method
Firstly, install the `rust` toolchain, as shown in <https://www.rust-lang.org/tools/install>

**Please ensure that the toolchain you are using is a `stable` version**

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
Suggest using VSCode in conjunction with extensions for a better editing experience

Provided basic support for some editors:

- [**Vim**]\:
  This is an editor active on platforms such as Unix and Linux, although relatively niche<br/>
  Basic syntax highlighting and folding, as well as indentation rules, have been configured for it<br/>
  And if you are using `coc-snippets` or `UltiSnips` (untested),
  You can enjoy some configuration code snippets,
  such as `set` process control syntax `op` and `iop`, etc

  please read [syntax](./syntax/vim/) README

- [**MT-Manager**]\:
  This is an Android file manager with a text editor that supports custom highlighting,
  It has been configured with basic syntax highlighting.

  This plugin can be obtained at [here](./syntax/MT-Manager/)

- [**VSCode**]\:
  This is a cross platform editor,
  Provided syntax support for Bang language by [westernat] in this editor

  This extension can be obtained at [here](./syntax/vscode/support/)

- [**BlocklyEditor**]\:
  This is a graphical code editor framework that implements an editor for Bang language.

  Having two branches in Chinese and English

  Not recommended to use, only contains some basic statements

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

# Contributing
See [CONTRIBUTING.md](./CONTRIBUTING.md).
