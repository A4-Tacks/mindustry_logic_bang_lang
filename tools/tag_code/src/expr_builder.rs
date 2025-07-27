use std::{fmt::{self, Display, Write}, rc::Rc, str::FromStr};

use either::{for_both, Either::{self, Left, Right}};
use linked_hash_map::LinkedHashMap;
use var_utils::Var;

use crate::logic_parser::ParseLine;

pub mod ops;

#[cfg(test)]
mod tests;

use ops::*;

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum Expr {
    Single(Sin, Rc<Expr>),
    Binary(Bin, Rc<Expr>, Rc<Expr>),
    Select(Cmp, Rc<Expr>, Rc<Expr>, Rc<Expr>, Rc<Expr>),
    Var(Var),
}

impl From<Var> for Expr {
    fn from(v: Var) -> Self {
        Self::Var(v)
    }
}

impl Expr {
    pub fn prec(&self) -> u32 {
        match self {
            Expr::Single(op, ..) => op.prec(),
            Expr::Binary(op, ..) => op.prec(),
            Expr::Select(..) => 0,
            Expr::Var(..) => u32::MAX,
        }
    }

    pub fn comb(&self) -> Option<Comb> {
        match self {
            Expr::Single(op, ..) => Some(op.comb()),
            Expr::Binary(op, ..) => Some(op.comb()),
            Expr::Select(..) => Some(Comb::Right),
            Expr::Var(..) => None,
        }
    }

    pub fn contains(&self, ref_: &Var) -> bool {
        macro_rules! x { ($($v:ident),+) => { $($v.contains(ref_))||+ } }
        match self {
            Expr::Single(_, a) => x!(a),
            Expr::Binary(_, a, b) => x!(a, b),
            Expr::Select(_, a, b, c, d) => x!(a, b, c, d),
            Expr::Var(var) => var == ref_,
        }
    }
}

impl Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fn fsp(op: &impl Oper) -> &'static str {
            if op.is_func() {
                " "
            } else {
                ""
            }
        }
        fn paren<O: OpMeta>(
            op: Either<&O, &Expr>,
            ws: &str,
            comb: impl Into<Option<Comb>>,
            expr: &Expr,
            f: &mut fmt::Formatter,
        ) -> fmt::Result {
            let op_prec = for_both!(op, op => op.prec());
            if expr.prec() > op_prec
            || expr.prec() == op_prec
                && expr.comb().zip(comb.into()).is_none_or(|(a, b)| a == b)
            {
                write!(f, "{ws}{expr}")
            } else {
                write!(f, "({expr})")
            }
        }
        match self {
            Expr::Single(op, expr) => {
                write!(f, "{}", op.expr_display())?;
                paren(Left(op), fsp(op), Comb::Right, expr, f)
            },
            Expr::Binary(op, a, b) if op.is_func() => {
                write!(f, "{}({a}, {b})", op.name())
            },
            Expr::Binary(op, a, b) => {
                paren(Left(op), "", Comb::Left, a, f)?;
                write!(f, " {} ", op.expr_display())?;
                paren(Left(op), "", Comb::Right, b, f)?;
                Ok(())
            },
            Expr::Select(cmp, a, b, r1, r2) => {
                paren(Left(cmp), "", Comb::Left, a, f)?;
                write!(f, " {} ", cmp.expr_display())?;
                paren(Left(cmp), "", Comb::Right, b, f)?;
                write!(f, " ? ")?;
                paren(Left(cmp), "", None, r1, f)?;
                write!(f, " : ")?;
                paren::<Bin>(Right(self), "", Comb::Right, r2, f)?;
                Ok(())
            },
            Expr::Var(var) => var.fmt(f),
        }
    }
}

