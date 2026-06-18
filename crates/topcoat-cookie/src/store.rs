use std::{borrow::Cow, marker::PhantomData};

use serde::{Serialize, de::DeserializeOwned};

use crate::{Cookie, Cookies};

/// A typed value backed by a single cookie.
///
/// A `CookieStore` holds a deserialized `T` in memory and writes it back to its
/// cookie as JSON on [`commit`](Self::commit). It is built from any [`Cookies`]
/// jar, so signing, encryption, prefixes, and default attributes all compose
/// through the jar it wraps.
///
/// Reads and mutations operate on the in-memory value only. **Nothing is written
/// to the response until [`commit`](Self::commit) is called**; dropping the store
/// (or calling [`rollback`](Self::rollback)) discards any pending changes. This
/// makes it easy to update a cookie only once some other work has succeeded —
/// just hold off on `commit` until then.
///
/// Obtain one by reading the incoming cookie through [`cookie_store`]:
///
/// ```rust,ignore
/// use serde::{Deserialize, Serialize};
/// use topcoat::{
///     Result,
///     context::Cx,
///     cookie::{cookie_store, private_cookies},
///     router::route,
/// };
///
/// #[derive(Default, Serialize, Deserialize)]
/// struct Cart {
///     items: Vec<String>,
/// }
///
/// #[route(POST "/api/cart")]
/// async fn add_item(cx: &Cx) -> Result<String> {
///     let mut cart = cookie_store::<Cart>(private_cookies(cx), "cart").parse_or_default();
///
///     cart.update(|cart| cart.items.push("widget".to_owned()));
///
///     // `commit` writes the cookie and hands the value back; without it the
///     // change is discarded.
///     let cart = cart.commit()?;
///     Ok(format!("{} items in cart", cart.items.len()))
/// }
/// ```
pub struct CookieStore<T, J> {
    jar: J,
    key: Cow<'static, str>,
    value: T,
}

impl<T, J> CookieStore<T, J>
where
    T: Serialize + DeserializeOwned,
    J: Cookies,
{
    /// Builds a store around an already-known `value`.
    ///
    /// Most code instead goes through [`cookie_store`] and one of the `parse*`
    /// methods, which read the existing cookie first.
    pub fn new(jar: J, key: impl Into<Cow<'static, str>>, value: T) -> Self {
        Self {
            jar,
            key: key.into(),
            value,
        }
    }

    /// Returns a reference to the in-memory value.
    pub fn read(&self) -> &T {
        &self.value
    }

    /// Returns a clone of the in-memory value.
    pub fn get(&self) -> T
    where
        T: Clone,
    {
        self.value.clone()
    }

    /// Replaces the in-memory value.
    ///
    /// Like every mutation, this is not persisted until [`commit`](Self::commit).
    pub fn set(&mut self, value: T) {
        self.value = value;
    }

    /// Mutates the in-memory value in place, returning whatever `f` returns.
    ///
    /// Like every mutation, this is not persisted until [`commit`](Self::commit).
    ///
    /// ```rust,ignore
    /// let count = cart.update(|cart| {
    ///     cart.items.push("widget".to_owned());
    ///     cart.items.len()
    /// });
    /// ```
    pub fn update<F, R>(&mut self, f: F) -> R
    where
        F: FnOnce(&mut T) -> R,
    {
        f(&mut self.value)
    }

    /// Serializes the current value, queues it on the backing jar as a
    /// `Set-Cookie`, and returns the value.
    ///
    /// This is the only method that writes anything: until it is called, the
    /// store's value lives only in memory. Returns an error if the value cannot
    /// be serialized.
    pub fn commit(self) -> Result<T, serde_json::Error> {
        let serialized = serde_json::to_string(&self.value)?;
        self.jar.add(Cookie::new(self.key, serialized));
        Ok(self.value)
    }

    /// Discards the store along with any uncommitted changes.
    ///
    /// Equivalent to dropping the store without calling [`commit`](Self::commit);
    /// it exists to make that intent explicit at the call site.
    pub fn rollback(self) {}
}

/// A [`CookieStore`] that has not yet read its backing cookie.
///
/// Created by [`cookie_store`]. Call one of the `parse*` methods to read and
/// deserialize the cookie, yielding a [`CookieStore`] you can read and mutate.
pub struct UnparsedCookieStore<T, J> {
    jar: J,
    key: Cow<'static, str>,
    _marker: PhantomData<fn() -> T>,
}

impl<T, J> UnparsedCookieStore<T, J>
where
    T: Serialize + DeserializeOwned,
    J: Cookies,
{
    /// Builds an unparsed store for the cookie named `key`, backed by `jar`.
    ///
    /// Usually called through [`cookie_store`].
    pub fn new(jar: J, key: impl Into<Cow<'static, str>>) -> Self {
        Self {
            jar,
            key: key.into(),
            _marker: PhantomData,
        }
    }

    /// Seeds a [`CookieStore`] with `value` without reading the existing cookie.
    ///
    /// Use this to overwrite the cookie outright when you don't need its current
    /// contents. The value is not written until the returned store is
    /// [`commit`](CookieStore::commit)ted.
    ///
    /// ```rust,ignore
    /// cookie_store::<Cart>(private_cookies(cx), "cart")
    ///     .set(Cart::default())
    ///     .commit()?;
    /// ```
    pub fn set(self, value: T) -> CookieStore<T, J> {
        CookieStore::new(self.jar, self.key, value)
    }

    /// Reads and deserializes the backing cookie.
    ///
    /// Returns `Ok(None)` when the cookie is absent, and `Err` when it is
    /// present but cannot be deserialized.
    pub fn parse(self) -> Result<Option<CookieStore<T, J>>, serde_json::Error> {
        match self.jar.get(&self.key) {
            Some(cookie) => {
                let value = serde_json::from_str(cookie.value())?;
                Ok(Some(CookieStore::new(self.jar, self.key, value)))
            }
            None => Ok(None),
        }
    }

    /// Parses the backing cookie, falling back to `default` when it is absent or
    /// malformed.
    pub fn parse_or(self, default: T) -> CookieStore<T, J> {
        self.parse_or_else(move || default)
    }

    /// Parses the backing cookie, falling back to `f()` when it is absent or
    /// malformed.
    pub fn parse_or_else<F>(self, f: F) -> CookieStore<T, J>
    where
        F: FnOnce() -> T,
    {
        let value = match self.jar.get(&self.key) {
            Some(cookie) => serde_json::from_str(cookie.value()).unwrap_or_else(|_| f()),
            None => f(),
        };
        CookieStore::new(self.jar, self.key, value)
    }

    /// Parses the backing cookie, falling back to `T::default()` when it is
    /// absent or malformed.
    pub fn parse_or_default(self) -> CookieStore<T, J>
    where
        T: Default,
    {
        self.parse_or_else(T::default)
    }
}

/// Builds an [`UnparsedCookieStore`] for the cookie named `key`, backed by `jar`.
///
/// `jar` is any [`Cookies`] jar, so signing, encryption, prefixes, and default
/// attributes compose through it. Specify the stored type as `T`:
///
/// ```rust,ignore
/// let cart = cookie_store::<Cart>(private_cookies(cx), "cart").parse_or_default();
/// ```
pub fn cookie_store<T, J>(jar: J, key: impl Into<Cow<'static, str>>) -> UnparsedCookieStore<T, J>
where
    T: Serialize + DeserializeOwned,
    J: Cookies,
{
    UnparsedCookieStore::new(jar, key)
}
