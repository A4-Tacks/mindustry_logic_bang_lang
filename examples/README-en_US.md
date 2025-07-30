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

If it is not listed in the above list,
you can watch it yourself after reading the above content.
The reading order can refer to the file creation order

There is also a [reference](./reference.md) manual,
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

- [`21point.mdtlbl`](./21point.mdtlbl)
- [`bezier_curve.mdtlbl`](./bezier_curve.mdtlbl)
- [`gravity_simulation.mdtlbl`](./gravity_simulation.mdtlbl)
- [`sine_superposition.mdtlbl`](./sine_superposition.mdtlbl)
* [`std`](./std) Some of the more general and large tools
* [`for_each`](./std/for_each.mdtlbl) Exquisite `for-each` implementation
* [`function.mdtlbl`](./std/function.mdtlbl) Quickly generate non recursive functions
* [`stack.mdtlbl`](./std/stack.mdtlbl) Packaging a stack to simplify common stack operations
* [`count_loop.mdtlbl`](./std/count_loop.mdtlbl) Generate loop expansion for dynamic count
* [`timeit.mdtlbl`](./std/timeit.mdtlbl) Test execution lines to measure performance

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