pub fn build<'a>(lines: impl IntoIterator<Item = &'a ParseLine<'a>>) -> Vec<String> {
    let mut ops: LinkedHashMap<Var, (i32, i32, Rc<Expr>)> = LinkedHashMap::new();
    let mut out = vec![];

    macro_rules! println {
        ($($y:tt)*) => {{
            out.push(format!($($y)*));
        }};
    }
    macro_rules! weak {
        ($i:expr) => {
            for weak_ref in $i {
                if let Some((_, wc, _)) = ops.get_mut(weak_ref) {
                    *wc += 1;
                }
            }
        };
    }

    for (i, line) in lines.into_iter().enumerate() {
        let ParseLine::Args(args) = line else {
            match line {
                ParseLine::Label(_) | ParseLine::Args(_) => (),
                ParseLine::Jump(_, args) => weak!(&args[1..]),
            }
            println!("#{i}.unsupported: {line}");
            continue;
        };
        macro_rules! ref_it {
            ($s:expr) => {{
                if let Some((c, wc, _)) = ops.get_mut($s) {
                    *c += 1;
                    *wc += 1;
                }
                ops.get($s)
                    .map(|(_, _, exp)| exp.clone())
                    .unwrap_or_else(|| Rc::new($s.clone().into()))
            }};
        }
        macro_rules! add {
            ($dst:expr, $v:expr) => {{
                let v: Rc<Expr> = $v;
                if ops.contains_key($dst) && v.contains($dst) {
                    ops.insert($dst.clone(), (ops[$dst].0, ops[$dst].0, v));
                } else {
                    ops.insert($dst.clone(), (0, 0, v));
                }
            }};
        }
        match args.first() {
            "set" if args.len() >= 3 => {
                let dst = &args[1];
                let v = ref_it!(&args[2]);

                println!("{dst} = {v};");
                add!(dst, v);
            },
            "op" if args.len() >= 4 && Sin::from_str(&args[1]).is_ok() => {
                let dst = &args[2];
                let oper = args[1].parse().unwrap();
                let a = ref_it!(&args[3]);
                let expr = Expr::Single(oper, a);

                println!("{dst} = {expr};");
                add!(dst, expr.into());
            },
            "op" if args.len() >= 5 && Bin::from_str(&args[1]).is_ok() => {
                let dst = &args[2];
                let oper = args[1].parse().unwrap();
                let a = ref_it!(&args[3]);
                let b = ref_it!(&args[4]);
                let expr = Expr::Binary(oper, a, b);

                println!("{dst} = {expr};");
                add!(dst, expr.into());
            },
            "read" if args.len() >= 4 => {
                let dst = &args[1];
                let a = ref_it!(&args[2]);
                let b = ref_it!(&args[3]);
                let expr = Expr::Binary(Bin::Read, a, b);

                println!("{dst} = {expr};");
                add!(dst, expr.into());
            },
            "sensor" if args.len() >= 4 => {
                let dst = &args[1];
                let a = ref_it!(&args[2]);
                let b = ref_it!(&args[3]);
                let expr = Expr::Binary(Bin::Sense, a, b);

                println!("{dst} = {expr};");
                add!(dst, expr.into());
            },
            "select" if args.len() >= 7 && Cmp::from_str(&args[2]).is_ok() => {
                let dst = &args[1];
                let cmp = args[2].parse().unwrap();
                let x = ref_it!(&args[3]);
                let y = ref_it!(&args[4]);
                let a = ref_it!(&args[5]);
                let b = ref_it!(&args[6]);
                let expr = Expr::Select(cmp, x, y, a, b);

                println!("{dst} = {expr};");
                add!(dst, expr.into());
            },
            _ => {
                weak!(&args[1..]);
                println!("#{i}.unsupported: {line}");
                continue
            },
        }

        write!(out.last_mut().unwrap(), " # {line}").unwrap();
    }

    println!("");
    println!("-- no reference variables --");

    let finish_start = out.len();

    for (name, &(c, _wc, ref expr)) in &ops {
        if c == 0 {
            println!("{name} = {expr};");
        }
    }

    let finish_rng = finish_start..out.len();

    println!("");
    println!("-- strict no reference variables --");

    let finish_start = out.len();

    for (name, &(c, wc, ref expr)) in &ops {
        if c == 0 && wc == 0 {
            println!("{name} = {expr};");
        }
    }

    if out.len() - finish_start == finish_rng.len() {
        out.truncate(finish_rng.end);
    }

    out
}
