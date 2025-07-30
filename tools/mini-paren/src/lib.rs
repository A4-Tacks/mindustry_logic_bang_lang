//! 最小化嵌值语言, 适用于逻辑语言, 使用圆括号和美元符号进行嵌值

use std::borrow::Cow;

pub struct Line<'a>(Vec<Value<'a>>);

impl<'a> Line<'a> {
    fn get_ret_var(&self) -> Option<&'a str> {
        self.0.iter().find_map(Value::as_ret_var)
    }
}

pub enum Value<'a> {
    Lines(Vec<Line<'a>>),
    Var(&'a str),
}

impl<'a> Value<'a> {
    fn as_ret_var(&self) -> Option<&'a str> {
        self.as_var()
            .and_then(|v| v.strip_prefix('$'))
    }

    fn as_real_var(&self) -> Option<&'a str> {
        self.as_ret_var().or_else(|| self.as_var())
    }

    fn as_var(&self) -> Option<&'a str> {
        if let Self::Var(v) = self {
            Some(v)
        } else {
            None
        }
    }

    fn find_ret_var(&self) -> Option<&'a str> {
        match self {
            Value::Var(_) => self.as_ret_var(),
            Value::Lines(lines) => {
                lines.iter()
                    .filter_map(Line::get_ret_var)
                    .max_by_key(|v| v.len())
            },
        }
    }
}

#[derive(Debug, Default)]
pub struct State<'a> {
    count: usize,
    rets: Vec<Cow<'a, str>>,
    pub out: String,
}

impl<'a> State<'a> {
    fn tmp_var(&mut self) -> String {
        let n = self.count;
        self.count += 1;
        format!("__{n}")
    }

    pub fn process_value(&mut self, value: &Value<'a>) -> Cow<'a, str> {
        match value {
            Value::Lines(lines) => {
                let mut ret: Cow<'a, str> = value.find_ret_var().unwrap_or("_").into();
                if ret.is_empty() {
                    ret = self.tmp_var().into();
                }
                self.rets.push(ret);

                self.process_lines(lines);

                self.rets.pop().unwrap()
            },
            Value::Var(v) => {
                if let Some(ret) = self.rets.last() {
                    let real = value.as_real_var().unwrap();
                    if real.is_empty() {
                        ret.clone()
                    } else {
                        real.into()
                    }
                } else {
                    (*v).into()
                }
            },
        }
    }

    pub fn process_lines(&mut self, lines: &[Line<'a>]) {
        for line in lines {
            let args = line.0.iter()
                .map(|arg| self.process_value(arg))
                .collect::<Vec<_>>();
            self.out.push_str(&args[0]);
            for arg in &args[1..] {
                self.out.push(' ');
                self.out.push_str(arg);
            }
            self.out.push('\n');
        }
    }
}

peg::parser!(pub grammar parser() for str {
    rule _() = __?
    rule __() = quiet!{[' ' | '\t']+} / expected!("whitespace")
    rule ident() -> &'input str
        = quiet!{$((!__ !nl() [^'#' | '"' | ';' | '(' | ')'])+ / "()")}
        / expected!("ident")
    rule string() -> &'input str
        = quiet!{$("\"" (!nl() [^'"'])+ "\"")}
        / expected!("string")
    rule comment() = "#" [^'\r' | '\n']*
    rule nl() = _ (comment()? ("\r"? "\n" _ nl()? / ![_]) / &")")
    rule end() = _ ";" _ nl()? / nl()
    rule line() -> Line<'input> = l:value() ++ _ { Line(l) }
    rule value() -> Value<'input>
        = i:ident() { Value::Var(i) }
        / s:string() { Value::Var(s) }
        / "(" l:lines() ")" { Value::Lines(l) }
    pub rule lines() -> Vec<Line<'input>>
        = _ nl()? l:line()++end() _ end()? {l}
});
