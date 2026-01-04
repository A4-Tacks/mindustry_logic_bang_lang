use syntax::*;
use walk::Node;
use std::ops::ControlFlow;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CurLocation {
    LineFirst,
    Binder,
    BindName { first: bool },
    Other,
}

pub fn cur_location(top: &Expand) -> CurLocation {
    let mut location = CurLocation::Other;
    let pred = |s: &syntax::Var| s.ends_with(LSP_DEBUG);
    let _ = walk::nodes(top.iter(), |node| {
        match node {
            Node::Value(Value::Var(var)) |
            Node::Key(ConstKey::Var(var))
                if pred(var) => return ControlFlow::Break(()),
            Node::Value(Value::ValueBindRef(syntax::ValueBindRef { bind_target: syntax::ValueBindRefTarget::NameBind(name), .. })) |
            Node::Value(Value::ValueBind(syntax::ValueBind(_, name))) |
            Node::Key(ConstKey::ValueBind(syntax::ValueBind(_, name)))
                if pred(name) => return ControlFlow::Break(location = CurLocation::BindName { first: false }),
            Node::Line(LogicLine::Other(args)) => if let Some(first) = args.first() {
                match first {
                    Value::Var(var) if pred(var) => return ControlFlow::Break(location = CurLocation::LineFirst),
                    Value::ValueBindRef(ValueBindRef { bind_target: ValueBindRefTarget::NameBind(name), .. }) |
                    Value::ValueBind(ValueBind(_, name))
                        if pred(name) => return ControlFlow::Break(location = CurLocation::BindName { first: true }),
                    _ => ()
                }
            }
            Node::Value(Value::ValueBindRef(syntax::ValueBindRef { value, .. })) |
            Node::Value(Value::ValueBind(syntax::ValueBind(value, _))) |
            Node::Key(ConstKey::ValueBind(syntax::ValueBind(value, _)))
                if value.as_var().is_some_and(pred) => return ControlFlow::Break(location = CurLocation::Binder),
            _ => (),
        }

        ControlFlow::Continue(())
    });
    location
}

#[cfg(test)]
mod tests {
    use super::*;

    thread_local! {
        static PARSER: parser::TopLevelParser = parser::TopLevelParser::new();
    }

    #[track_caller]
    fn check_first(src: &str) {
        let loc = parse_and_location(src);
        assert_eq!(loc, CurLocation::LineFirst, "`{}`", source_cursor(src));
    }

    #[track_caller]
    fn check_at_bind_name(src: &str) {
        let loc = parse_and_location(src);
        assert!(matches!(loc, CurLocation::BindName { .. }),
                "actual: {loc:?}, `{}`", source_cursor(src));
    }

    #[track_caller]
    fn check_not_first(src: &str) {
        let loc = parse_and_location(src);
        assert_ne!(loc, CurLocation::LineFirst);
    }

    #[track_caller]
    fn parse_and_location(src: &str) -> CurLocation {
        let input = source_cursor(src);
        let top = PARSER.with(|parser| parser.parse(&mut Default::default(), &input)).unwrap();
        cur_location(&top)
    }

    fn source_cursor(src: &str) -> String {
        let replaced = src.replacen("$0", LSP_DEBUG, 1);
        assert_ne!(src, replaced);
        replaced
    }

    #[track_caller]
    fn check_parse_error(src: &str) {
        let input = source_cursor(src);
        let parsed
            = PARSER.with(|parser| parser.parse(&mut Default::default(), &input));
        assert!(parsed.is_err(), "{parsed:#?}")
    }

    #[test]
    fn test_cur_first() {
        check_first("$0;");
        check_first("$0 x;");
        check_first("$0 x @;");
        check_first("$0 @;");
        check_first("$0 @;");
        check_first("F$0 @;");
        check_first("F$0;");
    }

    #[test]
    fn test_cur_first_op_expr() {
        check_parse_error("$0 x = 2;");
        check_parse_error("$0 x y = 2;");
        check_parse_error("$0 x y = 2, 3;");
        check_parse_error("$0 x y = 2, 3;");
    }

    #[test]
    fn test_cur_not_first() {
        check_not_first("x $0;");
        check_not_first("x $0 y;");
        check_not_first("op $0 2 + 3;");
    }

    #[test]
    fn test_on_bind_name() {
        check_at_bind_name("x.$0;");
        check_at_bind_name("{x.$0;}");
        check_at_bind_name("m x.$0;");
        check_at_bind_name("break x.$0;");
        check_at_bind_name("break 2 && x.$0;");
    }
}
