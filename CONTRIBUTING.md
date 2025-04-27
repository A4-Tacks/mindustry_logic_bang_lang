对于所有文本文件:

- 不要留下任何位于行末的空白字符, 即不与正则 `/[ \t\r]+$/` 匹配

- 保持在文件末尾具有一个换行, 即完整的 end of line

-------------------------------------------------------------------------------

如果涉及了 Rust 代码, 那么请尽量保持与原项目一致的代码风格:

- 让几乎所有代码保持在不超过 79 列的宽度, 含缩进,
  且该宽度尽量考虑 rust-analyzer 的 InlayHints

- 编写 `where` 约束时, 不在 `where` 后换行,
  且多个约束以 6 空格缩进与首个约束对齐, 每一个约束都以逗号结束

- `match` 匹配臂中, 必须以逗号结尾, 哪怕是一个代码块

- 对于大闭包代码块, 可以使用负缩进, 例如:
  ```rust
  x.into_iter()
      .for_each(|elem|
  {
      println!("{elem}");
  });
  ```
  尽量只在链式调用末尾处使用

-------------------------------------------------------------------------------

如果修改了文法定义, 既 `./tools/parser/src/parser.lalrpop`,
请至少完成一次完整的编译以验证文法, 这需要的时间可能有点长

如果修改后的文法存在二义性, 例如移入/规约冲突, 可能编译它是一个不理智的行为,
考虑单独编译并使用 [lalrpop] 的命令行工具来生成代码, 来及时捕获二义性并做出修改

```bash
lalrpop tools/parser/src/parser.lalrpop && cargo test --workspace
```

请确保将整个工作空间的测试全部运行完毕并通过, 并对新的修改编写新的测试

[lalrpop]: https://github.com/lalrpop/lalrpop

-------------------------------------------------------------------------------

如果修改了代码片段, 请使用 [make-snippets.jq] 同步更新 vscode 扩展的代码片段

```bash
jq --indent 4 -nRf syntax/vscode/make-snippets.jq < syntax/vim/UltiSnips/mdtlbl.snippets > syntax/vscode/support/snippets/snippets.json
```

[make-snippets.jq]: ./syntax/vscode/make-snippets.jq

-------------------------------------------------------------------------------

如果修改了用于 MT-管理器 的逻辑语法高亮相关,
请使用 [mtsyntax-plus] 工具, 来更新编译后版本

```bash
mtsyntax-plus < syntax/MT-Manager/MindustryLogic.mtsx > syntax/MT-Manager/MindustryLogic-compiled.mtsx
```

[mtsyntax-plus]: https://github.com/A4-Tacks/mtsyntax-plus

-------------------------------------------------------------------------------

如果翻译教程, 请注意关于一些作者扭曲的用词问题, 例如:

- 求值 -> take (拿取)
- 评估 -> eval (求值/评估)
- 量 -> Var (变量)
- 原始量 -> Repr Var (布局变量)
- 返回句柄 -> Result Handle (结果句柄)

这类术语因为一些特殊原因与英文意思不能直接对照, 需要单独替换成别的词再做翻译
