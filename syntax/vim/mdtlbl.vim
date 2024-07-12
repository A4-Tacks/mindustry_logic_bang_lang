" Vim syntax file
" Language:		mindustry_logic_bang_lang (mdtlbl)
" Maintainer:		A4-Tacks <wdsjxhno1001@163.com>
" Last Change:		2024-05-27
" URL:			https://github.com/A4-Tacks/mindustry_logic_bang_lang
scriptencoding utf-8

" 已加载高亮时就退出
if exists('b:current_syntax')
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
            \ while gwhile do skip if elif else switch gswitch break continue
            \ const take setres select match
            \ inline
            \ op noop print
syn keyword mdtlblKeyword goto nextgroup=mdtlblIdentLabelRest
syn keyword mdtlblKeyword case nextgroup=mdtlblStar skipwhite
syn match mdtlblStar /\*/ contained

syn keyword mdtlblOpFunKeyword
            \ add sub mul div idiv mod pow
            \ equal notEqual land lessThan lessThanEq greaterThan greaterThanEq
            \ strictEqual strictNotEqual always _
            \ shl shr or and xor max
            \ min angle len noise not abs log
            \ floor ceil sqrt rand sin cos tan
            \ asin acos atan lnot

syn match mdtlblOpExprCtrl "\%(//\|\*\*\|&&\|<<\|>>\|[+\-*/%|&^]\)\=="
syn match mdtlblOpExprCtrl /++\|--/
syn match mdtlblCmpTreeOper /&&\|||\|!\|=>/
syn match mdtlblCmpOper /[<>]=\=\|[=!]==\=/
syn match mdtlblArgsExpand /@/

" 注释 {{{1
syn region mdtlblComment	start=/#\%([^*]\|$\)/	end=/$/		contains=mdtlblCommentMeta oneline
syn region mdtlblLongComment	start=/#\*/		end=/\*#/	contains=mdtlblCommentMeta fold
syn keyword mdtlblCommentMeta Todo TODO Note NOTE Hint HINT

setlocal comments=s:#*,mb:*,ex:*#,:#
setlocal commentstring=#%s
setlocal formatoptions+=rq

