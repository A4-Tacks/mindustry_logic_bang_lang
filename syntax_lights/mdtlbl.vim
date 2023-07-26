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
syn keyword mdtlblKeyword while do skip goto if elif else switch case const take
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

syn match mdtlblResultHandle /\$/
hi link mdtlblResultHandle Identifier


" Label {{{1
syn match mdtlblDefineResultHandle /\I\i*\s*:/
hi link mdtlblDefineResultHandle Identifier

syn match mdtlblIdentLabel /:\s*\I\i*/
hi link mdtlblIdentLabel Label

setlocal foldmethod=syntax
syn region mdtlblBlock start=/{/ end=/}/ transparent fold
syn region mdtlblDExp start=/(/ end=/)/ transparent fold

function! <SID>lineFilter(line)
    " 过滤掉注释与字符串与原始标识符
    let regex = '\('
                \. '#\*.\{-0,}\*#'
                \. '\|#.*$'
                \. '\|' . "'[^'\\S]*'"
                \. '\)'
    return substitute(a:line, regex, '_', 'g')
endfunction

function! GetMdtlblIndent()
    if v:lnum <= 1 | return 0 | endif
    let lnum = v:lnum
    let pnum = prevnonblank(lnum - 1)

    let preline = <SID>lineFilter(getline(pnum))
    let line = <SID>lineFilter(getline(lnum))

    if line =~# 'case' && preline =~# '\<switch\>'
        " 拦截, 无需缩进
        let diff = 0
    elseif line =~# '\(^\s*[)}]\|\<case\>\)'
        let diff = -1
    elseif preline =~# '[({:]\s*$' && line !~# '\<case\>'
        let diff = 1
    else
        let diff = 0
    endif

    return indent(pnum) + diff * &shiftwidth
endfunction

setlocal indentexpr=GetMdtlblIndent()

" END {{{1
" }}}1
