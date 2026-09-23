use std::{borrow::Cow, marker::PhantomData};

use serde::{Serialize, de::DeserializeOwned};

use crate::{Cookie, Cookies};

/// A typed value stored as JSON in a single cookie.
///
/// A `CookieStore` holds a `T` in memory. Reads and changes only affect that
/// in-memory value. Nothing is written to the response until you call
/// [`commit`](Self::commit). Dropping the store, or calling
/// [`rollback`](Self::rollback), discards the changes. To update a cookie only
/// after some other work succeeds, call `commit` after that work.
///
/// The store reads and writes its cookie through a [`Cookies`] jar, so the
/// jar's signing, encryption, name prefix, and attributes apply to it.
///
/// Create one with [`cookie_store`] and one of the `parse*` methods:
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
    /// Creates a store for the cookie named `key` that holds `value`, without
    /// reading the cookie.
    ///
    /// To start from the cookie's current value, use [`cookie_store`] and one
    /// of the `parse*` methods instead.
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

    /// Replaces the in-memory value and returns the store.
    ///
    /// The change is not written until [`commit`](Self::commit).
    #[must_use]
    pub fn set(mut self, value: T) -> Self {
        self.value = value;
        self
    }

    /// Calls `f` to change the in-memory value in place and returns the store.
    ///
    /// The change is not written until [`commit`](Self::commit).
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

    /// Serializes the value to JSON, adds it to the jar as the cookie's new
    /// value, and returns the value.
    ///
    /// # Errors
    ///
    /// Returns `Err` if serializing `T` to JSON fails.
    pub fn commit(self) -> Result<T, serde_json::Error> {
        let serialized = serde_json::to_string(&self.value)?;
        self.jar.add(Cookie::new(self.key, serialized));
        Ok(self.value)
    }

    /// Discards the store and its uncommitted changes.
    ///
    /// This is the same as dropping the store, but makes the intent clear at
    /// the call site.
    pub fn rollback(self) {}

    /// Removes the cookie from the client and discards the in-memory value.
    ///
    /// The removal goes through the jar, which applies the same `Path`,
    /// `Domain`, and name prefix as when the cookie was written, so the browser
    /// matches and deletes it. See [`Cookies::remove`].
    pub fn remove(self) {
        self.jar.remove(Cookie::new(self.key, ""));
    }
}

/// A [`CookieStore`] that has not read its cookie yet.
///
/// Created by [`cookie_store`]. Call one of the `parse*` methods to read the
/// cookie and get a [`CookieStore`]. Use [`set`](Self::set) or
/// [`remove`](Self::remove) to skip reading it.
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
    /// Creates an unparsed store for the cookie named `key` in `jar`.
    ///
    /// Same as [`cookie_store`].
    pub fn new(jar: J, key: impl Into<Cow<'static, str>>) -> Self {
        Self {
            jar,
            key: key.into(),
            _marker: PhantomData,
        }
    }

    /// Returns a [`CookieStore`] that holds `value`, without reading the
    /// cookie.
    ///
    /// Use this to replace the cookie when its current value does not matter.
    /// The value is not written until you call
    /// [`commit`](CookieStore::commit) on the returned store.
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

    /// Removes the cookie from the client without reading it first.
    ///
    /// See [`CookieStore::remove`].
    pub fn remove(self) {
        self.jar.remove(Cookie::new(self.key, ""));
    }

    /// Reads the cookie and deserializes its value.
    ///
    /// Returns `Ok(None)` when the cookie is absent.
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

    /// Reads the cookie, or uses `default` when the cookie is absent or cannot
    /// be deserialized.
    pub fn parse_or(self, default: T) -> CookieStore<T, J> {
        self.parse_or_else(move || default)
    }

    /// Reads the cookie, or calls `f` for a value when the cookie is absent or
    /// cannot be deserialized.
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

    /// Reads the cookie, or uses `T::default()` when the cookie is absent or
    /// cannot be deserialized.
    pub fn parse_or_default(self) -> CookieStore<T, J>
    where
        T: Default,
    {
        self.parse_or_else(T::default)
    }
}

/// Creates an [`UnparsedCookieStore`] for the cookie named `key` in `jar`.
///
/// `jar` can be any [`Cookies`] jar, and its signing, encryption, name prefix,
/// and attributes apply to the stored cookie. Name the stored type as `T`:
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
    use std::{cell::RefCell, collections::HashMap};

    use http::{HeaderMap, Request, header};
    use serde::{Deserialize, Serialize};
    use topcoat_core::context::{Cx, CxTestBuilder};

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

        // And serialized into the jar under the right name.
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
        // On a parsed store.
        let parsed = MockJar::with(&[("cart", r#"{"items":[]}"#)]);
        cookie_store::<Cart, _>(&parsed, "cart")
            .parse_or_default()
            .remove();
        let removed = parsed.removed();
        assert_eq!(removed.len(), 1);
        assert_eq!(removed[0].name(), "cart");
        assert!(parsed.added().is_empty());

        // And directly on the unparsed store.
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

        CxTestBuilder::new()
            .request_context(parts)
            .request_context(CookieJarCell::new())
            .build()
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
