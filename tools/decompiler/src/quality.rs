use crate::Reduce;

pub trait Loss {
    fn loss(&self) -> f32;
}

impl Loss for Reduce<'_> {
    fn loss(&self) -> f32 {
        match self {
            Reduce::Pure(items) => (items.len() as f32 * 1.0).sqrt(),
            Reduce::Product(reduces) => reduces.loss(),
            Reduce::Label(_) => 2.0,
            Reduce::Jump(_) => 4.0,
            Reduce::Break(_) => 3.5,
            Reduce::Skip(_, sub) => sub.loss(),
            Reduce::DoWhile(_, sub) => sub.loss()*0.95,
            Reduce::While(_, deps, reduces) => {
                deps.loss() * 0.7 + reduces.loss() * 0.9
            },
            Reduce::IfElse(_, then, else_br) => {
                then.loss() * 0.9 + else_br.loss() * 0.9
            },
            Reduce::GSwitch(_, sub) => {
                sub.iter()
                    .map(|x| x.1.loss())
                    .max_by(|a, b| a.total_cmp(b))
                    .unwrap_or_default()
                    + 3.0
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
    iter.map(Loss::loss).sum::<f32>() * (len as f32).ln().max(1.0)
}
