bangls 是一个 bang 语言的语言服务器, 使用语言服务器协议 (LSP)

使用支持 LSP 的编辑器可以利用语言服务器提供某语言的补全、跳转等功能, 例如:

- VSCode: LSP 始祖, 虽然特性支持经常还不如其它编辑器
- Vim/NeoVim: 通常安装插件后可支持 LSP

## 在 VSCode 中

将 `bangls` 或 `bangls.exe` 可执行文件放入 `$PATH` `%PATH%` 等环境变量中包含的目录, 安装插件即可

语言服务器及 VSCode 插件 (.vsix) 都在 [Releases] 中获取

(由于 VSCode 特性支持较差, 暂不支持使用代码操作进行编译)

## 在 Vim/NeoVim 中使用 coc.nvim

```json
{
    "languageserver": {
        "bangls": {
            "command": "bangls",
            "filetypes": ["mdtlbl"]
        }
    }
}
```

即可以在 `.mdtlbl` 后缀的文件中具有补全等功能

[Releases]: https://github.com/A4-Tacks/mindustry_logic_bang_lang/releases


功能支持
===============================================================================
- [x] 基本补全
- [x] 实时分析
- [x] 报错显示
- [x] 启发式片段补全
- [ ] 定义跳转
- [ ] 引用跳转
- [x] 文档查看
