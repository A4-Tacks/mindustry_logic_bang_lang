" Vim syntax file
" Language:		mindustry_logic_bang_lang (mdtlbl)
" Maintainer:		A4-Tacks <wdsjxhno1001@163.com>
" Last Change:		2023-12-24
" URL:		https://github.com/A4-Tacks/mindustry_logic_bang_lang

" 已加载高亮时就退出
if exists("b:current_syntax")
    finish
endif

" 定义 {{{1

" 语法文件注册, 请复制到如vimrc并取消注释
"augroup filetypedetect
"    autocmd BufNewFile,BufRead *.mdtlbl setfiletype mdtlbl
"augroup END


" debug clear
"syn clear

" 大小写敏感
syn case match

" 一些关键字 {{{1
syn keyword mdtlblKeyword
            \ while gwhile do skip goto if elif else switch case break continue
            \ const take setres select
            \ inline
            \ op set noop print

syn keyword mdtlblOpFunKeyword
            \ add sub mul div idiv mod pow
            \ equal notEqual land lessThan lessThanEq greaterThan greaterThanEq
            \ strictEqual strictNotEqual shl shr or and xor max
            \ min angle len noise not abs log
            \ floor ceil sqrt rand sin cos tan
            \ asin acos atan lnot

syn match mdtlblCmpTreeOper /&&\|||\|!/

" 注释 {{{1
syn region mdtlblComment start=/#/ end=/$/ oneline
syn region mdtlblLongComment start=/#\*/ end=/\*#/ fold

setlocal comments=s:#*,mb:*,ex:*#,:#
setlocal commentstring=#%s
setlocal formatoptions+=rq

" 值(Var) {{{1
syn match mdtlblStringFailedEscape /\\./ contained
syn match mdtlblStringColor /\[\v%(#\x{6,8}|%(c%(lear|yan|oral)|b%(l%(ack|ue)|r%(own|ick))|white|li%(ghtgray|me)|g%(r%(ay|een)|old%(enrod)?)|darkgray|navy|r%(oyal|ed)|s%(late|ky|carlet|almon)|t%(eal|an)|acid|forest|o%(live|range)|yellow|p%(ink|urple)|ma%(genta|roon)|violet))\]/ contained
syn match mdtlblSpecialChar /^ *\\ \|\\\%([n\\[]\|$\)/ contained
syn region mdtlblString start=/"/ end=/"/ contains=mdtlblSpecialChar,mdtlblStringFailedEscape,mdtlblStringColor

syn match mdtlblOIdent /@\I\i*\%(-\i*\)*/
syn match mdtlblOtherVar /'[^' \t]\+'/
syn match mdtlblNumber /\v(<0%(x\-?[0-9a-fA-F][0-9a-fA-F_]*|b\-?[01][_01]*)|\-?<\d[0-9_]*%(\.\d[0-9_]*|e[+\-]?\d[0-9_]*)?)>/
syn match mdtlblBoolean /\v<%(true|false)>/
syn match mdtlblNull /\<null\>/

syn match mdtlblResultHandle /\$/

" Label {{{1
syn match mdtlblDefineResultHandle /\%((\%(\s\|#\*.*\*#\|\%(#[^*].*\|#\)\=\n\)*\)\@<=\I\i*:/

syn match mdtlblQuickDExpTakeIdent /\I\i*\%(\%(\s\|#\*.*\*#\|\%(#[^*].*\|#\)\=\n\)*\[\)\@=/
syn match mdtlblIdentLabel /:\I\i*/

setlocal foldmethod=syntax
syn region mdtlblBlock start=/{/ end=/}/ transparent fold
syn region mdtlblDExp start=/(/ end=/)/ transparent fold
syn region mdtlblArgs matchgroup=mdtlblArgsBracket start=/\[/ end=/\]/ transparent fold

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

function! <SID>getMdtlblIndent()
    if v:lnum <= 1 | return 0 | endif
    let lnum = v:lnum
    let pnum = prevnonblank(lnum - 1)
    let p2num = prevnonblank(pnum - 1)

    let line = <SID>lineFilter(getline(lnum))
    let preline = <SID>lineFilter(getline(pnum))
    let pre2line = <SID>lineFilter(getline(p2num))

    let diff = 0

    if preline =~# '\([({[:]\|\<\(else\)\>\)$'
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

setlocal indentexpr=<SID>getMdtlblIndent()
setlocal indentkeys+==case
setlocal indentkeys+==}
setlocal indentkeys+==)
setlocal indentkeys+==:

" END And Color Links {{{1
hi def link mdtlblKeyword Keyword
hi def link mdtlblOpFunKeyword Operator
hi def link mdtlblCmpTreeOper Operator
hi def link mdtlblComment Comment
hi def link mdtlblLongComment Comment
hi def link mdtlblStringFailedEscape Error
hi def link mdtlblStringColor Include
hi def link mdtlblSpecialChar SpecialChar
hi def link mdtlblString String
hi def link mdtlblOIdent Identifier
hi def link mdtlblOtherVar Identifier
hi def link mdtlblNumber Number
hi def link mdtlblBoolean Boolean
hi def link mdtlblNull Boolean
hi def link mdtlblResultHandle Identifier
hi def link mdtlblDefineResultHandle Identifier
hi def link mdtlblIdentLabel Label
hi def link mdtlblArgsBracket Macro
hi def link mdtlblQuickDExpTakeIdent Macro
" }}}1
