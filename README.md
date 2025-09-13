English README, please click on [**README-en_US.md**](./README-en_US.md)

# 简介
这是语言`mindustry_logic_bang`的编译器, 编译目标语言是[`Mindustry`]游戏中的`逻辑语言`,
简称 Bang

`逻辑语言`在这是指[`Mindustry`]
游戏中的一种名为`逻辑处理器`的建筑中编写与汇编相似的代码并将其导出的形式,
以下简称 MLog

[`Mindustry`]: https://github.com/Anuken/Mindustry

MLog 本身的形式类似汇编, 使用条件跳转和赋值程序计数器来进行控制流,
而控制流需要全部手搓跳转, 非常的不友好, 函数传参等也要手搓且速度影响明显

而且 MLog 想要进行计算等, 只能使用单条运算指令, 编写复杂公式等会非常冗长

Bang 则基于MLog本身的风格进行扩展改进, 以零成本元编程为重点进行扩展,
侧重静态封装抽象、值传递、结构化控制流展开、编译期算数等
还增加了 op-expr, 可以将常见语言中熟悉的表达式快速展开为多条运算指令

例如
```
i = 0; do {
    x, y = cos(i)*r, sin(i)*r;
} while ++i < 360;
```
编译为
```
set i 0
op cos __0 i 0
op mul x __0 r
op sin __1 i 0
op mul y __1 r
op add i i 1
jump 1 lessThan i 360
```

并且只要功底够好, 可以将绝大多数的 MLog 以没有性能损失的方法,
用 Bang 编写, 方便阅读、修改、抽象和增删

Bang 提供了一个灵活的大型常量系统,
配合编译期算数、DExp 代码传递、参数系统、基于分支匹配的条件编译,
可以完成模拟重载、编译期数据结构、模拟面向对象特性、链式调用等


# 学习
请参考示例的 [README](./examples/README.md), 或[示例目录]的其它实例

[示例目录]: ./examples


# 高级使用
可以使用各种现成的工具, 来方便的完成各种目的

将工具代码复制粘贴到你的代码中即可使用

- [For 循环](./examples/std/for_each.mdtlbl)
  ```
  For! i in 1..6 (
      print i;
  );
  ```

- [循环展开](./examples/std/count_loop.mdtlbl)
  ```
  i = 13;
  CountLoop! i 5 const(
      print "x";
  );
  ```

- [包装栈](./examples/std/stack.mdtlbl)
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

- [从内存同步](./examples/std/mempack.mdtlbl)
  ```
  MemPack! cell1 0, num foo;

  num.Store! 2;
  foo.Write! 3;

  print num", "foo.Load[];
  ```

