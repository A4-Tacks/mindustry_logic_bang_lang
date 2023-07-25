" Vim syntax file
" Language:		mindustry_logic_bang_lang (mdtlbl)
" Maintainer:		A4-Tacks <wdsjxhno1001@163.com>
" Last Change:		2023-7-25
" URL:		https://github.com/A4-Tacks/mindustry_logic_bang_lang

" 已加载高亮时就退出
if exists("b:current_syntax")
    finish
endif

" 语法文件注册, 请复制到如vimrc并取消注释 {{{1
"augroup filetypedetect
"    autocmd BufNewFile,BufRead *.mdtlbl setfiletype mdtlbl
"augroup END


" debug clear {{{1
"syn clear


" 大小写敏感 {{{1
syn case match


" 控制语句 {{{1
syn keyword mdtlblKeyword while do skip goto if elif else switch case
hi link mdtlblKeyword Keyword


" 注释 {{{1
syn region mdtlblComment start=/#/ end=/$/
syn region mdtlblLongComment start=/#\*/ end=/\*#/
hi link mdtlblComment Comment
hi link mdtlblLongComment Comment


" 值(Var) {{{1
syn match mdtlblSpecialChar /\\n/ contained
hi link mdtlblString String

syn region mdtlblString start=/"/ end=/"/ contains=mdtlblSpecialChar
hi link mdtlblSpecialChar SpecialChar

syn match mdtlblOIdent /@\I\i*\(-\i*\)*/
hi link mdtlblOIdent Identifier

syn match mdtlblOtherValue /'[^'\s]\+'/
hi link mdtlblOtherValue Identifier

syn match mdtlblNumber /\v<(0(x\-?[0-9a-fA-F][0-9a-fA-F_]*|b\-?[01][_01]*)|\-?[0-9][0-9_]*(\.[0-9][0-9_]*)?)>/
hi link mdtlblNumber Number


" Label {{{1
syn match mdtlblIdentLabel /:\I\i*/
hi link mdtlblIdentLabel Label

" END {{{1
" }}}1
