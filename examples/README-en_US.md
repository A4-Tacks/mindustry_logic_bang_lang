# Study Guide
If you want to learn Bang language,
first you need to be familiar with the logical exported syntax of Mindustry logical editor

You also need to have the ability to manually write its exported syntax outside the logic editor

If you are not familiar with the syntax exported by the logic editor,
[here](https://github.com/A4-Tacks/learn-mindustry-logic) is a tutorial written in Chinese,
and you may be able to find more suitable resources than it

If you meet the above conditions, then you only need to start reading from [Learning Tutorial](./learn-en_US.md)

<details markdown='1'><summary>Deprecated reading index</summary>

## The following is the recommended reading order
> [`value.mdtlbl`](./syntax/value.mdtlbl)<br/>
> [`mult_line_string.mdtlbl`](./syntax/mult_line_string.mdtlbl)<br/>
> [`dexp.mdtlbl`](./syntax/dexp.mdtlbl)<br/>
> [`print.mdtlbl`](./syntax/print.mdtlbl)<br/>
> [`op.mdtlbl`](./syntax/op.mdtlbl)<br/>
> [`op_expr.mdtlbl`](./syntax/op_expr.mdtlbl)<br/>
> [`control.mdtlbl`](./syntax/control.mdtlbl)<br/>
> [`control_plus.mdtlbl`](./syntax/control_plus.mdtlbl)<br/>
> [`control_block.mdtlbl`](./syntax/control_block.mdtlbl)<br/>
> [`cmps.mdtlbl`](./syntax/cmps.mdtlbl)<br/>
> [`insert_sort.mdtlbl`](./projects/algorithms/insert_sort.mdtlbl)<br/>
> [`switch.mdtlbl`](./syntax/switch.mdtlbl)<br/>
> [`const.mdtlbl`](./syntax/const.mdtlbl)<br/>
> [`inline_block.mdtlbl`](./syntax/inline_block.mdtlbl)<br/>
> [`take.mdtlbl`](./syntax/take.mdtlbl)<br/>
> [`compiling_eval.mdtlbl`](./syntax/compiling_eval.mdtlbl)<br/>
> [`cmp_deps.mdtlbl`](./syntax/cmp_deps.mdtlbl)<br/>
> [`switch_append.mdtlbl`](./syntax/switch_append.mdtlbl)<br/>
> [`switch_catch.mdtlbl`](./syntax/switch_catch.mdtlbl)<br/>
> [`take2.mdtlbl`](./syntax/take2.mdtlbl)<br/>
> [`gswitch.mdtlbl`](./syntax/gswitch.mdtlbl)<br/>
> [`mul_takes_and_consts.mdtlbl`](./syntax/mul_takes_and_consts.mdtlbl)<br/>
> [`cmper.mdtlbl`](./syntax/cmper.mdtlbl)<br/>
> [`setres.mdtlbl`](./syntax/setres.mdtlbl)<br/>
> [`consted_dexp.mdtlbl`](./syntax/consted_dexp.mdtlbl)<br/>
> [`quick_dexp_take.mdtlbl`](./syntax/quick_dexp_take.mdtlbl)<br/>
> [`value_bind.mdtlbl`](./syntax/value_bind.mdtlbl)<br/>
> [`dexp_binder.mdtlbl`](./syntax/dexp_binder.mdtlbl)<br/>
> [`closured_value.mdtlbl`](./syntax/closured_value.mdtlbl)<br/>
> [`caller.mdtlbl`](./syntax/caller.mdtlbl)<br/>
> [`match.mdtlbl`](./syntax/match.mdtlbl)<br/>
> [`const_match.mdtlbl`](./syntax/const_match.mdtlbl)<br/>
> [`builtin_functions.mdtlbl`](./syntax/builtin_functions.mdtlbl)<br/>
> [`value_bind_ref.mdtlbl`](./syntax/value_bind_ref.mdtlbl)<br/>

If it is not listed in the above list,
you can watch it yourself after reading the above content.
The reading order can refer to the file creation order

There is also a [reference](./legacy/reference.md) manual,
You can read together with the above content

> [!WARNING]
> The version of the reference manual mentioned above is completely outdated.
> It may be useful for beginners, but advanced usage cannot constitute a language reference for use
>
> And the tutorial directory mentioned above is iterated step by step from ancient versions,
> and its style is very unsuitable for learning
>
> If you have any questions,
> it is recommended to ask directly in the issues and discussions

</details>

## Recommended examples
There are some large and advanced complex examples that can be used as references
or pasted into your code for quick and convenient use

- [`21point.mdtlbl`](./projects/games/21point.mdtlbl)
- [`bezier_curve.mdtlbl`](./projects/display/bezier_curve.mdtlbl)
- [`gravity_simulation.mdtlbl`](./projects/display/gravity_simulation.mdtlbl)
- [`sine_superposition.mdtlbl`](./projects/display/sine_superposition.mdtlbl)
* [`std`](./std) Some of the more general and large tools
* [`for_each`](./std/for_each.mdtlbl) Exquisite `for-each` implementation
* [`function.mdtlbl`](./std/function.mdtlbl) Quickly generate non recursive functions
* [`stack.mdtlbl`](./std/stack.mdtlbl) Packaging a stack to simplify common stack operations
* [`count_loop.mdtlbl`](./std/count_loop.mdtlbl) Generate loop expansion for dynamic count
* [`timeit.mdtlbl`](./std/timeit.mdtlbl) Test execution lines to measure performance
* [`sync.mdtlbl`](./std/sync.mdtlbl) Mutex lock, for shared mutable data of multiple processor block

## Simple Attempt
If you feel that Bang language is too complex or does not require the capabilities it provides,
you can try some of the additional features of this compiler

### About Logical Language

- Rename label: `mindustry_logic_bang_lang in`
- Convert absolute address into label: `mindustry_logic_bang_lang i`
- Some simplify variable check: `mindustry_logic_bang_lang l`
- Extract and build op statements: `mindustry_logic_bang_lang b`

### About Paren Language
This is a lightweight logical language extension,
that only provides the feature of embedding multiple statement return values into variables

A simplified version similar to DExp in Bang

The variable name starting with `$` in parentheses will be treated as the return variable of the current parentheses.

If the variable name is empty, a new anonymous variable will be created

For more examples, please refer to [`mini_paren.logic`](./mini_paren.logic)

Compile it using `mindustry_logic_bang_lang p`
