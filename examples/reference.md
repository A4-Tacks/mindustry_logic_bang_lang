Bang 语言参考手册
==============================================================================
- 对应版本: 0.12.6

本语言(Bang)旨在对逻辑语言(MindustryLogicCode)进行一定的扩展,
并且最大可能的使这些扩展零开销(不含优化, 而是设计上).

逻辑语言的层级与汇编相似, 是一个很低级的语言, 非常不方便编写.
所以本语言在不丢失性能的情况下对逻辑语言进行一定的扩展, 使得编写更加容易.


使用技巧
------------------------------------------------------------------------------
1. 可以连续使用多个编译模式进行编译, 效果等同于多个编译器以进程管道连接

   例如 `mindustry_logic_bang_lang tC`,
   效果等同于 `mindustry_logic_bang_lang t | mindustry_logic_bang_lang C`

2. 遇到不明白的语法展开与扩展等, 可以使用A选项编译来窥探其被展开成了什么样

   注: 这只能观察到构建期的展开, 如if等会被展开为goto.

3. 对于逻辑语言转换到Bang时, 可以使用R选项编译,
   这样会得到一份不那么精美的Bang语言的代码, 虽仍需人工修整, 不过也是方便许多.

   对于流程控制, Bang编译器无法进行重建, 因为逻辑的行为难以分析,
   且如果有`@counter`的修改, 难度更是进一步上升, 所以流程控制的重建需人工介入.

4. 因为常量表替换的踪迹难寻, 所以作者极度推荐使用命名规范来人工追踪其行为.

   对于普通变量可以使用小蛇形命名法或小驼峰命名法,
   对于一切被写入常量表的使用大驼峰或大蛇形命名法.

   注:
   小蛇形(lower\_snake\_case) 大蛇形(UPPER\_SNAKE\_CASE)
   小驼峰(lowerCamelCase) 大驼峰(UpperCamelCase)

5. 对于运算可以使用先进的OpExpr语句, 而不是Op语句.
   前者可以编写多对操作且拥有优先级与三元表达式, 可以更清晰的进行运算.

6. 跳出或继续单层循环 switch等, 你可以使用更先进的`break`/`continue`,
   而不是以前只能使用的goto, 这样既明确了语义, 也不用为起一个标签名称而烦恼.

7. 有时你可能需要一个代码折叠, 一个单一的行, 亦或者其它需求,
   你可以考虑InlineBlock, 它不带有常量表作用域, 因此, 你不用为其而烦恼.

8. 用户命名的Var名称中携带两个及以上下划线时, 不推荐这样做,
   因为这是内部的命名格式, 用户使用容易极大的影响代码健壮性.


以下为描述及语法参考
==============================================================================

量 (Var)
------------------------------------------------------------------------------
### 这是Bang最为基本的成员, 包含以下几个格式

- 数字(Number) 2 10 16 进制整数 浮点数, 可以包含辅助阅读的下划线,
  如 `0x1b` `-2` `0x-5` `0b1101_0010` `0b-8` `0xffff_fa1b` `1_234.567_8`

- 字符串(String) 由双引号包含的一段非双引号字符, 其中换行符会被替换为`\n`
  如 `"foo"` `"bar"`

- 标识符(Ident) 主流编程语言中的标识符格式, 如`abc` `_x_y` `data2`

- 扩展标识符(OIdent) 由一个`@`符号起始,
  接下来的一个字符符合Ident的起始字符规则,
  接下来的零至若干个字符符合Ident的除开头字符规则, 但是额外的允许`-`字符,
  如 `@counter` `@foo-bar` `@_ab`

- 其它变量(OtherVar) 这是为了应付逻辑语言中可能需要的奇葩变量名而被创建的,
  逻辑语言中, 一个变量的格式, 只要不是数字 字符串等格式且不含空白符,
  那么, 它将是一个变量.
  例如 `*&@21^^-` `8a$$%2!!~` `a'b`, 所以, 为了可以和逻辑进行对接, 设计了这种特殊格式.

  其语法为由单引号包裹的若干个非空白字符, 其中双引号会被替换为单引号,
  如 `'*&@21^^-'` `'8a$$%2!!~'` `'a"b'`


值 (Value)
------------------------------------------------------------------------------
这是Bang中的一等公民, 一切高级扩展几乎都是围绕着它展开的.

当它被编译时, 会对它进行take以使用它的句柄.

### 它有以下几种格式

- 量(Var) 所有的Var格式, 其句柄会进行常量表查询, 如果没有查到那么句柄即为它本身
  例如 `123` `abc`

- 依赖表达式(DExp) 可以在其中编写一个返回句柄及一个Expand.

  语法上是一个圆括号, 一个可选的Var加一个冒号, 然后是Expand.

  进行take时其返回句柄也会进行常量表查询, 但是语法上是Var.
  当它被take时, 会先将其内的Expand进行编译, 然后直接将编写的返回句柄返回.

  如果返回句柄为空, 即为缺省, 那么将会在编译时自动分配一个匿名句柄.

  例如 `(foo; bar;)` `(a: set a 2;)`

