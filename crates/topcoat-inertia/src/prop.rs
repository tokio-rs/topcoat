use std::borrow::Cow;
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use serde::Serialize;
use serde_json::Value;
use topcoat_core::error::Result;

use crate::ScrollMetadata;

pub struct Prop<'cx> {
    pub(crate) source: PropSource<'cx>,
    pub(crate) initial: InitialBehavior,
    pub(crate) always: bool,
    pub(crate) merge: Option<MergeBehavior>,
    pub(crate) once: Option<OnceBehavior>,
    pub(crate) scroll: Option<ScrollBehavior>,
    pub(crate) rescue: bool,
}

impl fmt::Debug for Prop<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Prop")
            .field("initial", &self.initial)
            .field("always", &self.always)
            .field("merge", &self.merge)
            .field("once", &self.once)
            .field("scroll", &self.scroll)
            .field("rescue", &self.rescue)
            .finish_non_exhaustive()
    }
}

impl Prop<'static> {
    #[must_use]
    pub fn value(value: impl Serialize) -> Self {
        Self::from_result(serde_json::to_value(value).map_err(Into::into))
    }

    pub(crate) fn from_result(value: Result<Value>) -> Self {
        Self::new(PropSource::Value(value))
    }
}

impl<'cx> Prop<'cx> {
    fn new(source: PropSource<'cx>) -> Self {
        Self {
            source,
            initial: InitialBehavior::Include,
            always: false,
            merge: None,
            once: None,
            scroll: None,
            rescue: false,
        }
    }

