use std::borrow::Cow;
use std::fmt::{Debug, Display, Formatter};
use std::str::pattern::{Pattern, Searcher};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub enum IdentifierInner<'a> {
    Full(Cow<'a, str>),
    Partial(Cow<'a, str>, Cow<'a, str>),
}

#[repr(transparent)]
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(try_from = "String", into = "String")]
pub struct Identifier<'a>(IdentifierInner<'a>);

impl<'a> Identifier<'a> {
    /// Safety. Full string should contain only one ':'
    pub const unsafe fn new_full_unchecked(full: Cow<'a, str>) -> Self {
        Self(IdentifierInner::Full(full))
    }

    /// Safety. Key and value should contain no ':'
    pub const unsafe fn new_partial_unchecked(key: Cow<'a, str>, value: Cow<'a, str>) -> Self {
        Self(IdentifierInner::Partial(key, value))
    }

    /// Safety. Identifier inner should be valid for identifier being
    pub const unsafe fn from_inner_unchecked(inner: IdentifierInner<'a>) -> Self {
        Self(inner)
    }

    /// Safety. Identifier inner should be valid for identifier being
    pub unsafe fn from_inner_ref_unchecked(inner: &'a IdentifierInner) -> Self {
        Self(match inner {
            IdentifierInner::Full(full) =>
                IdentifierInner::Full(Cow::Borrowed(full.as_ref())),
            IdentifierInner::Partial(key, value) =>
                IdentifierInner::Partial(Cow::Borrowed(key.as_ref()), Cow::Borrowed(value.as_ref()))
        })
    }

    pub fn as_reference(&'a self) -> Identifier {
        unsafe {
            Self::from_inner_ref_unchecked(&self.0)
        }
    }

    pub fn into_inner(self) -> IdentifierInner<'a> {
        self.0
    }

    pub const fn get_inner(&self) -> &IdentifierInner<'a> {
        &self.0
    }

    pub fn new_full(full: Cow<'a, str>) -> Option<Self> {
        let mut searcher = ':'.into_searcher(full.as_ref());
        match searcher.next_match().is_some() && searcher.next_match().is_none() {
            true => Some(unsafe { Self::new_full_unchecked(full) }),
            false => None,
        }
    }

    pub fn new_partial(key: Cow<'a, str>, value: Cow<'a, str>) -> Option<Self> {
        match key.contains(':') || value.contains(':') {
            true => None,
            false => Some(unsafe { Self::new_partial_unchecked(key, value) }),
        }
    }

    pub fn get_full(&self) -> Cow<str> {
        match self.get_inner() {
            IdentifierInner::Full(full) => Cow::Borrowed(full.as_ref()),
            IdentifierInner::Partial(key, value) => Cow::Owned(format!("{}:{}", key, value))
        }
    }

    pub fn get_partial(&self) -> (&str, &str) {
        match self.get_inner() {
            IdentifierInner::Full(full) => {
                let full = full.as_ref();
                let double_dot_index = unsafe { full.find(':').unwrap_unchecked() };
                (&full[0..double_dot_index], &full[(double_dot_index + 1)..full.len()])
            }
            IdentifierInner::Partial(key, value) => (key.as_ref(), value.as_ref())
        }
    }

    pub fn into_full(self) -> Cow<'a, str> {
        match self.into_inner() {
            IdentifierInner::Full(full) => full,
            IdentifierInner::Partial(key, value) => Cow::Owned(format!("{}:{}", key, value)),
        }
    }

    pub fn into_partial(self) -> (Cow<'a, str>, Cow<'a, str>) {
        match self.into_inner() {
            IdentifierInner::Full(full) => {
                let full = full.as_ref();
                let double_dot_index = unsafe { full.find(':').unwrap_unchecked() };
                (Cow::Owned(full[0..double_dot_index].to_owned()), Cow::Owned(full[(double_dot_index + 1)..full.len()].to_owned()))
            }
            IdentifierInner::Partial(key, value) => (key, value)
        }
    }
}

impl<'a> Display for Identifier<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self.get_inner() {
            IdentifierInner::Full(full) => write!(f, "{}", full),
            IdentifierInner::Partial(key, value) => write!(f, "{}:{}", key, value)
        }
    }
}

impl<'a> PartialEq for Identifier<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.get_partial() == other.get_partial()
    }
}

impl<'a> From<Identifier<'a>> for String {
    fn from(identifier: Identifier<'a>) -> Self {
        match identifier.into_inner() {
            IdentifierInner::Full(full) => full.into_owned(),
            IdentifierInner::Partial(key, value) => format!("{}:{}", key, value)
        }
    }
}

impl<'a> From<Identifier<'a>> for Cow<'a, str> {
    fn from(identifier: Identifier<'a>) -> Self {
        match identifier.into_inner() {
            IdentifierInner::Full(full) => full,
            IdentifierInner::Partial(key, value) => Cow::Owned(format!("{}:{}", key, value))
        }
    }
}

#[derive(thiserror::Error, Debug)]
#[error("Parsing of identifier is failed")]
pub struct IdentifierParseError;

impl<'a> TryFrom<&'a str> for Identifier<'a> {
    type Error = IdentifierParseError;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        Identifier::new_full(Cow::Borrowed(value)).ok_or(IdentifierParseError)
    }
}

impl<'a> TryFrom<String> for Identifier<'a> {
    type Error = IdentifierParseError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Identifier::new_full(Cow::Owned(value)).ok_or(IdentifierParseError)
    }
}