# 简介
这是语言`mindustry_logic_bang`的编译器, 编译目标语言是`Mindustry`游戏中的`逻辑语言`

`逻辑语言`在这是指[`Mindustry`](https://github.com/Anuken/Mindustry)
游戏中的一种名为`逻辑处理器`的建筑中编写与汇编相似的代码并将其导出的形式

# 对比
1. ### **流程控制**
   在`mindustry_logic_bang`语言中,
   可以使用`if` `elif` `else` `while` `do_while` `switch`等语句完成流程控制

   ---
   `逻辑语言`中, 跳转方式只有`jump`也就是`goto`,
   又或者设置`@counter`也就是运行时决定跳转到的位置,
   这两种方式的可读性都很差

2. ### **代码复用**
   在`mindustry_logic_bang`语言中, 我们可以使用`const-DExp`与`take`结合使用,
   它的行为可以类似于宏, 并完成零开销的代码复用.<br/>
   这门语言被设计为零开销语言, 你可以零开销的完成很多事, 而不是在编写效率与运行效率之间进行取舍.

   ---
   在`逻辑语言`中, 代码复用也不是很强, 如果手动将其封装为一个函数, 则要接受:
   1. 设置返回地址
   2. 跳转至函数头部
   3. 在函数尾部跳转回返回地址

   我们起码需要接受这整整三行的开销, 这在编写小型快速的功能时完全不能接受.
   并且更复杂的场景需要传入参数甚至返回值, 这想要做到开销就更大了.

3. ### **条件语句**
   在`mindustry_logic_bang`语言中, 我们可以使用`<=` `>=`等符号进行比较,
   并且可以使用`&&` `||` `!`符号进行组织复杂逻辑

   ---
   在`逻辑语言`中, 我们如果在游戏内自带的编辑界面, 我们可以选择`<=` `>=`等符号.<br/>
   但是如果是手动编辑`逻辑语言`, 我们将会看到`lessThanEq` `greaterThanEq`, 这编辑起来很不方便.<br/>
   并且组织复杂条件困难, 我们都知道`逻辑语言`是直接用`jump`的, 而这个`jump`是单条件的,
   我们需要手动编写短路逻辑来向指定位置各种跳转, 这太恐怖了!<br/>
   (因为密密麻麻的`jump`, 很多复杂逻辑经常被称作"盘丝洞")

4. ### **运算**
   在`mindustry_logic_bang`语言中, 我们可以使用`DExp`来将语句嵌套的塞进一行,
   可以在一行内完成多个计算.<br/>
   产生的中间变量完全编译生成名字,
   当然你可以手动指定这个变量, 在之后要使用这个中间变量的场景完成零开销.<br/>

   如果你手动编写`逻辑语言`而不是使用内置的编辑器,
   那么对于常用运算依旧要使用其序列化名称如`add` `idiv`等,
   而本语言对于这些常用的运算都分配了运算符号, 可以提升编写体验.

   ---
   在`逻辑语言`中, 每一行只能有一个`op`来进行运算, 这经常会导致很多行的运算, 非常搞心态,
   并且还要注意中间变量的复杂关系.

5. ### **学习成本**
   **注意**: 学习这门语言首先要对`逻辑语言`较为熟悉

   这门语言包含的内容并不是很多, 并且为大多数语法提供了一个示例,
   按照目录学习可以快速的掌握这门语言.

   并且在`examples/std/`中, 有着一些编写好的`const-DExp`,
   可以让你知道怎样规范的编写`const-DExp`.

6. ### **特殊语句**
   对于一些常用的特殊语句, 如`set` `print`, 是被专门处理的<br/>
   例如:

   | `bang语言`     | `逻辑语言`              |
   | -------------- | ----------------------- |
   | `set a 2;`     | `set a 2`               |
   | `a b = 1 2;`   | `set a 1`<br/>`set b 2` |
   | `print 1 2;`   | `print 1`<br/>`print 2` |
   | `op i i + 1;`  | `op add i i 1`          |
   | `op + i i 1;`  | `op add i i 1`          |

   所以不用再编写十几行`print`来打印了, 可以放到一两行中了.

# 这是一份示例的代码及编译结果
**Bang语言代码**:
```
id count = 0 0;

while id < @unitCount {
    lookup unit unit_type id;
    const Bind = (@unit: ubind unit_type;);

    :restart # 用于开始统计该种单位的跳转点

    skip Bind === null {
        # 目前已经绑定了一个非空单位
        first icount = @unit 1;

        while Bind != first {
            # 若头单位死亡, 则重新统计该类单位
            goto :restart (sensor $ first @dead;);
            op icount icount + 1;
        }
        op count count + icount; # 将该单位数累加到总单位数

        # 打印每种存在的单位
        print unit_type ": " icount "\n";
    }

    op id id + 1; # 推进单位id
}

print "unit total: " count;
printflush message1;
```
**以上代码将被编译为**
```
set id 0
set count 0
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

# 项目构建
构建这个项目将会比较慢, 原因如下:
1. 使用`rustc`进行编译, 而它略慢, 相对于`gcc` `clang`
2. 使用了大型语法分析框架`lalrpop`, 它会生成二十多万行代码, 再叠加上`rustc`编译更慢

你可以先翻一翻Releases, 看一看有没有已构建的程序, 如果没有或无法使用再尝试自己构建.

## 构建方法
首先安装`rust`工具链, 安装方式可以参考 <https://www.rust-lang.org/tools/install><br/>
请确保你正在使用的工具链是`stable`版本的.<br/>

接下来的构建需要更新索引并从`crates`中获取依赖, 你应该具有合适的网络环境或者配置了镜像源等

将工作目录切换至项目路径(一般就是你`git clone`下来生成的那个目录)
```shell
cargo build --release # 执行这个你可以在target/release下获得编译完成的二进制文件
cargo install --path . # 执行这个你可以在你的shell中直接使用它(假设你已经配置好cargo相关环境)
```

# 编辑器支持
为一些编辑器提供了基础的支持
- [**Vim**](https://github.com/vim/vim):
  这是一个活跃在Unix, Linux等平台的编辑器, 虽然相对来说比较小众<br/>
  为其配置了基础的语法高亮及折叠, 与缩进规则.<br/>
  并且如果你在使用`coc-snippets`, 或者`Ultisnips`(未测试) 的话,
  你可以享受一些配置的代码片段, 如`set` 流程控制语法 `op` `iop`等

- [**MT-Manager**](https://mt2.cn/):
  这是一个安卓端的文件管理器, 其中有一个文本编辑器, 可支持自定义高亮,
  为它配置了基础的语法高亮.

- [**VSCode**](https://code.visualstudio.com/):
  这是一个跨平台的编辑器,
  由 [westernat](https://github.com/westernat) 提供了它对Bang语言的语法支持

`LSP` 目前暂无实现, 也没啥必要实现, 逻辑语言这乱的, 这功能也没法用啥


# 性能
就算你塞几千行代码也基本是瞬间完成, 不用担心什么性能.

# 报错
基本没有什么报错位置, 不怎么友好, 不过基本也没啥报错, 信息也差不多够找出错误

# 如何使用
我们先说明本示例程序的二进制文件名为`mindustry_logic_bang_lang`,
因为可能由于平台原因或个人进行的重命名带来名称不同,
例如在 Windows 上会有`exe`后缀.

这个编译器是从输入流中读取输入, 然后输出到输出流(标准输出)或者标准错误,
而我们可以使用shell的重定向功能将文件作为输入流, 并将输出流输出到另一个文件.

以下为一个示例:

```shell
mindustry_logic_bang_lang c < my_source.mdtlbl > out.logic
```

这个示例中, 我们使用了几乎所有shell都会有的语法, `<`和`>`.

- 选项`c`代表将输入的`MindustryLogicBangLang`语言编译为`MindustryLogicCode`
- `<`后面跟着一个文件, 将这个文件作为程序的标准输入,
- `>`后面跟着一个文件, 并将这个文件作为程序标准输出, 也就是标准输出被覆写进这个文件

如果你的文件名或者其路径包含空格或特殊字符, 那么你可能需要使用单引号或双引号将其包裹.

其它的编译选项可以不传入任何参数来查看其说明:

```shell
mindustry_logic_bang_lang
```
