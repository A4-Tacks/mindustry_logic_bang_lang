use std::{
    borrow::Borrow,
    fmt::{self, Display},
    hash::Hash,
    mem,
    ops::Deref,
    rc,
};

type Rc<T> = rc::Rc<T>;

#[derive(Default, Clone)]
pub struct Var {
    value: Rc<String>,
}
impl fmt::Debug for Var {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        std::fmt::Debug::fmt(&*self.value, f)
    }
}
impl Var {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn to_mut(&mut self) -> &mut String {
        Rc::make_mut(&mut self.value)
    }

    pub fn into_owned(&mut self) -> String {
        mem::take(Rc::make_mut(&mut self.value))
    }

    pub fn as_str(&self) -> &str {
        &**self
    }
}
impl FromIterator<char> for Var {
    fn from_iter<T: IntoIterator<Item = char>>(iter: T) -> Self {
        String::from_iter(iter).into()
    }
}
impl From<&'_ Var> for Var {
    fn from(value: &'_ Var) -> Self {
        value.clone()
    }
}
impl From<Var> for String {
    fn from(mut value: Var) -> Self {
        value.into_owned()
    }
}
impl From<Var> for Rc<String> {
    fn from(value: Var) -> Self {
        value.value
    }
}
impl Display for Var {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.value, f)
    }
}
impl Hash for Var {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.value.hash(state)
    }
}
impl Borrow<String> for Var {
    fn borrow(&self) -> &String {
        &self.value
    }
}
impl Borrow<str> for Var {
    fn borrow(&self) -> &str {
        &**self
    }
}
impl AsRef<String> for Var {
    fn as_ref(&self) -> &String {
        self.borrow()
    }
}
impl AsRef<str> for Var {
    fn as_ref(&self) -> &str {
        self.borrow()
    }
}
impl From<String> for Var {
    fn from(value: String) -> Self {
        Self { value: value.into() }
    }
}
impl From<&'_ String> for Var {
    fn from(value: &'_ String) -> Self {
        value.as_str().into()
    }
}
impl From<&'_ str> for Var {
    fn from(value: &'_ str) -> Self {
        Self { value: Rc::new(value.into()) }
    }
}
impl Deref for Var {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &**self.value
    }
}
impl Eq for Var { }
impl PartialEq for Var {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}
impl PartialEq<String> for Var {
    fn eq(&self, other: &String) -> bool {
        &**self == other
    }
}
impl PartialEq<str> for Var {
    fn eq(&self, other: &str) -> bool {
        *self == other
    }
}
impl PartialEq<&str> for Var {
    fn eq(&self, other: &&str) -> bool {
        **self == **other
    }
}
