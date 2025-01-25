Add to your vim plugin system, e.g vim-plug

Snippets from [UltiSnips](./UltiSnips/),
coc-snippets maybe auto load

# Example

```sh
ln -s /path/to/mindustry_logic_bang_lang/syntax/vim ~/.vim/plugged/mdtlbl.vim
```
And
```vimscript
call plug#begin('~/.vim/plugged')
Plug './mdtlbl.vim'
call plug#end()
```
