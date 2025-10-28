if exists('b:did_ftplugin') | finish | endif
let b:did_ftplugin = 1

aug mdtlblFtPlugin
    if has('linux')
        au BufWrite <buffer> call s:mdtlblAutoClipboard()
    endif
aug end

let g:mdtlbl_clipboard_command = get(g:, 'mdtlbl_clipboard_command', '| xclip -se c')

function s:mdtlblAutoClipboard() abort
    let finish = 0
    try
        let tail = g:mdtlbl_clipboard_command
        exe 'w !mindustry_logic_bang_lang cl ' . tail
        let finish = 1
    finally
        if finish | return | endif
        echomsg 'Auto compile to clipboard failed, please read syntax/vim/README.md'
    endtry
endfunction

function s:mdtlblQuickSingleQuote() abort
    let col = col('.')
    let line = getline('.')
    let ended_line = col > 1 ? line[:col-2] : ''

    if ended_line =~# '\>$'
        return "! ;\<left>"
    endif
    return "''\<left>"
endfunction
inoremap <buffer><expr> ' <sid>mdtlblQuickSingleQuote()

setlocal foldmethod=expr foldexpr=indent(prevnonblank(v:lnum))/&sw " fixed indent fold"
setlocal comments=s:#*,mb:*,ex:*#,:#
setlocal commentstring=#%s
setlocal formatoptions+=rq

let b:undo_ftplugin = 'setlocal foldmethod< foldexpr< comments< commentstring< formatoptions<'
            \ . ' | aug mdtlblFtPlugin | au! | aug end'
            \ . ' | iunmap <expr><buffer> '''
