#!/usr/bin/env -S /bin/jq --indent 4 -nRf
"^snippet\\s+(?<prefix>\\S+)\\s+\"(?<description>[^\"]*)\"" as $title
| reduce inputs as $line ([]; if $line | test($title) then
  .+[$line | capture($title) | {key: .description, value: .}]
elif $line | test("^endsnippet|^\\s*(# |$)") | not then
  last.value.body += [$line]
end)
# 对于重复的注解, 需要报错
| (reduce .[] as $i ({}; .[$i.value.description] |= (if . then
  error("Duplicate description \($i.value)")
end + 1))) as $_
| from_entries
