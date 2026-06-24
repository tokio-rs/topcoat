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
/// ```rust
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
///     // `commit` writes the cookie and hands the value back; without it the
///     // change is discarded.
///     let cart = cookie_store::<Cart, _>(private_cookies(cx), "cart")
///         .parse_or_default()
///         .update(|cart| cart.items.push("widget".to_owned()))
///         .commit()?;
///
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

    /// Replaces the in-memory value, returning the store so calls can be chained.
    ///
    /// Like every mutation, this is not persisted until [`commit`](Self::commit).
    #[must_use]
    pub fn set(mut self, value: T) -> Self {
        self.value = value;
        self
    }

    /// Mutates the in-memory value in place, returning the store so calls can be
    /// chained.
    ///
    /// Like every mutation, this is not persisted until [`commit`](Self::commit).
    ///
    /// ```rust
    /// # use serde::{Deserialize, Serialize};
    /// # use topcoat::{
    /// #     Result,
    /// #     context::Cx,
    /// #     cookie::{cookie_store, private_cookies},
    /// # };
    /// # #[derive(Default, Serialize, Deserialize)]
    /// # struct Cart {
    /// #     items: Vec<String>,
    /// # }
    /// # fn example(cx: &Cx) -> Result<(), topcoat::Error> {
    /// let cart = cookie_store::<Cart, _>(private_cookies(cx), "cart")
    ///     .parse_or_default()
    ///     .update(|cart| cart.items.push("widget".to_owned()))
    ///     .commit()?;
    /// #     Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn update<F>(mut self, f: F) -> Self
    where
        F: FnOnce(&mut T),
    {
        f(&mut self.value);
        self
    }

    /// Serializes the current value, queues it on the backing jar as a
    /// `Set-Cookie`, and returns the value.
    ///
    /// This is the only method that writes anything: until it is called, the
    /// store's value lives only in memory. Returns an error if the value cannot
    /// be serialized.
    ///
    /// # Errors
    ///
    /// Returns `Err` if serializing `T` to JSON fails.
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

    /// Queues a removal of the backing cookie, expiring it on the client.
    ///
    /// Writes to the response like [`commit`](Self::commit), but deletes the
    /// cookie instead of saving a value; the in-memory value is dropped. The
    /// removal goes through the jar, so the `Path`/`Domain` and prefix attributes
    /// the cookie was written with are reapplied and the browser can match it.
    pub fn remove(self) {
        self.jar.remove(Cookie::new(self.key, ""));
    }
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
    /// ```rust
    /// # use serde::{Deserialize, Serialize};
    /// # use topcoat::{
    /// #     Result,
    /// #     context::Cx,
    /// #     cookie::{cookie_store, private_cookies},
    /// # };
    /// # #[derive(Default, Serialize, Deserialize)]
    /// # struct Cart {
    /// #     items: Vec<String>,
    /// # }
    /// # fn example(cx: &Cx) -> Result<(), topcoat::Error> {
    /// cookie_store::<Cart, _>(private_cookies(cx), "cart")
    ///     .set(Cart::default())
    ///     .commit()?;
    /// #     Ok(())
    /// # }
    /// ```
    pub fn set(self, value: T) -> CookieStore<T, J> {
        CookieStore::new(self.jar, self.key, value)
    }

    /// Queues a removal of the backing cookie without reading it first.
    ///
    /// Use this to delete the cookie regardless of its current contents.
    pub fn remove(self) {
        self.jar.remove(Cookie::new(self.key, ""));
    }

    /// Reads and deserializes the backing cookie.
    ///
    /// Returns `Ok(None)` when the cookie is absent, and `Err` when it is
    /// present but cannot be deserialized.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the cookie is present but its value cannot be
    /// deserialized into `T`.
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
/// ```rust
/// # use serde::{Deserialize, Serialize};
/// # use topcoat::{
/// #     Result,
/// #     context::Cx,
/// #     cookie::{cookie_store, private_cookies},
/// # };
/// # #[derive(Default, Serialize, Deserialize)]
/// # struct Cart {
/// #     items: Vec<String>,
/// # }
/// # fn example(cx: &Cx) -> Result<(), topcoat::Error> {
/// let cart = cookie_store::<Cart, _>(private_cookies(cx), "cart").parse_or_default();
/// #     Ok(())
/// # }
/// ```
pub fn cookie_store<T, J>(jar: J, key: impl Into<Cow<'static, str>>) -> UnparsedCookieStore<T, J>
where
    T: Serialize + DeserializeOwned,
    J: Cookies,
{
    UnparsedCookieStore::new(jar, key)
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, collections::HashMap, sync::Arc};

    use http::{HeaderMap, Request, header, request::Parts};
    use serde::{Deserialize, Serialize};
    use topcoat_core::runtime::context::{ContextMap, Cx};

    use super::*;
    use crate::{CookieJarCell, Key, cookies, write_cookies};

    #[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
    struct Cart {
        items: Vec<String>,
    }

    /// A minimal [`Cookies`] jar for testing the store in isolation: it serves
    /// preset incoming cookies and records the cookies written through it.
    #[derive(Default)]
    struct MockJar {
        incoming: HashMap<String, String>,
        added: RefCell<Vec<Cookie<'static>>>,
        removed: RefCell<Vec<Cookie<'static>>>,
    }

    impl MockJar {
        fn with(pairs: &[(&str, &str)]) -> Self {
            Self {
                incoming: pairs
                    .iter()
                    .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
                    .collect(),
                ..Self::default()
            }
        }

        fn added(&self) -> Vec<Cookie<'static>> {
            self.added.borrow().clone()
        }

        fn removed(&self) -> Vec<Cookie<'static>> {
            self.removed.borrow().clone()
        }
    }

    impl Cookies for &MockJar {
        fn get(&self, name: &str) -> Option<Cookie<'static>> {
            self.incoming
                .get(name)
                .map(|value| Cookie::new(name.to_owned(), value.clone()))
        }

        fn add<C: Into<Cookie<'static>>>(&self, cookie: C) {
            self.added.borrow_mut().push(cookie.into());
        }

        fn remove<C: Into<Cookie<'static>>>(&self, cookie: C) {
            self.removed.borrow_mut().push(cookie.into());
        }
    }

    #[test]
    fn parse_reads_existing_value() {
        let jar = MockJar::with(&[("cart", r#"{"items":["a","b"]}"#)]);
        let store = cookie_store::<Cart, _>(&jar, "cart")
            .parse()
            .unwrap()
            .expect("cookie is present");

        assert_eq!(store.read().items, ["a", "b"]);
    }

    #[test]
    fn parse_returns_none_when_absent() {
        let jar = MockJar::with(&[]);
        assert!(
            cookie_store::<Cart, _>(&jar, "cart")
                .parse()
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn parse_errors_on_malformed() {
        let jar = MockJar::with(&[("cart", "not json")]);
        assert!(cookie_store::<Cart, _>(&jar, "cart").parse().is_err());
    }

    #[test]
    fn parse_or_default_falls_back_when_absent_or_malformed() {
        let absent = MockJar::with(&[]);
        assert_eq!(
            cookie_store::<Cart, _>(&absent, "cart")
                .parse_or_default()
                .get(),
            Cart::default()
        );

        // A malformed cookie is treated like a missing one, not an error.
        let malformed = MockJar::with(&[("cart", "not json")]);
        assert_eq!(
            cookie_store::<Cart, _>(&malformed, "cart")
                .parse_or_default()
                .get(),
            Cart::default()
        );
    }

    #[test]
    fn parse_or_and_parse_or_else_use_their_fallbacks() {
        let jar = MockJar::with(&[]);
        let from_value = cookie_store::<Cart, _>(&jar, "cart").parse_or(Cart {
            items: vec!["x".to_owned()],
        });
        assert_eq!(from_value.read().items, ["x"]);

        let from_closure = cookie_store::<Cart, _>(&jar, "cart").parse_or_else(|| Cart {
            items: vec!["y".to_owned()],
        });
        assert_eq!(from_closure.read().items, ["y"]);
    }

    #[test]
    fn commit_writes_serialized_cookie_and_returns_value() {
        let jar = MockJar::with(&[]);
        let cart = cookie_store::<Cart, _>(&jar, "cart")
            .parse_or_default()
            .update(|cart| cart.items.push("widget".to_owned()))
            .commit()
            .unwrap();

        // The value is handed back.
        assert_eq!(cart.items, ["widget"]);

        // …and serialized into the jar under the right name.
        let added = jar.added();
        assert_eq!(added.len(), 1);
        assert_eq!(added[0].name(), "cart");
        assert_eq!(added[0].value(), r#"{"items":["widget"]}"#);
    }

    #[test]
    fn get_clones_and_read_borrows() {
        let jar = MockJar::with(&[("cart", r#"{"items":["a"]}"#)]);
        let store = cookie_store::<Cart, _>(&jar, "cart").parse_or_default();

        assert_eq!(store.read().items, ["a"]);
        assert_eq!(store.get(), *store.read());
    }

    #[test]
    fn nothing_is_written_without_commit() {
        let jar = MockJar::with(&[]);
        let _ = cookie_store::<Cart, _>(&jar, "cart")
            .parse_or_default()
            .update(|cart| cart.items.push("z".to_owned()));

        assert!(jar.added().is_empty());
    }

    #[test]
    fn rollback_writes_nothing() {
        let jar = MockJar::with(&[]);
        cookie_store::<Cart, _>(&jar, "cart")
            .parse_or_default()
            .update(|cart| cart.items.push("z".to_owned()))
            .rollback();

        assert!(jar.added().is_empty());
    }

    #[test]
    fn unparsed_set_overwrites_without_reading() {
        let jar = MockJar::with(&[("cart", r#"{"items":["old"]}"#)]);
        cookie_store::<Cart, _>(&jar, "cart")
            .set(Cart {
                items: vec!["new".to_owned()],
            })
            .commit()
            .unwrap();

        assert_eq!(jar.added()[0].value(), r#"{"items":["new"]}"#);
    }

    #[test]
    fn remove_queues_a_removal() {
        // On a parsed store…
        let parsed = MockJar::with(&[("cart", r#"{"items":[]}"#)]);
        cookie_store::<Cart, _>(&parsed, "cart")
            .parse_or_default()
            .remove();
        let removed = parsed.removed();
        assert_eq!(removed.len(), 1);
        assert_eq!(removed[0].name(), "cart");
        assert!(parsed.added().is_empty());

        // …and directly on the unparsed store.
        let unparsed = MockJar::with(&[]);
        cookie_store::<Cart, _>(&unparsed, "cart").remove();
        assert_eq!(unparsed.removed()[0].name(), "cart");
    }

    /// Builds a `Cx` whose request carries the given raw `Cookie` header values.
    fn cx_with(cookie_headers: &[&str]) -> Cx {
        let mut builder = Request::builder();
        for value in cookie_headers {
            builder = builder.header(header::COOKIE, *value);
        }
        let (parts, ()) = builder.body(()).unwrap().into_parts();

        let mut request_context = ContextMap::new();
        request_context.insert::<Parts>(parts);
        request_context.insert(CookieJarCell::new());
        Cx::new(Arc::new(ContextMap::new()), request_context)
    }

    /// The leading `name=value` pair of the request's first `Set-Cookie` value.
    fn echoed_pair(cx: &Cx) -> String {
        let mut headers = HeaderMap::new();
        write_cookies(cx, &mut headers);
        let set = headers
            .get(header::SET_COOKIE)
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned();
        set.split(';').next().unwrap().to_owned()
    }

    #[test]
    fn round_trips_through_a_signed_jar() {
        // Proves the store's serialization composes with the jar's layers: write
        // through a signed jar, then read the echoed cookie back through one.
        let key = Key::generate();

        let writer = cx_with(&[]);
        cookie_store::<Cart, _>(cookies(&writer).signed(&key), "cart")
            .set(Cart {
                items: vec!["widget".to_owned()],
            })
            .commit()
            .unwrap();
        let echoed = echoed_pair(&writer);

        let reader = cx_with(&[&echoed]);
        let cart = cookie_store::<Cart, _>(cookies(&reader).signed(&key), "cart")
            .parse()
            .unwrap()
            .expect("the signed cookie should verify and deserialize");

        assert_eq!(cart.read().items, ["widget"]);
    }
}
