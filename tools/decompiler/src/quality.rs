use crate::Reduce;

mod hard {
    pub const LABEL: f32 = 2.0;
    pub const JUMP: f32 = 4.0;
    pub const BREAK: f32 = 3.0;
    pub const DEPS_POW: f32 = 1.3;
    pub const PEFER_FORCE: f32 = 0.10;
    //pub const PAIR: f32 = JUMP+LABEL;
}

pub trait Loss {
    fn loss(&self) -> f32;
    fn loss_pefer(&self) -> f32 {
        self.loss()
    }
}

impl Loss for Reduce<'_> {
    fn loss(&self) -> f32 {
        0.5 + match self {
            Reduce::Pure(items) => items.len() as f32,
            Reduce::Product(reduces) => reduces.loss(),
            Reduce::Label(_) => hard::LABEL,
            Reduce::Jump(_) => hard::JUMP,
            Reduce::Break(_) => hard::BREAK,
            Reduce::Skip(_, sub) => sub.loss_pefer(),
            Reduce::DoWhile(_, sub) => sub.loss_pefer(),
            Reduce::While(_, deps, reduces) => {
                deps.loss().powf(hard::DEPS_POW) * 0.7 + reduces.loss_pefer()
            },
            Reduce::IfElse(_, then, else_br) => {
                then.loss_pefer() + else_br.loss_pefer()
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
        self.iter().map(Loss::loss).sum::<f32>()
    }

    fn loss_pefer(&self) -> f32 {
        let g = rqrt(self.len() as f32) * hard::PEFER_FORCE;
        self.loss() * (1.0+g)
    }
}

fn rqrt(n: f32) -> f32 {
    if n >= 1.0 {
        return n.sqrt();
    }
    1.0 / (-n+2.0).sqrt()
}

#[cfg(test)]
mod tests;
