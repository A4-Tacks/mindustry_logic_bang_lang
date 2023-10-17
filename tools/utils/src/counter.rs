/// 一个负责获取结果并自增的结构
/// # Examples
/// ```
/// # use utils::counter::Counter;
/// let mut counter: Counter<_> = Counter::new(|n| {
///     let old = *n;
///     *n += 1;
///     format!("__{old}")
/// });
/// assert_eq!(counter.get(), "__0");
/// assert_eq!(counter.get(), "__1");
/// assert_eq!(counter.get(), "__2");
/// assert_eq!(counter.get(), "__3");
/// ```
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct Counter<F, T = usize>
{
    counter: T,
    getter: F,
}

impl<F, R, T> Counter<F, T>
where F: FnMut(&mut T) -> R
{
    pub fn with_counter(f: F, counter: T) -> Self {
        Self {
            counter,
            getter: f
        }
    }

    pub fn get(&mut self) -> R {
        (self.getter)(&mut self.counter)
    }
}

impl<F, R, T> Counter<F, T>
where F: FnMut(&mut T) -> R,
      T: Default
{
    pub fn new(f: F) -> Self {
        Self::with_counter(f, Default::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        let mut counter: Counter<_> = Counter::new(|n| {
            let old = *n;
            *n += 1;
            format!("__{old}")
        });
        assert_eq!(counter.get(), "__0");
        assert_eq!(counter.get(), "__1");
        assert_eq!(counter.get(), "__2");
        assert_eq!(counter.get(), "__3");
    }
}
