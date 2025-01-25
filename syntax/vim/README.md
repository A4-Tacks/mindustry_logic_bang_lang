`.mdtlbl` (Mindustry Logic Bang Lang) and `.logic` (Mindustry Logic) vim supports

# Install
Add to your vim plugin system, e.g vim-plug

Snippets from [UltiSnips](./UltiSnips/),
coc-snippets maybe auto load


# Features
- Input single quote in word end, to **Quick Take**, `Foo'` -> `Foo! ;`
- Input single quote not in word end, to two single quote, `'` -> `''`
- Write buffer auto compile to clipboard (*linux only*)

  Required `mindustry_logic_bang_lang` directory in `$PATH`,
  default `g:mdtlbl_clipboard_command` = `| xclip -se c`


# Example Install Steps
**vim-plug**:

```sh
ln -s /path/to/mindustry_logic_bang_lang/syntax/vim ~/.vim/plugged/mdtlbl.vim
```

And add to your vimrc

```vimscript
call plug#begin('~/.vim/plugged')
Plug './mdtlbl.vim'
call plug#end()
```
