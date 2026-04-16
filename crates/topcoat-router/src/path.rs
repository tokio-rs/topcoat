use std::{borrow::Borrow, fmt::Display, ops::Deref};

use ref_cast::RefCast;

use crate::Segment;

#[derive(Debug, PartialEq, Eq, Hash, RefCast)]
#[repr(transparent)]
pub struct Path {
    inner: str,
}

impl Path {
    pub fn new<S: AsRef<str> + ?Sized>(s: &S) -> &Self {
        let inner = s.as_ref();
        if !inner.starts_with("/") {
            panic!("paths must start with \"/\"");
        }
        Self::ref_cast(inner)
    }

    pub fn segments(&self) -> impl Iterator<Item = Segment<'_>> {
        self.inner.split("/").skip(1).map(Segment::new)
    }

    pub fn to_axum_path(&self) -> String {
        self.segments()
            .filter(|s| !s.is_group())
            .collect::<PathBuf>()
            .inner
    }

    pub fn as_str(&self) -> &str {
        &self.inner
    }
}

impl Display for Path {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}

impl ToOwned for Path {
    type Owned = PathBuf;

    fn to_owned(&self) -> Self::Owned {
        PathBuf {
            inner: self.inner.to_owned(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PathBuf {
    inner: String,
}

impl Borrow<Path> for PathBuf {
    fn borrow(&self) -> &Path {
        Path::new(&self.inner)
    }
}

impl Deref for PathBuf {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        Path::ref_cast(&self.inner)
    }
}

impl<'a> FromIterator<Segment<'a>> for PathBuf {
    fn from_iter<T: IntoIterator<Item = Segment<'a>>>(iter: T) -> Self {
        let mut buf = String::new();
        for segment in iter {
            use std::fmt::Write;
            write!(buf, "/{segment}").unwrap();
        }
        Self { inner: buf }
    }
}

impl Display for PathBuf {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}