- 原始变量(ReprVar) 和Var相同, 不同的是它不会进行常量表查询.
  语法是两个反引号包裹的Var.

- 返回句柄替换符(ResultHandle) 语法上是一个单一的美元符号 `$`,
  它的句柄为最里层的DExp的返回句柄.

- 快速take(QuickTake) 以简洁的语法展开为一个包含了传参的DExp,
  语法上是一个Var后面跟上一个方括号, 方括号内部为正常的参数语法.

  相比直接take, 一些灵活使用展开后返回句柄的场景, 它可以非常方便.

  但是, 需要注意的是, 它是一个DExp而不是一个已经take完毕的句柄,
  所以需要提前求值的场景它可能会出问题.

- 值绑定(ValueBind) 将一个Var绑定到一个Value,
  其句柄为对Value进行take后与Var进行特定格式映射关系建立.

  语法为一个Value后面使用点号连接一个Var, 例如 `foo.bar`

- 常量化DExp(consted-DExp) 它会被展开为一个DExp,
  其中立即进行一次匿名const, 然后立即take并将其设置为这个DExp的返回句柄.

  其意义是, 在const的DExp内部的标签是会被重命名的, 但是这个标签首先要被记录,
  而这个记录是被设计在常量语句里面的, 而不是DExp或者Expand, 所以才有这个的诞生.

  使用它, 可以在那些被传入的DExp中也使用标签且不必担心与外部或者内部标签名称冲突.

  语法是DExp前添加`const`修饰
  例如 `const(:x goto :x;)`


行 (LogicLine)
------------------------------------------------------------------------------
这是主要部分, 一切操作都离不开它.

### 以下是它的变体

- 其它行(Other) 这是最基础的行, 它由一至多个Value组成, 由分号结束.
  例如 `foo bar 1 2 3;`

  当它被编译时会正常编译为一个单元行, 内容是对每个Value顺序take得到的句柄.

- 标签(Label) 是一个冒号接着一个Var, 用于被跳转的目标. 例如`:x` `:y`

- 无操作(noop) 这是一个没什么用的语句, 仅仅是编译为一行noop.
  通常用来充当真正需要编译为一行无操作行的占位行.

- 设置值(set) 也是一个没什么用的语句, 就是逻辑语言原有的set.
  可能用处就是防止参数数目写错?

- 操作(Op) 这是逻辑中主要的数学运算语句, 拥有大量数字运算符.
  Bang为其提供了多种语法, 以下是一些示例
  `op add n a 1;` `op + n a 1;` `op n a add 1;` `op n a + 1;`

- 设置值2(Sets) 展开为一至多个set, 差别仅仅是可以多个并且语法为等号方便看.

  如 `a = 1;` `a b = 1 2;` `a b c = 1 2 3;`

- 块(Block) 由一对花括号包裹着的Expand.

  注: Expand语法上是零至多个行, 编译时的影响是携带一个常量表作用域.

- 内联块(InlineBlock) 和Block不同的是, 它不带有常量表作用域.

- 打印(Prints) 展开为多个print语句, 每个参数使用一个print语句.

- 操作表达式(OpExpr) 以在高级语言中较为熟悉的表达式形式展开为op set等,

  例如 `a, b = 1+2*3, a - 2;`

- 控制(Control) 一系列控制语句, 如if while switch select goto break等

- 内建指令(BuiltinCommand) 一些非控制语句的主要语句


控制 (Control)
------------------------------------------------------------------------------
### 这是一系列控制语句, 负责完成流程控制

- 跳转(Goto) 逻辑语句`jump`的封装, 根据条件跳转到标签, 条件不满足直接继续向下

  语法为关键字goto与一个标签及Cmp并以分号结束

- 中断(Break) Goto的封装, 只不过跳转目标为一个懒求值的流程控制行,
  对 while gwhile do\_while switch select 有效.

  对于 switch select 和在顶层时, 目标为结束处, 其余参考它在其它语言的作用.

  语法为关键字break与一个Cmp并以分号结束

- 继续(Continue) Goto的封装, 同Break, 但是,
  对于 switch select 和在顶层时, 目标为起始处

- 跳过(Skip) 语法为 关键字skip接上Cmp及一个行, 当条件不满足此行执行被跳过

- 循环(While) 语法为 关键字while接上Cmp及一个行, 执行此行直到条件不被满足

- do循环(DoWhile) 语法为关键字do接上一个块接上关键字while及一个Cmp与分号

  执行此行, 如果条件满足则继续执行此行

- G循环(GWhile) 语法为关键字gwhile接上Cmp及一个行,
  除了以进入此语句多执行一行的代价换来只展开一次条件外, 与While相同

