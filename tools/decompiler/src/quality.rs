use crate::Reduce;

mod hard {
    pub const LABEL: f32 = 2.0;
    pub const JUMP: f32 = 4.0;
    pub const BREAK: f32 = 3.0;
    pub const PAIR: f32 = JUMP+LABEL;
}

pub trait Loss {
    fn loss(&self) -> f32;
}

impl Loss for Reduce<'_> {
    fn loss(&self) -> f32 {
        match self {
            Reduce::Pure(items) => (items.len() as f32 + 1.0).sqrt(),
            Reduce::Product(reduces) => reduces.loss(),
            Reduce::Label(_) => hard::LABEL,
            Reduce::Jump(_) => hard::JUMP,
            Reduce::Break(_) => hard::BREAK,
            Reduce::Skip(_, sub) => (sub.loss()+hard::PAIR)*0.98,
            Reduce::DoWhile(_, sub) => (sub.loss()+hard::PAIR)*0.95,
            Reduce::While(_, deps, reduces) => {
                deps.loss().powf(1.3) * 0.7 + (reduces.loss()+hard::PAIR*2.0)*0.98*0.95
            },
            Reduce::IfElse(_, then, else_br) => {
                (then.loss() + else_br.loss()+hard::PAIR*2.0) * 0.9
            },
            Reduce::GSwitch(_, sub) => {
                sub.iter()
                    .map(|x| x.1.loss())
                    .max_by(|a, b| a.total_cmp(b))
                    .unwrap_or_default()
                    * 0.6
            },
        }
    }
}

impl Loss for [Reduce<'_>] {
    fn loss(&self) -> f32 {
        reduces_loss(self)
    }
}

fn reduces_loss<'a, I>(reduces: I) -> f32
where I: IntoIterator<Item = &'a Reduce<'a>>,
      I::IntoIter: ExactSizeIterator,
{
    let iter = reduces.into_iter();
    let len = iter.len();
    let g = rqrt(len as f32 - 3.0) / 1.0;
    iter.map(Loss::loss).sum::<f32>()// * (1.0+g)
}

fn rqrt(n: f32) -> f32 {
    if n >= 1.0 {
        return n.sqrt();
    }
    1.0 / (-n+2.0).sqrt()
}

#[cfg(test)]
mod tests;
