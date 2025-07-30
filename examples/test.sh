#!/usr/bin/bash
set -o nounset
set -o errtrace
#set -o pipefail
function CATCH_ERROR {
    local __LEC=$? __i
    echo "Traceback (most recent call last):" >&2
    for ((__i = ${#FUNCNAME[@]} - 1; __i >= 0; --__i)); do
        printf '  File %q line %s in %q\n' >&2 \
            "${BASH_SOURCE[$__i]}" \
            "${BASH_LINENO[$__i]}" \
            "${FUNCNAME[$__i]}"
    done
    echo "Error: [ExitCode: ${__LEC}]" >&2
    exit "${__LEC}"
}
trap CATCH_ERROR ERR

hash mktemp mindustry_logic_bang_lang gawk find xargs diff cat sort

OPTIND=1
while getopts h opt; do case "$opt" in
    h)
        printf 'Usage: %q [Options]\n' "${0##*/}"
        echo '测试示例中格式为 `#* [opts] >>>'\'' 的文档注释所包含的内容'
        echo '输入由2空行分隔'
        echo
        printf '%s\n' \
            'Options:' \
            '    -h                 show help' \
            && exit
        ;;
    :|\?)
        ((--OPTIND <= 0)) && OPTIND=1
        printf '%q: parse args failed, near by %q\n' "$0" "${!OPTIND}" >&2
        exit 2
esac done
set -- "${@:OPTIND}"
if [ $# -ne 0 ]; then
    printf '%q: unexpected arg %q\n' "$0" "$1" >&2
    exit 2
fi

cd -- "$(command dirname -- "$0")" || exit

coproc ext {
    find . \( -name '*.mdtlbl' -o -name '*.logic' \) -print0 |
        sort -z |
        xargs -0 gawk '
    BEGIN {
        ORS="\0"
    }

    /^ *#\*+$/{indoc=1}
    {indoc_end=/^ *\*#$/}
    indoc{$0 = gensub(/^( *)[^ ].*/, "\\1# ...", "g")}
    indoc_end{indoc=0}

    /^*#/&&file&&lines&&outs{
        in_out=0
        print fnr":"mode":"file
        print lines
        print outs
        lines=""
        outs=""
    }
    in_out{outs=outs$0"\n"}
    /^#\* *[^ ]* >>>/{
        file=FILENAME
        fnr=FNR
        mode=gensub(/^#\* *([^ ]*) >>>.*/, "\\1", "g")
        if (!mode) mode="c"
        in_out=1
        outs=""
    }
    FILENAME!=prev_filename{lines=""; outs=""; prev_filename=FILENAME}
    in_out{next}

    {ws=/^ *$/||/^ *\*#/}
    ws{++wsc}
    !ws{wsc=0}
    {in_input=wsc<=1}
    !ws{lines=lines$0"\n"}
    !in_input{lines=""}
    '
}

tmp=$(mktemp)
trap "rm -v ${tmp@Q}" exit
declare -i count=0

shopt -s extglob

while
    IFS=: read -rd '' fnr mode file &&
    read -rd '' lines &&
    IFS='' read -rd '' outs
do
    count+=1
    outs=${outs%%*($'\n')}

    printf 'test %3s >>> %4d from %q\n' "$mode" "$fnr" "${file#./}"
    mindustry_logic_bang_lang "$mode" <<< "$lines" > "$tmp" || {
        echo "Run error, exit code: $?"
        echo $'\e[1;32mlines:\e[m'
        cat -n <<< "$lines"
        echo $'\e[1;33mouts(expected):\e[m'
        cat -n <<< "$outs"
        exit 2
    }
    if ! diff --color=auto "$tmp" -<<< "$outs"
    then
        echo $'\e[1;32mlines:\e[m'
        cat -n <<< "$lines"
        echo $'\e[1;31mcurs:\e[m'
        cat -n "$tmp"
        echo $'\e[1;33mouts(expected):\e[m'
        cat -n <<< "$outs"
        exit 1
    fi
done <&"${ext[0]}"

echo "$count tests finished"