    fn future<T>(future: impl Future<Output = Result<T>> + Send + 'cx) -> Self
    where
        T: Serialize,
    {
        Self::new(PropSource::Future(Box::pin(async move {
            let value = future.await?;
            Ok(serde_json::to_value(value)?)
        })))
    }

    #[must_use]
    pub fn group(mut self, group: impl Into<Cow<'static, str>>) -> Self {
        self.initial = InitialBehavior::Deferred {
            group: group.into(),
        };
        self
    }

    #[must_use]
    pub fn rescue(mut self) -> Self {
        self.rescue = true;
        self
    }

    #[must_use]
    pub fn always(mut self) -> Self {
        self.always = true;
        self
    }

    #[must_use]
    pub fn merge(mut self) -> Self {
        self.merge = Some(MergeBehavior::append_root());
        self
    }

    #[must_use]
    pub fn deep_merge(mut self) -> Self {
        self.merge.get_or_insert_with(MergeBehavior::default).deep = true;
        self
    }

    #[must_use]
    pub fn append(mut self) -> Self {
        self.merge
            .get_or_insert_with(MergeBehavior::default)
            .append_root = true;
        self
    }

    #[must_use]
    pub fn append_at(mut self, path: impl Into<String>) -> Self {
        self.merge
            .get_or_insert_with(MergeBehavior::default)
            .append
            .push(path.into());
        self
    }

    #[must_use]
    pub fn prepend(mut self) -> Self {
        self.merge
            .get_or_insert_with(MergeBehavior::default)
            .prepend_root = true;
        self
    }

    #[must_use]
    pub fn prepend_at(mut self, path: impl Into<String>) -> Self {
        self.merge
            .get_or_insert_with(MergeBehavior::default)
            .prepend
            .push(path.into());
        self
    }

    #[must_use]
    pub fn match_on(mut self, path: impl Into<String>) -> Self {
        self.merge
            .get_or_insert_with(MergeBehavior::default)
            .match_on
            .push(path.into());
        self
    }

    #[must_use]
    pub fn once(mut self) -> Self {
        self.once.get_or_insert_with(OnceBehavior::default);
        self
    }

    #[must_use]
    pub fn as_key(mut self, key: impl Into<String>) -> Self {
        self.once.get_or_insert_with(OnceBehavior::default).key = Some(key.into());
        self
    }

    #[must_use]
    pub fn until(mut self, duration: Duration) -> Self {
        self.once.get_or_insert_with(OnceBehavior::default).expires = Some(duration);
        self
    }

    #[must_use]
    pub fn fresh(mut self) -> Self {
        self.once.get_or_insert_with(OnceBehavior::default).fresh = true;
        self
    }

    #[must_use]
    pub fn defer(mut self) -> Self {
        self.initial = InitialBehavior::Deferred {
            group: Cow::Borrowed("default"),
        };
        self
    }

    #[must_use]
    pub fn scroll(mut self, metadata: ScrollMetadata) -> Self {
        self.scroll = Some(ScrollBehavior {
            metadata,
            wrapper: None,
        });
        self
    }

    #[must_use]
    pub fn wrapper(mut self, path: impl Into<String>) -> Self {
        if let Some(scroll) = &mut self.scroll {
            scroll.wrapper = Some(path.into());
        }
        self
    }

    pub(crate) fn validate(&self, path: &str) -> Result<()> {
        if self.always && self.initial != InitialBehavior::Include {
            return Err(
                InvalidProp::new(path, "always props cannot be optional or deferred").into(),
            );
        }
        if self.rescue && !matches!(self.initial, InitialBehavior::Deferred { .. }) {
            return Err(InvalidProp::new(path, "only deferred props can be rescued").into());
        }
        if self.scroll.is_some() && self.once.is_some() {
            return Err(InvalidProp::new(path, "scroll props cannot be once props").into());
        }
        if let Some(merge) = &self.merge
            && merge.deep
            && (merge.append_root
                || merge.prepend_root
                || !merge.append.is_empty()
                || !merge.prepend.is_empty())
        {
            return Err(InvalidProp::new(
                path,
                "deep merge cannot be combined with append or prepend",
            )
            .into());
        }
        if self
            .scroll
            .as_ref()
            .and_then(|scroll| scroll.wrapper.as_deref())
            == Some("")
        {
            return Err(InvalidProp::new(path, "scroll wrapper cannot be empty").into());
        }
        Ok(())
    }
}

pub(crate) enum PropSource<'cx> {
    Value(Result<Value>),
    Future(Pin<Box<dyn Future<Output = Result<Value>> + Send + 'cx>>),
}

impl PropSource<'_> {
    pub(crate) async fn resolve(self) -> Result<Value> {
        match self {
            Self::Value(value) => value,
            Self::Future(future) => future.await,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum InitialBehavior {
    Include,
    Optional,
    Deferred { group: Cow<'static, str> },
}

#[derive(Debug, Default)]
pub(crate) struct MergeBehavior {
    pub(crate) deep: bool,
    pub(crate) append_root: bool,
    pub(crate) prepend_root: bool,
    pub(crate) append: Vec<String>,
    pub(crate) prepend: Vec<String>,
    pub(crate) match_on: Vec<String>,
}

impl MergeBehavior {
    fn append_root() -> Self {
        Self {
            append_root: true,
            ..Self::default()
        }
    }
}

#[derive(Debug, Default)]
pub(crate) struct OnceBehavior {
    pub(crate) key: Option<String>,
    pub(crate) expires: Option<Duration>,
    pub(crate) fresh: bool,
}

#[derive(Debug)]
pub(crate) struct ScrollBehavior {
    pub(crate) metadata: ScrollMetadata,
    pub(crate) wrapper: Option<String>,
}

#[derive(Debug)]
struct InvalidProp {
    path: String,
    reason: &'static str,
}

impl InvalidProp {
    fn new(path: &str, reason: &'static str) -> Self {
        Self {
            path: path.to_owned(),
            reason,
        }
    }
}

impl fmt::Display for InvalidProp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid Inertia prop `{}`: {}", self.path, self.reason)
    }
}

impl std::error::Error for InvalidProp {}

#[must_use]
pub fn value(value: impl Serialize) -> Prop<'static> {
    Prop::value(value)
}

pub fn lazy<'cx, T>(future: impl Future<Output = Result<T>> + Send + 'cx) -> Prop<'cx>
where
    T: Serialize,
{
    Prop::future(future)
}

#[must_use]
pub fn always(serializable: impl Serialize) -> Prop<'static> {
    value(serializable).always()
}

pub fn optional<'cx, T>(future: impl Future<Output = Result<T>> + Send + 'cx) -> Prop<'cx>
where
    T: Serialize,
{
    let mut prop = Prop::future(future);
    prop.initial = InitialBehavior::Optional;
    prop
}

pub fn defer<'cx, T>(future: impl Future<Output = Result<T>> + Send + 'cx) -> Prop<'cx>
where
    T: Serialize,
{
    Prop::future(future).defer()
}

#[must_use]
pub fn merge(serializable: impl Serialize) -> Prop<'static> {
    value(serializable).merge()
}

pub fn once<'cx, T>(future: impl Future<Output = Result<T>> + Send + 'cx) -> Prop<'cx>
where
    T: Serialize,
{
    Prop::future(future).once()
}

#[must_use]
pub fn scroll(value: impl Serialize, metadata: ScrollMetadata) -> Prop<'static> {
    Prop::value(value).scroll(metadata)
}
