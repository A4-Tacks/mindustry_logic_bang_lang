use std::{
    borrow::{Borrow, Cow},
    ops::Deref,
};

#[derive(Debug, Copy)]
pub enum TCow<'a, B: 'a + ?Sized, O> {
    Borrowed(&'a B),
    Owned(O),
}
impl<'a, B> TCow<'a, B, B::Owned>
where B: 'a + ?Sized + ToOwned,
{
    pub fn from_cow(cow: Cow<'a, B>) -> Self {
        match cow {
            Cow::Borrowed(b) => Self::Borrowed(b),
            Cow::Owned(owned) => Self::Owned(owned),
        }
    }

    pub fn into_owned(self) -> B::Owned {
        match self {
            TCow::Borrowed(b) => b.to_owned(),
            TCow::Owned(owned) => owned,
        }
    }

    pub fn make_mut(&mut self) -> &mut B::Owned {
        match self {
            TCow::Borrowed(b) => {
                *self = Self::Owned(b.to_owned());
                self.make_mut()
            },
            TCow::Owned(owned) => owned,
        }
    }
}
impl<'a, B: 'a + ?Sized, O> From<O> for TCow<'a, B, O> {
    fn from(value: O) -> Self {
        Self::Owned(value)
    }
}
impl<'a, B: 'a + ?Sized, O: Clone> Clone for TCow<'a, B, O> {
    fn clone(&self) -> Self {
        match self {
            Self::Borrowed(b) => Self::Borrowed(b),
            Self::Owned(owned) => Self::Owned(owned.clone()),
        }
    }
}
impl<'a, B, O> std::hash::Hash for TCow<'a, B, O>
where B: 'a + ?Sized + std::hash::Hash,
      O: Deref<Target = B> + std::hash::Hash,
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        (**self).hash(state)
    }
}
impl<'a, B, O> Eq for TCow<'a, B, O>
where B: 'a + Eq + ?Sized,
      O: Deref<Target = B>,
{
}
impl<'a, B, O> PartialEq for TCow<'a, B, O>
where B: 'a + PartialEq + ?Sized,
      O: Deref<Target = B>,
{
    fn eq(&self, other: &Self) -> bool {
        **self == **other
    }
}
impl<'a, B, O> Deref for TCow<'a, B, O>
where B: 'a + ?Sized,
      O: Deref<Target = B>,
{
    type Target = B;

    fn deref(&self) -> &Self::Target {
        match self {
            TCow::Borrowed(b) => *b,
            TCow::Owned(owned) => &**owned,
        }
    }
}
impl<'a, B, O> Borrow<B> for TCow<'a, B, O>
where B: 'a + ?Sized,
      O: Borrow<B>,
{
    fn borrow(&self) -> &B {
        match self {
            TCow::Borrowed(b) => *b,
            TCow::Owned(owned) => owned.borrow(),
        }
    }
}
impl<'a, B, O> AsRef<B> for TCow<'a, B, O>
where B: 'a + ?Sized,
      O: AsRef<B>,
{
    fn as_ref(&self) -> &B {
        match self {
            TCow::Borrowed(b) => *b,
            TCow::Owned(owned) => owned.as_ref(),
        }
    }
}
impl<'a, T, B, O> FromIterator<T> for TCow<'a, B, O>
where B: 'a + ?Sized,
      O: FromIterator<T>,
{
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self::Owned(iter.into_iter().collect())
    }
}
impl<'a, T, B> Extend<T> for TCow<'a, B, B::Owned>
where B: 'a + ?Sized + ToOwned,
      B::Owned: Extend<T>,
{
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        self.make_mut()
            .extend(iter)
    }
}
impl<'a, B, O> ToString for TCow<'a, B, O>
where B: 'a + ?Sized + ToString,
      O: Borrow<B>,
{
    fn to_string(&self) -> String {
        let borrowed: &B = &self.borrow();
        borrowed.to_string()
    }
}
