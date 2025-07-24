" Vim syntax file
" Language:		mindustry logic (logic)
" Maintainer:		A4-Tacks <wdsjxhno1001@163.com>
" Last Change:		2024-09-18
" URL:			https://github.com/A4-Tacks/mindustry_logic_bang_lang
scriptencoding utf-8

" 已加载高亮时就退出
if exists('b:current_syntax')
    finish
endif

" Register {{{1

" 语法文件注册, 请复制到如vimrc并取消注释
" 注册语法文件后将此文件复制到语法目录, 如 ~/.vim/syntax/
"augroup filetypedetect
"    autocmd BufNewFile,BufRead *.logic setfiletype logic
"augroup END


" debug clear
"syn clear

" Define {{{1

" 大小写敏感
syn case match

setlocal iskeyword=!,36-57,60-126
setlocal foldmethod=indent

syn match	logicEOL		/^\|;/				transparent nextgroup=logicHead,logicJump,logicOp,logicPrint		skipwhite
syn match	logicComment		/#.*/				contains=logicCommentMeta
syn match	logicHead		/[^ \t\r;:#]\+/			contained
syn match	logicOp			/op[ \t]/			contained nextgroup=logicOpOper,logicJumpOper		skipwhite
syn match	logicJump		/jump[ \t]/			contained nextgroup=logicJumpLabel			skipwhite
syn match	logicJumpLabel		/\s*:\=[^ \t\r;:#]\+[ \t]/	contained nextgroup=logicJumpOper,logicJumpAlways	skipwhite
syn match	logicHeadLabel		/\%(^\|;\)\@1<=\s*:\=[^ \t\r;:#]\+:\ze\s*[;#\n]/
syn match	logicPrint		/print[ \t]/			contained nextgroup=logicPrintBody,logicPrintBodyVar	skipwhite
syn match	logicPrintBody		/"[^"\n]*"/			contained nextgroup=logicPrintRest			skipwhite contains=logicStringEscape,logicStringColor
syn match	logicPrintBodyVar	/[^ \t\r;:#"]\+/		contained nextgroup=logicPrintRest			skipwhite
syn match	logicPrintRest		/;[ \t]*print[ \t]/		contained nextgroup=logicPrintBody,logicPrintBodyVar	skipwhite conceal
syn region	logicString		start=/"/ end=/"/		contains=logicStringEscape,logicStringColor
syn match	logicStringColor					contained /\[\v%(#\x{6,8}|%(c%(lear|yan|oral)|b%(l%(ack|ue)|r%(own|ick))|white|li%(ghtgray|me)|g%(r%(ay|een)|old%(enrod)?)|darkgray|navy|r%(oyal|ed)|s%(late|ky|carlet|almon)|t%(eal|an)|acid|forest|o%(live|range)|yellow|p%(ink|urple)|ma%(genta|roon)|violet))=\]/
syn match	logicStringEscape	/\\n\|\[\[/			contained
syn match	logicNumber		/\v[+-]?%(<0x[+-]?[0-9a-fA-F]+|0b[+-]?[01]+|<\d+%(\.\d+|e[+-]?\d+)?)>/
syn match	logicNumber		/\v<%(true|false)>/
syn match	logicNull		/\v<null>/
syn match	logicMeta		/\v<\@[^ \t\r;:#]+>/
syn keyword	logicCommentMeta	TODO Todo HINT Hint		contained

" Jump Compare Operator {{{
syn match	logicJumpOperEqPostfix	/./							contained conceal cchar==
syn match	logicJumpOperEqPostfix2	/./							contained conceal cchar== nextgroup=logicJumpOperEqPostfix
syn match	logicJumpOper		/\<lessThan\>/						contained conceal cchar=<
syn match	logicJumpOper		/\<greaterThan\>/					contained conceal cchar=>
syn match	logicJumpOper		/\<lessThanE\zeq\>/					contained conceal cchar=< nextgroup=logicJumpOperEqPostfix
syn match	logicJumpOper		/\<greaterThanE\zeq\>/					contained conceal cchar=> nextgroup=logicJumpOperEqPostfix
syn match	logicJumpOper		/\<equa\zel\>/						contained conceal cchar== nextgroup=logicJumpOperEqPostfix
syn match	logicJumpOper		/\<notEqua\zel\>/					contained conceal cchar=! nextgroup=logicJumpOperEqPostfix
syn match	logicJumpOper		/\<strictEqu\zeal\>/					contained conceal cchar== nextgroup=logicJumpOperEqPostfix2
syn match	logicJumpOper		/\<notEqual\s\+\(:*\<[^ \t\r;:#]\+\>:*\)\s\+\1\>:\@!/	contained conceal cchar=!
syn match	logicJumpAlways		/\<always\>/						contained conceal cchar=_
" }}}
" Op Operator {{{
syn match	logicOpOperStar	/./		conceal cchar=*	contained
syn match	logicOpOperLt	/./		conceal cchar=<	contained
syn match	logicOpOperGt	/./		conceal cchar=>	contained
syn match	logicOpOperGt2	/./		conceal cchar=>	contained nextgroup=logicOpOperGt
syn match	logicOpOperMod	/./		conceal cchar=%	contained
syn match	logicOpOperSl	/./		conceal cchar=/	contained
syn match	logicOpOperAnd	/./		conceal cchar=&	contained
syn keyword	logicOpOper	add		conceal cchar=+	contained
syn keyword	logicOpOper	sub		conceal cchar=-	contained
syn keyword	logicOpOper	mul		conceal cchar=*	contained
syn keyword	logicOpOper	div		conceal cchar=/	contained
syn keyword	logicOpOper	mod		conceal cchar=%	contained
syn keyword	logicOpOper	or		conceal cchar=|	contained
syn keyword	logicOpOper	and		conceal cchar=&	contained
syn keyword	logicOpOper	xor		conceal cchar=^	contained
syn keyword	logicOpOper	not		conceal cchar=!	contained
syn match	logicOpOper	/\<emo\zed\>/	conceal cchar=%	contained nextgroup=logicOpOperMod
syn match	logicOpOper	/\<idi\zev\>/	conceal cchar=/	contained nextgroup=logicOpOperSl
syn match	logicOpOper	/\<sh\zel\>/	conceal cchar=<	contained nextgroup=logicOpOperLt
syn match	logicOpOper	/\<sh\zer\>/	conceal cchar=>	contained nextgroup=logicOpOperGt
syn match	logicOpOper	/\<us\zehr\>/	conceal cchar=>	contained nextgroup=logicOpOperGt2
syn match	logicOpOper	/\<po\zew\>/	conceal cchar=*	contained nextgroup=logicOpOperStar
syn match	logicOpOper	/\<lan\zed\>/	conceal cchar=&	contained nextgroup=logicOpOperAnd
syn keyword	logicOpOper	contained
				\ max min angle angleDiff len noise
				\ abs sign log logn log10 floor ceil round sqrt rand
				\ sin cos tan asin acos atan
" }}}

" Link {{{1

hi def link logicComment		Comment
hi def link logicHead			Structure
hi def link logicPrint			logicHead
hi def link logicPrintRest		logicPrint
hi def link logicPrintBody		logicString
hi def link logicOp			logicHead
hi def link logicJump			Label
hi def link logicJumpLabel		Tag
hi def link logicHeadLabel		Tag
hi def link logicString			String
hi def link logicStringEscape		Special
hi def link logicStringColor		Include
hi def link logicNumber			Number
hi def link logicNull			Number
hi def link logicMeta			Identifier
hi def link logicCommentMeta		Todo
hi def link logicJumpOper		Operator
hi def link logicJumpAlways		logicJumpOper
hi def link logicJumpOperEqPostfix	logicJumpOper
hi def link logicJumpOperEqPostfix2	logicJumpOper
hi def link logicOpOper			Operator
hi def link logicOpOperStar		logicOpOper
hi def link logicOpOperLt		logicOpOper
hi def link logicOpOperGt		logicOpOper
hi def link logicOpOperGt2		logicOpOper
hi def link logicOpOperSl		logicOpOper
hi def link logicOpOperAnd		logicOpOper
hi def link logicOpOperMod		logicOpOper

" End {{{1
" vim:nowrap ts=8 sts=8 noet
" }}}1
