" Vim syntax file
" Language:		mindustry_logic_bang_lang (mdtlbl)
" Maintainer:		A4-Tacks <wdsjxhno1001@163.com>
" Last Change:		2023-8-24
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

" 一些关键字 {{{1
syn keyword mdtlblKeyword
            \ while gwhile do skip goto if elif else switch case break continue
            \ const take setres select
            \ inline
            \ op set noop print
hi link mdtlblKeyword Keyword

syn keyword mdtlblOpFunKeyword
            \ add sub mul div idiv mod pow
            \ equal notEqual land lessThan lessThanEq greaterThan greaterThanEq
            \ strictEqual strictNotEqual shl shr or and xor max
            \ min angle len noise not abs log
            \ floor ceil sqrt rand sin cos tan
            \ asin acos atan lnot
hi link mdtlblOpFunKeyword Operator

syn match mdtlblCmpTreeOper /&&\|||\|!/
hi link mdtlblCmpTreeOper Operator

" 注释 {{{1
syn region mdtlblComment start=/#/ end=/$/
syn region mdtlblLongComment start=/#\*/ end=/\*#/
hi link mdtlblComment Comment
hi link mdtlblLongComment Comment

setlocal comments=s:#*,mb:*,ex:*#,:#
setlocal commentstring=#%s
setlocal formatoptions+=rq


" 值(Var) {{{1
syn match mdtlblSpecialChar /\\n/ contained
hi link mdtlblString String

syn region mdtlblString start=/"/ end=/"/ contains=mdtlblSpecialChar
hi link mdtlblSpecialChar SpecialChar

syn match mdtlblOIdent /@\I\i*\(-\i*\)*/
hi link mdtlblOIdent Identifier

syn match mdtlblOtherVar /'[^' \t]\+'/
hi link mdtlblOtherVar Identifier

syn match mdtlblNumber /\v(<0(x\-?[0-9a-fA-F][0-9a-fA-F_]*|b\-?[01][_01]*)|\-?<[0-9][0-9_]*(\.[0-9][0-9_]*)?)>/
hi link mdtlblNumber Number

syn match mdtlblBoolean /\v<true|false>/
hi link mdtlblBoolean Boolean

syn match mdtlblNull /\<null\>/
hi link mdtlblNull Boolean

syn match mdtlblResultHandle /\$/
hi link mdtlblResultHandle Identifier


" Label {{{1
syn match mdtlblDefineResultHandle /\I\i*:/
hi link mdtlblDefineResultHandle Identifier

syn match mdtlblIdentLabel /:\I\i*/
hi link mdtlblIdentLabel Label

setlocal foldmethod=syntax
syn region mdtlblBlock start=/{/ end=/}/ transparent fold
syn region mdtlblDExp start=/(/ end=/)/ transparent fold
syn region mdtlblArgs start=/\[/ end=/\]/ transparent fold

" Indent (缩进控制) {{{1

function! <SID>lineFilter(line)
    " 过滤掉注释与字符串与原始标识符
    let regex_a = ''
                \. '#\*.\{-0,}\*#'
                \. '\|#.*$'
    let regex_b = '@\I\i*\(-\i*\)*'
                \. '\|' . "'[^' \\t]*'"
                \. '\|"[^"]*"'
    let line = substitute(a:line, regex_a, '', 'g')
    return trim(substitute(line, regex_b, '_', 'g'))
endfunction

function! GetMdtlblIndent()
    if v:lnum <= 1 | return 0 | endif
    let lnum = v:lnum
    let pnum = prevnonblank(lnum - 1)
    let p2num = prevnonblank(pnum - 1)

    let line = <SID>lineFilter(getline(lnum))
    let preline = <SID>lineFilter(getline(pnum))
    let pre2line = <SID>lineFilter(getline(p2num))

    let diff = 0

    if preline =~# '\([({\[:]\|\<\(else\)\>\)$'
        let diff += 1
    endif

    if line =~# '\(^[)}\]]\|\<case\>\)' && !(preline =~# '\<case\>' && preline !~# ':$')
        let diff -= 1
    endif

    if pre2line =~# 'else$'
        let diff -= 1
    endif

    return indent(pnum) + diff * &shiftwidth
endfunction

setlocal indentexpr=GetMdtlblIndent()
setlocal indentkeys+==case
setlocal indentkeys+==}
setlocal indentkeys+==)
setlocal indentkeys+==:

" END {{{1
" }}}1
