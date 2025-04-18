English README, please click on [**README-en_US.md**](./README-en_US.md)

# 学习指南
如果你想学习这门语言, 首先你需要对`Mindustry`的逻辑导出形式较为熟悉.

而如果不熟悉可以阅读或参考
[逻辑入门教程](https://github.com/A4-Tacks/learn-mindustry-logic)

还需要有一定的离开逻辑编辑器手动编写其导出形式的能力

如果你满足了上述条件, 那么你只需要阅读[学习教程](./learn.md)即可,
还可以看下面的旧版阅读索引, 比较乱和难读, 但是更全些

<details markdown='1'><summary>已经弃用的阅读索引</summary>

## 以下是推荐的阅读顺序
> [`value.mdtlbl`](./value.mdtlbl)<br/>
> [`mult_line_string.mdtlbl`](./mult_line_string.mdtlbl)<br/>
> [`dexp.mdtlbl`](./dexp.mdtlbl)<br/>
> [`print.mdtlbl`](./print.mdtlbl)<br/>
> [`op.mdtlbl`](./op.mdtlbl)<br/>
> [`op_expr.mdtlbl`](./op_expr.mdtlbl)<br/>
> [`control.mdtlbl`](./control.mdtlbl)<br/>
> [`control_plus.mdtlbl`](./control_plus.mdtlbl)<br/>
> [`control_block.mdtlbl`](./control_block.mdtlbl)<br/>
> [`cmps.mdtlbl`](./cmps.mdtlbl)<br/>
> [`insert_sort.mdtlbl`](./insert_sort.mdtlbl)<br/>
> [`switch.mdtlbl`](./switch.mdtlbl)<br/>
> [`const.mdtlbl`](./const.mdtlbl)<br/>
> [`inline_block.mdtlbl`](./inline_block.mdtlbl)<br/>
> [`take.mdtlbl`](./take.mdtlbl)<br/>
> [`compiling_eval.mdtlbl`](./compiling_eval.mdtlbl)<br/>
> [`cmp_deps.mdtlbl`](./cmp_deps.mdtlbl)<br/>
> [`switch_append.mdtlbl`](./switch_append.mdtlbl)<br/>
> [`switch_catch.mdtlbl`](./switch_catch.mdtlbl)<br/>
> [`take2.mdtlbl`](./take2.mdtlbl)<br/>
> [`gswitch.mdtlbl`](./gswitch.mdtlbl)<br/>
> [`mul_takes_and_consts.mdtlbl`](./mul_takes_and_consts.mdtlbl)<br/>
> [`cmper.mdtlbl`](./cmper.mdtlbl)<br/>
> [`setres.mdtlbl`](./setres.mdtlbl)<br/>
> [`consted_dexp.mdtlbl`](./consted_dexp.mdtlbl)<br/>
> [`quick_dexp_take.mdtlbl`](./quick_dexp_take.mdtlbl)<br/>
> [`value_bind.mdtlbl`](./value_bind.mdtlbl)<br/>
> [`dexp_binder.mdtlbl`](./dexp_binder.mdtlbl)<br/>
> [`closured_value.mdtlbl`](./closured_value.mdtlbl)<br/>
> [`caller.mdtlbl`](./caller.mdtlbl)<br/>
> [`match.mdtlbl`](./match.mdtlbl)<br/>
> [`const_match.mdtlbl`](./const_match.mdtlbl)<br/>
> [`builtin_functions.mdtlbl`](./builtin_functions.mdtlbl)<br/>
> [`value_bind_ref.mdtlbl`](./value_bind_ref.mdtlbl)<br/>

如果没有列出那请在看完上述后自行观看, 顺序可以参考文件创建顺序.

同时也有一篇[参考手册](./reference.md), 可以共同观看.

> [!WARNING]
> 上述提到的[参考手册]编写版本已经是完全过时的, 初学或许可用,
> 进阶完全不能构成语言参考来使用
>
> 且上述提到的教程目录是由远古版本一步步迭代而来, 风格非常不适合学习,
> 如果有什么疑问建议直接在讨论中询问

</details>

## 推荐的例子
有一些大型且之后编写的例子, 熟练后可以作为参考, 或截取其中部分作为工具库

- [`21point.mdtlbl`](./21point.mdtlbl)
- [`bezier_curve.mdtlbl`](./bezier_curve.mdtlbl)
- [`gravity_simulation.mdtlbl`](./gravity_simulation.mdtlbl)
- [`sine_superposition.mdtlbl`](./sine_superposition.mdtlbl)
* [`std`](./std) 部分较为通用、大型的工具
* [`for_each`](./std/for_each.mdtlbl) 方便的循环包装
* [`function.mdtlbl`](./std/function.mdtlbl) 快速生成逻辑函数
* [`stack.mdtlbl`](./std/stack.mdtlbl) 包装一个栈, 简化常用栈操作
* [`count_loop.mdtlbl`](./std/count_loop.mdtlbl) 对动态次数循环生成循环展开
* [`timeit.mdtlbl`](./std/timeit.mdtlbl) 测试执行行数, 衡量性能
