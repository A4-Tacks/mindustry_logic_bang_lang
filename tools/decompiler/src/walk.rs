use crate::{Jump, Label, Reduce};

impl Reduce<'_> {
    pub fn walk_reduces(&self, f: &mut impl FnMut(&Reduce<'_>)) {
        f(self);
        match self {
            Reduce::Pure(..) | Reduce::Label(..) | Reduce::Jump(..) |
            Reduce::Break(..) => (),
            Reduce::Product(reduces) => {
                for sub_reduce in reduces {
                    sub_reduce.walk_reduces(f);
                }
            },
            Reduce::Skip(_, reduces) |
            Reduce::DoWhile(_, reduces) => {
                for sub_reduce in reduces.as_ref() {
                    sub_reduce.walk_reduces(f);
                }
            },
            Reduce::While(_, deps, reduces) |
            Reduce::IfElse(_, deps, reduces) => {
                for dep in deps.as_ref() {
                    dep.walk_reduces(f);
                }
                for sub_reduce in reduces.as_ref() {
                    sub_reduce.walk_reduces(f);
                }
            },
        }
    }

    pub fn walk_reduce_slices(&self, f: &mut impl FnMut(&[Reduce<'_>])) {
        self.walk_reduces(&mut |sub_reduce| match sub_reduce {
            Reduce::Pure(..) | Reduce::Label(..) | Reduce::Jump(..) |
            Reduce::Break(..) => (),
            Reduce::Product(reduces) => f(&reduces),
            Reduce::Skip(_, reduces) => f(&reduces),
            Reduce::DoWhile(_, reduces) => f(&reduces),
            Reduce::While(_, deps, reduces) |
            Reduce::IfElse(_, deps, reduces) => {
                f(&deps);
                f(&reduces)
            },
        });
    }

    pub fn walk_label_defs(&self, f: &mut impl FnMut(&Label)) {
        self.walk_reduces(&mut |r| match r {
            Reduce::Label(label) => f(label),
            _ => (),
        });
    }

    pub fn walk_label_usages(&self, f: &mut impl FnMut(&Label)) {
        self.walk_reduces(&mut |r| match r {
            Reduce::Jump(Jump(label, _)) => f(label),
            _ => (),
        });
    }
}

pub fn label_defs(cleaned: &Reduce<'_>) -> usize {
    let mut label_def_count = 0;
    cleaned.walk_label_defs(&mut |_| label_def_count += 1);
    label_def_count
}

pub fn label_usages(cleaned: &Reduce<'_>) -> usize {
    let mut label_usage_count = 0;
    cleaned.walk_label_usages(&mut |_| label_usage_count += 1);
    label_usage_count
}
