# 语法示例 / Syntax examples
这里收录按语法点拆分的小示例, 每个文件只讲一个特性,
文件头部的文档注释写明了该特性的引入版本

Short single-feature examples, one feature per file.
Each file's doc comment notes the version that introduced it.

阅读顺序沿用 [示例 README](../README.md) 里给出的推荐顺序,
主线教程是 [learn.md](../learn.md)

The order below follows the reading index in the
[example README](../README.md); the main tutorial is [learn.md](../learn.md).

## 基本单元 / Basics
> [`value.mdtlbl`](./value.mdtlbl) 量与语句的基本形式 / Basic Var and statement forms<br/>
> [`mult_line_string.mdtlbl`](./mult_line_string.mdtlbl) 多行字符串 (0.13.9) / Multiline string (0.13.9)<br/>
> [`dexp.mdtlbl`](./dexp.mdtlbl) 内联表达式 DExp / Inline expression DExp<br/>
> [`print.mdtlbl`](./print.mdtlbl) print 展开成多条打印 / print expanded into rows<br/>
> [`op.mdtlbl`](./op.mdtlbl) 同一结果的多种 op 写法 / Equivalent op spellings<br/>
> [`op_expr.mdtlbl`](./op_expr.mdtlbl) op 表达式系统 (0.11.0) / op-expr system (0.11.0)<br/>
> [`setres.mdtlbl`](./setres.mdtlbl) 返回句柄 setres / Result handle setres<br/>

## 控制流 / Control flow
> [`control.mdtlbl`](./control.mdtlbl) 控制语句与其 Cmp 条件 / Control rows and their Cmp<br/>
> [`control_plus.mdtlbl`](./control_plus.mdtlbl) 免标签跳出与继续 (0.11.7) / Label-free break and continue (0.11.7)<br/>
> [`control_block.mdtlbl`](./control_block.mdtlbl) 自定义跳出点 (0.12.8) / Custom break targets (0.12.8)<br/>
> [`cmps.mdtlbl`](./cmps.mdtlbl) 复合条件 (0.7.0) / Composite comparison (0.7.0)<br/>
> [`cmper.mdtlbl`](./cmper.mdtlbl) JumpCmp 作为值 (0.13.0) / JumpCmp used as a value (0.13.0)<br/>
> [`switch.mdtlbl`](./switch.mdtlbl) 常量时间整数分支 / Constant-time integer branch<br/>
> [`switch_append.mdtlbl`](./switch_append.mdtlbl) switch 追加语法 (0.4.1) / switch append (0.4.1)<br/>
> [`switch_catch.mdtlbl`](./switch_catch.mdtlbl) switch 捕获 / switch catch<br/>
> [`gswitch.mdtlbl`](./gswitch.mdtlbl) 与常量和参数交互的 switch (0.16.3) / switch with const and params (0.16.3)<br/>

## 常量与求值 / Const and evaluation
> [`const.mdtlbl`](./const.mdtlbl) 内联常量 / Inline const<br/>
> [`inline_block.mdtlbl`](./inline_block.mdtlbl) 不建块作用域的行单元 (0.11.1) / Blockless inline rows (0.11.1)<br/>
> [`take.mdtlbl`](./take.mdtlbl) 求值语句 take / take statement<br/>
> [`take2.mdtlbl`](./take2.mdtlbl) take 的更多形式 (0.5.0) / More take forms (0.5.0)<br/>
> [`compiling_eval.mdtlbl`](./compiling_eval.mdtlbl) 编译期计算 (0.13.6) / Compile-time eval (0.13.6)<br/>
> [`cmp_deps.mdtlbl`](./cmp_deps.mdtlbl) 比较依赖语句 (0.12.0) / Comparison dependency (0.12.0)<br/>
> [`consted_dexp.mdtlbl`](./consted_dexp.mdtlbl) 常量化的 DExp (0.11.3) / Consted DExp (0.11.3)<br/>
> [`quick_dexp_take.mdtlbl`](./quick_dexp_take.mdtlbl) 快捷 DExp 求值 / Quick DExp take<br/>

## 绑定与闭包 / Binding and closure
> [`value_bind.mdtlbl`](./value_bind.mdtlbl) 值绑定 (0.10.0) / Value bind (0.10.0)<br/>
> [`value_bind_ref.mdtlbl`](./value_bind_ref.mdtlbl) 追溯时取绑定值 (0.15.3) / Binding ref on follow (0.15.3)<br/>
> [`dexp_binder.mdtlbl`](./dexp_binder.mdtlbl) 为 const 记录绑定者 (0.14.4) / Const binder records (0.14.4)<br/>
> [`closured_value.mdtlbl`](./closured_value.mdtlbl) 闭包值 (0.15.0) / ClosuredValue (0.15.0)<br/>
> [`caller.mdtlbl`](./caller.mdtlbl) 抑制 const-dexp 膨胀的调用包装 / Caller wrapper against DExp bloat<br/>
> [`mul_takes_and_consts.mdtlbl`](./mul_takes_and_consts.mdtlbl) 一句多个 take 与 const (0.13.1) / Many take and const in one row (0.13.1)<br/>

## 匹配 / Match
> [`match.mdtlbl`](./match.mdtlbl) 模式匹配 (0.14.0) / Pattern match (0.14.0)<br/>
> [`const_match.mdtlbl`](./const_match.mdtlbl) 常量匹配 (0.16.0) / const-match (0.16.0)<br/>
> [`builtin_functions.mdtlbl`](./builtin_functions.mdtlbl) 内建函数绑定 (0.14.6) / Builtin bindings (0.14.6)<br/>

## 结构与总览 / Structures and overview
> [`once.mdtlbl`](./once.mdtlbl) 仅展开一次的包装器 / Expand-once wrapper<br/>
> [`iter.mdtlbl`](./iter.mdtlbl) 内存与迭代器 / Memory and iterator<br/>
> [`all_syntax.mdtlbl`](./all_syntax.mdtlbl) 近乎用满全部语法的程序 / Nearly all syntax in one program<br/>
