#!/usr/bin/bash
set -o nounset
set -e
set -x

hash tsc npm vsce ls

cd "$(command dirname -- "$0")"

../make-snippets.jq ../../vim/UltiSnips/mdtlbl.snippets > ./snippets/snippets.json
npm ci
tsc
vsce package

ls -lh ./*.vsix
mv ./*.vsix ./release/