" 值(Var) {{{1
syn match	mdtlblStringFailedEscape /\\\%("\@=\|.\)/	contained
syn match	mdtlblStringColor				contained /\[\v%(#\x{6,8}|%(c%(lear|yan|oral)|b%(l%(ack|ue)|r%(own|ick))|white|li%(ghtgray|me)|g%(r%(ay|een)|old%(enrod)?)|darkgray|navy|r%(oyal|ed)|s%(late|ky|carlet|almon)|t%(eal|an)|acid|forest|o%(live|range)|yellow|p%(ink|urple)|ma%(genta|roon)|violet))=\]/
syn match	mdtlblSpecialChar /^ *\\ \|\\\%([n\\[]\|$\)/	contained
syn cluster	mdtlblStringContains				contains=mdtlblSpecialChar,mdtlblStringFailedEscape,mdtlblStringColor
syn region	mdtlblString start=/"/ end=/"/			contains=@mdtlblStringContains

syn match mdtlblOIdent /@\I\i*\%(-\i*\)*/
syn match mdtlblOtherVar /'[^' \t]\+'/ contains=mdtlblStringColor
syn match mdtlblNumber /\v(<0%(x\-?[0-9a-fA-F][0-9a-fA-F_]*|b\-?[01][_01]*)|\-?<\d[0-9_]*%(\.\d[0-9_]*|e[+-]?\d[0-9_]*)?)>/
syn match mdtlblBoolean /\v<%(true|false)>/
syn match mdtlblNull /\<null\>/

syn match mdtlblResultHandle /\$/

" Label And ResultH {{{1
syn match mdtlblDefineResultHandle /\v%(\([%?]=)@2<=-=_@![0-9_]+%(\._@![0-9_]+|e[+-]=-=_@![0-9_]+)=>:/
syn match mdtlblDefineResultHandle /\v%(\([%?]=)@2<=0%(x-=_@![0-9a-fA-F_]+|b-=_@![01_]+)>:/
syn match mdtlblDefineResultHandle /\v%(\([%?]=)@2<=%(\I\i*|\@\I\i*%(-\i*)*|'[^' \t]+'):/  contains=mdtlblStringColor
syn match mdtlblDefineResultHandle /\v%(\([%?]=)@2<="[^"]*":/ contains=@mdtlblStringContains

syn match mdtlblQuickDExpTakeIdent /\v\@\I\i*%(-\i*)*%(%(-\>)=\[)@=/
syn match mdtlblQuickDExpTakeIdent /\v\I\i*%(%(-\>)=\[)@=/
syn match mdtlblQuickDExpTakeIdent /\v'[^' \t]+'%(%(-\>)=\[)@=/
syn match mdtlblQuickDExpTakeIdent /->/

syn match  mdtlblIdentLabel /\v%(^|\W@1<=):%(\I\i*|\@\I\i*%(-\i*)*|'[^' \t]+')/			nextgroup=mdtlblIdentLabelRest		contains=mdtlblStringColor
syn match  mdtlblIdentLabel /\v%(^|\W@1<=):-=_@![0-9_]+%(\._@![0-9_]+|e[+-]=-=_@![0-9_]+)=>/	nextgroup=mdtlblIdentLabelRest
syn match  mdtlblIdentLabel /\v%(^|\W@1<=):0%(x-=_@![0-9a-fA-F_]+|b-=_@![01_]+)>/		nextgroup=mdtlblIdentLabelRest
syn region mdtlblIdentLabel start=/\v%(^|\W@1<=):"/ end=/"/					contains=@mdtlblStringContains

syn match  mdtlblIdentLabelRest /\v:%(\I\i*|\@\I\i*%(-\i*)*|'[^' \t]+')/		nextgroup=mdtlblIdentLabelRest contained	contains=mdtlblStringColor
syn match  mdtlblIdentLabelRest /\v:-=_@![0-9_]+%(\._@![0-9_]+|e[+-]=-=_@![0-9_]+)=>/	nextgroup=mdtlblIdentLabelRest contained
syn match  mdtlblIdentLabelRest /\v:0%(x-=_@![0-9a-fA-F_]+|b-=_@![01_]+)>/		nextgroup=mdtlblIdentLabelRest contained
syn region mdtlblIdentLabelRest start=/:"/ end=/"/					contains=@mdtlblStringContains contained

" Fold {{{1
setlocal foldmethod=indent
"syn region mdtlblBlock					start=/{/	end=/}/		transparent
"syn region mdtlblDExp					start=/(\[\@!/	end=/)/		transparent
syn region mdtlblArgs	matchgroup=mdtlblArgsBracket	start=/(\@<!\[/	end=/]/		transparent
"syn region mdtlblClos					start=/(\[\@=/	end=/)/		transparent
syn region mdtlblClos					start=/(\@<=\[/	end=/]/		transparent

" Indent (缩进控制) {{{1
function! <SID>lineFilter(line)
    " 过滤掉注释与字符串与原始标识符
    let regex_a = ''
                \. '#\*.\{-}\*#'
                \. '\|#.*$'
    let regex_b = '@\I\i*\%(-\i*\)*'
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

    let diff = 0

    if preline =~# '\%(([%?]\=\|\[[*?]\=\|[{[:]\)$'
        let diff += 1
    endif

    if line =~# '^\%(%\=)\|[}\]]\|\<case\>\)' && !(preline =~# '^\<case\>' && preline !~# ':$')
        let diff -= 1
    endif

    let pat = '^\v%(\.\.@!|-\>)'

    if preline =~# pat && preline !~# '[:({\[]$'
        let diff -= 1
    endif

    if line =~# pat
        let diff += 1
    endif

    return indent(pnum) + diff * &shiftwidth
endfunction

setlocal indentexpr=<SID>getMdtlblIndent()
setlocal indentkeys+=0case
setlocal indentkeys+==case
setlocal indentkeys+==}
setlocal indentkeys+==]
setlocal indentkeys+==)
setlocal indentkeys+==:
setlocal indentkeys+==.
setlocal indentkeys+=0->

" END And Color Links {{{1
hi def link mdtlblKeyword		Keyword
hi def link mdtlblStar			Keyword
hi def link mdtlblOpFunKeyword		Operator
hi def link mdtlblCmpTreeOper		Operator
hi def link mdtlblCmpOper		NONE
hi def link mdtlblOpExprCtrl		Operator
hi def link mdtlblComment		Comment
hi def link mdtlblLongComment		Comment
hi def link mdtlblCommentMeta		Todo
hi def link mdtlblStringFailedEscape	Error
hi def link mdtlblStringColor		Include
hi def link mdtlblSpecialChar		SpecialChar
hi def link mdtlblString		String
hi def link mdtlblOIdent		Identifier
hi def link mdtlblOtherVar		Identifier
hi def link mdtlblNumber		Number
hi def link mdtlblBoolean		Boolean
hi def link mdtlblNull			Boolean
hi def link mdtlblResultHandle		Identifier
hi def link mdtlblDefineResultHandle	Identifier
hi def link mdtlblIdentLabel		Label
hi def link mdtlblIdentLabelRest	mdtlblIdentLabel
hi def link mdtlblArgsBracket		Macro
hi def link mdtlblQuickDExpTakeIdent	Macro
hi def link mdtlblArgsExpand		Structure
" }}}1
" vim:nowrap ts=8 sts=8 noet
