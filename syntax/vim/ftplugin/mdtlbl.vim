if exists('b:did_ftplugin') | finish | endif
let b:did_ftplugin = 1

aug mdtlblFtPlugin
    au BufWrite <buffer> w !mindustry_logic_bang_lang cl | xclip -se c
aug end

function s:mdtlblQuickSingleQuote() abort
    let col = col('.')
    let line = getline('.')
    let ended_line = col > 1 ? line[:col-2] : ''
    echom [line, ended_line]

    if ended_line =~# '\>$'
        return '! '
    endif
    return "''\<left>"
endfunction
inoremap <buffer><expr> ' <sid>mdtlblQuickSingleQuote()