- [基准测试](./examples/std/timeit.mdtlbl)
  ```
  TimeIt! 100 # 测试次数
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

- [函数](./examples/std/function.mdtlbl)
  ```
  const Foo = Function[a b (match @ => A B {
      ...result = A + B;
  })]->Call;
  print Foo[2 3];
  ```
  如果你有能力修改工具代码, 你可以使参数成为全局变量, 或函数体内自动替换的常量


# 安装
Releases 提供了预构建产物, 考虑先从其中下载自己所在平台的二进制文件,
如果为安卓通常可以运行 aarch64-unknown-linux-musl

如果有其它需求, 再考虑自己构建, 参考下面的[项目构建](#项目构建)

# 项目构建
构建这个项目将会比较慢, 原因如下:
1. 使用`rustc`进行编译, 而它略慢, 相对于`gcc` `clang`
2. 使用了大型语法分析框架`lalrpop`, 它会生成三十二万行代码, 再叠加上`rustc`编译更慢

> [!NOTE]
> 正常情况你应该不需要自己构建, 请在 [Releases] 中获取最新预编译版本

[Releases]: https://github.com/A4-Tacks/mindustry_logic_bang_lang/releases

## 构建方法
首先安装`rust`工具链, 安装方式可以参考 <https://www.rust-lang.org/tools/install>

**请确保你正在使用的工具链是最新`stable`版本的**

接下来的构建需要更新索引并从`crates-io`中获取依赖, 你应该具有合适的网络环境或者配置了镜像源等

将工作目录切换至项目路径(一般就是你`git clone`下来生成的那个目录)
```shell
cargo build --release # 执行这个你可以在target/release下获得编译完成的二进制文件
cargo install --path . # 执行这个你可以在你的shell中直接使用它(假设你已经配置好cargo相关环境)
```

# 编辑器支持
建议使用 VSCode 配合插件, 以获得一个较好的编辑体验

为一些编辑器提供了基础的支持:

- [**Vim**]\:
  这是一个活跃在Unix, Linux等平台的编辑器, 虽然相对来说比较小众<br/>
  为其配置了基础的语法高亮及折叠, 与缩进规则.<br/>
  并且如果你在使用`coc-snippets`, 或者`UltiSnips`(未测试) 的话,
  你可以享受一些配置的代码片段, 如`set` 流程控制语法 `op` `iop`等

  请阅读 [syntax](./syntax/vim/) README

- [**MT-Manager**]\:
  这是一个安卓端的文件管理器, 其中有一个文本编辑器, 可支持自定义高亮,
  为它配置了基础的语法高亮.

  插件可以在[此处](./syntax/MT-Manager/)获取

- [**VSCode**]\:
  这是一个跨平台的编辑器,
  由 [westernat] 提供了它对Bang语言的语法支持

  扩展可以在[此处](./syntax/vscode/support/)获取

- [**BlocklyEditor**]\:
  这是一个图形化代码编辑器框架, 使用此框架实现了一个关于Bang语言的编辑器

  具有中文及英文两个分支

  不建议使用, 只含有一些基本的语句

`LSP` 目前暂无实现, 也没啥必要实现, 逻辑语言这乱的, 这功能也没法用啥

[**Vim**]: https://github.com/vim/vim
[**MT-Manager**]: https://mt2.cn/
[**VSCode**]: https://code.visualstudio.com/
[westernat]: https://github.com/westernat
[**BlocklyEditor**]: https://github.com/A4-Tacks/blockly_for_mindustry_logic_bang_lang


# 性能
就算你塞几千行代码也基本是瞬间完成, 不用担心什么性能.

# 报错
报错不怎么友好, 不过报错也比较少, 信息也差不多够找出错误\
~~就是可能使用高级功能时调试起来过于地狱~~

不过好在, 你使用较为基本的功能并不会遇到那些恐怖的高级错误

# 如何使用
我们先说明本示例程序的文件名为`mindustry_logic_bang_lang`,
因为可能由于平台原因或个人进行的重命名带来名称不同,
例如在 Windows 上会有`exe`后缀.

这个编译器是从输入流中读取输入, 然后输出到输出流(标准输出)或者标准错误,
而我们可以使用shell的重定向功能将文件作为输入流, 并将输出流输出到另一个文件.

以下为一个示例:

```shell
mindustry_logic_bang_lang cl < my_source.mdtlbl > out.logic
```

这个示例中, 我们使用了几乎所有shell都会有的语法, `<`和`>`.

- 参数`c`代表将输入的`Bang`语言编译为`逻辑语言`, 然后参数`l`执行lint做一些检查
- `<`后面跟着一个文件, 将这个文件作为程序的标准输入,
- `>`后面跟着一个文件, 并将这个文件作为程序标准输出, 也就是标准输出被覆写进这个文件

如果有时需要直观的看到标记展开的形式, 可以将`c`参数改为`Li`参数,
将变成逻辑可导入的含标记形式. 就是会丢掉一些跳转优化.

如果你的文件名或者其路径包含空格或特殊字符, 那么你可能需要使用单引号或双引号将其包裹.

其它的编译选项可以不传入任何参数来查看其说明:

```shell
mindustry_logic_bang_lang
```

# 关于其它编译器的对比
除了 bang 的编译器, 还有不少好用的编译器可以将易于编写的语言编译到`逻辑语言`,
例如:

- [mindcode](https://github.com/cardillan/mindcode)
- [mlogjs](https://github.com/mlogjs/mlogjs)

一个简单的用于对比的例子

1. **Bang**:
   [code-and-compiled](./examples/pascals_triangle.mdtlbl)
2. **mlogjs**:
   [code](https://github.com/mlogjs/mlogjs/blob/e17c84769a14c59ae0607db3c71db31d52ea8ad8/compiler/test/examples/pascals_triangle.js)
   [compiled](https://github.com/mlogjs/mlogjs/blob/e17c84769a14c59ae0607db3c71db31d52ea8ad8/compiler/test/examples/pascals_triangle.mlog)
3. **mindcode**:
   [code](./examples/pascals_triangle.mnd)
   *目前暂未编译*

# 贡献
详见 [CONTRIBUTING.md](./CONTRIBUTING.md).