- 选择(Select) 语法为关键字select接上一个Value然后是一个块.

  作用是使用Value的值去乘以块中每一条语句实际编译出的行数,
  然后使`@counter`加上它,
  以达到常数时间内直接跳转到第N行的效果(N从0开始).

  对于块中每行实际编译出行数不相同的情况下, 采用noop填充.

  注意: 当 N\<0 N\>=目标行数 N不为整数 时, 将产生未定义行为

- 分支跳转(Switch) Select的封装, 语法及用法较为复杂, 具体参考示例.

- 分支判断(If) 语法为关键字if, 接着是Cmp及一个块.

  然后是可选的多个 关键字elif接上Cmp及一个块.

  最后是可选的 关键字else接上一个行

  其作用参考其它语言.


内建指令 (BuiltinCommand)
------------------------------------------------------------------------------
### 这是一个语句分类, 如下

- 常量(Const) 产生一个常量语句, 编译到该语句时, 将会往常量表写入一个映射关系.

  其语法为 关键字const跟一个Var及一个等号和Value接着一个分号

  该常量语句会将前面的Var映射到后方Value, 并将这个映射关系写入常量表.

  在建立这个语句时, 如果后方是一个Var, 那么会先到常量表查询是否存在映射关系,
  如果存在, 那么使用映射的目标值.

  如果后方是一个ReprVar, 那么将直接将包裹的Var作为值而不进行常量表查询.

  注意, 在这个语句中, 目标Value中的被跳转标记会被记录,
  它将在展开时进行重命名以避免可能的内外标签冲突

- 拿句柄(Take) 在此处将值求出,
  并且生成一个const将一个量映射到求出的句柄以供后续使用.

  其语法描述为`"take" ("[" Value* "]")? "=" Value ";"`

  当方括号内参数至少有一个时, 它会依次向`_0` `_1` `_2` `_3` `_4`进行const参数.
  注意, 这并不与const一样会进行标签记录.

- 设置返回(SetResult) 将值take并将该语句所在的DExp的返回句柄强行设置为此值的句柄.
  这个语句是编译期作用, 所以请尽量不要往运行期作用考虑以避免误导编写者与阅读者.

  尽量编写在明显的位置, 手动保证设置前后句柄不一致带来的各种影响.
  例如原定的返回句柄在设置完值后被强行修改返回句柄.

  语法为 关键字setres接一个Value及一个分号.


比较 (Cmp)
------------------------------------------------------------------------------
### 这是比较的核心原语, 为goto条件的核心, 有以下三种

- 短路或(CmpOr) 当左操作数满足则取满足, 否则取右操作数的结果

- 短路与(CmpAnd) 当左操作数不满足则取不满足, 否则取右操作数的结果

- 条件取反(CmpNot) 用于将后方条件反转.
  反转方式是应用 [德.摩根定律] 进行表达式变换.

  例如: `!((a < b && c < d) || (e > f && g < h))`
  会被变换为: `(a >= b || c >= d) && (e <= f || g >= h)`

- 比较子(CmpAtom) 基本的比较单元, 其语法有四种, 分为两个大类:

  1. 前缀, 例如 `< a b` `== a b`
  2. 中缀, 例如 `a < b` `a == b`

  又每个大类细分为两个小类:

  1. 名称关键字, 例如 `lessThan a b` `equal a b` `a equal b`
  2. 符号关键字, 例如 `< a b` `== a b` `a == b`

  这四种写法产生相同的结果, 其诞生原因是为了照顾以原逻辑语法习惯编写的人.

- 优先级括号(CmpParen) 用于辅助编写的括号, 可以强制规定优先级与结合.
  但其并不存在于语法中, 进行编译时直接将其消去.

[德.摩根定律]: https://baike.baidu.com/item/%E5%BE%B7%C2%B7%E6%91%A9%E6%A0%B9%E5%AE%9A%E5%BE%8B/489073

### 以下为其优先级与结合性表

| 类型     | 优先级 | 结合性 | 符号          |
| ---      | ---    | ---    | ---           |
| CmpParen | -1     | -      | `(`...`)`     |
| CmpAtom  | -2     | -      | -             |
| CmpNot   | -3     | R      | `!` \| `lnot` |
| CmpAnd   | -4     | LR     | `&&`          |
| CmpOr    | -5     | LR     | `\|\|`        |

### 以下为 CmpAtom 的可用比较符号

| 符号 | 名称           |
| ---  | ---            |
| `<`  | lessThan       |
| `<=` | lessThanEq     |
| `>`  | greaterThan    |
| `>=` | greaterThanEq  |
| `==` | equal          |
| `!=` | notEqual       |
| `===`| strictEqual    |
| `!==`| strictNotEqual |
| `_`  | always         |

**注1**:
CmpAtom允许单独的一个Value, 它会被展开为这个Value不等于\`false\`

**注2**:
`a !== b` 最终编译时会被展开为
```
(op $ (op $ a === b;) == `false`;)
```