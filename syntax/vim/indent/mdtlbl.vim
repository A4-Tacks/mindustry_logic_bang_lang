if exists("b:did_indent")
  finish
endif
let b:did_indent = 1

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

    if preline =~# '\%(([%?*]\=\|[(:]=\|\[[*?]\=\|[{[:]\)$'
        let diff += 1
    endif

    if line =~# '^\%(%\=)\|[}\]]\|\<case\>\)' && !(preline =~# '^\<case\>' && preline !~# ':$')
        let diff -= 1
    endif

    let pat = '^\v%(\.\.@!|-\>|\=\=@!)'

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
setlocal indentkeys+=0=
setlocal indentkeys+=;=

let b:undo_indent = 'setlocal indentexpr< indentkeys<'
