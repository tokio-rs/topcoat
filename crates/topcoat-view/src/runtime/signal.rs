use serde::{Deserialize, Serialize, de::DeserializeOwned};
use uuid::Uuid;

use crate::runtime::{IntoViewParts, Unescaped, View, ViewPart};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SignalId(Uuid);

impl SignalId {
    #[inline]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for SignalId {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Serialize, Deserialize)]
pub struct Signal<T> {
    id: SignalId,
    value: T,
}

impl<T> Signal<T> {
    #[inline]
    pub fn new(value: T) -> Self {
        Self {
            id: SignalId::new(),
            value,
        }
    }
}

pub struct SignalDeclaration<'a, T>(&'a Signal<T>);

impl<'a, T> SignalDeclaration<'a, T> {
    #[inline]
    pub fn new(signal: &'a Signal<T>) -> Self {
        Self(signal)
    }
}

impl<T> IntoViewParts for SignalDeclaration<'_, T>
where
    T: Serialize + DeserializeOwned,
{
    fn into_view_parts(self) -> impl Iterator<Item = super::ViewPart> {
        [
            ViewPart::UnescapedStaticStr(Unescaped::new_unchecked("<!-- signal: ")),
            ViewPart::UnescapedString(Unescaped::new_unchecked(
                serde_json::to_string(&self.0).unwrap(),
            )),
            ViewPart::UnescapedStaticStr(Unescaped::new_unchecked(" -->")),
        ]
        .into_iter()
    }
}

pub trait Signals {
    fn ids(&self) -> impl Iterator<Output = SignalId>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct ReactiveScopeId(Uuid);

impl ReactiveScopeId {
    #[inline]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for ReactiveScopeId {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

pub struct ReactiveScope<S> {
    id: ReactiveScopeId,
    signals: &S,
    placeholder: View,
}

impl ReactiveScope {
    #[inline]
    pub fn new(signals: &S, placeholder: View) -> Self {
        Self {
            id: ReactiveScopeId::new(),
            signals,
            placeholder,
        }
    }
}

impl<S, B> IntoViewParts for ReactiveScope<S, B>
where
    S: Signals,
{
    fn into_view_parts(self) -> impl Iterator<Item = ViewPart> {
        [
            ViewPart::UnescapedStaticStr(Unescaped::new_unchecked("<!-- reactive scope start: ")),
            ViewPart::UnescapedString(Unescaped::new_unchecked(
                serde_json::to_string(&self.id).unwrap(),
            )),
            ViewPart::UnescapedStaticStr(Unescaped::new_unchecked(" ")),
            ViewPart::UnescapedString(Unescaped::new_unchecked(
                serde_json::to_string(self.signals.ids().collect::<Vec<_>>()).unwrap(),
            )),
            ViewPart::UnescapedStaticStr(Unescaped::new_unchecked(" -->")),
            self.placeholder.into_inner(),
            ViewPart::UnescapedStaticStr(Unescaped::new_unchecked("<!-- reactive scope end: ")),
            ViewPart::UnescapedString(Unescaped::new_unchecked(
                serde_json::to_string(&self.id).unwrap(),
            )),
            ViewPart::UnescapedStaticStr(Unescaped::new_unchecked(" -->")),
        ]
        .into_iter()
    }
}

pub trait ReactiveScopeBody<S, E> {
    fn run(&self, signals: &S) -> Result<View, E>;
}
