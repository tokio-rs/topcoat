use std::marker::PhantomData;

use ref_cast::RefCast;
use serde::Serialize;

use crate::{
    BoolSurrogate, ResultSurrogate, StrSurrogate, Surrogate, Surrogated, impl_surrogate,
    impl_surrogate_ref,
};

/// A typed reference to the current HTML element with a stable ID.
///
/// Create the ID with [`element_id`](topcoat_view::element_id), use [`id`](Self::id)
/// in the markup, and resolve the reference inside an event handler. Capturing a
/// reference sends only its ID and expected element type to the browser.
///
/// ```rust
/// use topcoat::{
///     Result,
///     context::Cx,
///     runtime::{ElementRef, InputElement, signal},
///     view::{View, component, element_id, view},
/// };
///
/// #[component]
/// async fn field(cx: &Cx) -> Result<impl View> {
///     let input = ElementRef::<InputElement>::new(element_id(cx));
///     let available = signal(cx, || false);
///     Ok(view! {
///         <input id=(input.id())>
///         <button @click=$(|_e| available.set(input.resolve().is_ok()))>
///             "Check input"
///         </button>
///         <p>$(available.get())</p>
///     })
/// }
/// ```
///
/// Resolution is available through [`expr!`](https://docs.rs/topcoat/latest/topcoat/runtime/macro.expr.html), on
/// [`ElementRefSurrogate::resolve`]. Each call looks up the ID again, so a
/// replacement resolves to the new element and a removal reports a missing
/// element. IDs must be unique in the document. Use keyed component instances
/// or [`Cx::keyed`](topcoat_core::context::Cx::keyed) in loops, as with `element_id`.
#[derive(Debug, Clone)]
pub struct ElementRef<T = HtmlElement> {
    id: String,
    element: PhantomData<T>,
}

impl<T: ElementType> ElementRef<T> {
    /// References the element identified by `id`, expecting type `T`.
    ///
    /// This does not look up the element or check its type on the server.
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            element: PhantomData,
        }
    }

    /// The HTML ID to use for this element and attributes that refer to it.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }
}

/// An [`ElementRef`] captured by a runtime expression.
#[derive(Debug, Clone, RefCast)]
#[repr(transparent)]
pub struct ElementRefSurrogate<T>(ElementRef<T>);

impl<T: ElementType> ElementRefSurrogate<T> {
    pub(crate) fn new(value: ElementRef<T>) -> Self {
        Self(value)
    }

    /// The reference's HTML ID.
    #[must_use]
    pub fn id(&self) -> &StrSurrogate {
        self.0.id().into_surrogate()
    }

    /// Looks up the current element and checks its native HTML type.
    ///
    /// Returns an error when the ID is missing or the element has the wrong
    /// type. Resolve at the point of use, including after an `await`. A resolved
    /// handle refers to the node found by that call and cannot be serialized.
    /// DOM changes do not cause expressions to rerun.
    ///
    /// # Panics
    ///
    /// Always panics on the server. Call inside a browser event-handler closure.
    #[must_use]
    pub fn resolve(&self) -> ResultSurrogate<T, ElementRefError> {
        panic!("element references cannot be resolved on the server");
    }
}

impl_surrogate!({T: ElementType} ElementRef<T>, ElementRefSurrogate<T>);
impl_surrogate_ref!({T: ElementType} ElementRef<T>, ElementRefSurrogate<T>);

impl<T: ElementType> Serialize for ElementRefSurrogate<T> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct Encoded<'a> {
            t: &'static str,
            id: &'a str,
            kind: &'static str,
        }

        Encoded {
            t: "ElementRef",
            id: self.0.id(),
            kind: T::KIND,
        }
        .serialize(serializer)
    }
}

/// A native HTML element type supported by [`ElementRef`].
///
/// This trait is sealed. Resolved elements exist only in the browser and cannot
/// be captured, stored in signals, or sent in requests.
///
/// Resolved handles cannot be captured:
///
/// ```compile_fail
/// use topcoat::runtime::{InputElement, expr};
/// fn capture(element: InputElement) {
///     let _ = expr!(element);
/// }
/// ```
///
/// They cannot be signal values:
///
/// ```compile_fail
/// use topcoat::{context::Cx, runtime::{InputElement, signal}};
/// fn store(cx: &Cx, element: InputElement) {
///     let _ = signal(cx, || element);
/// }
/// ```
///
/// Procedure arguments also reject resolved handles:
///
/// ```compile_fail
/// use topcoat::{Result, runtime::{InputElement, procedure}};
/// #[procedure]
/// async fn send(element: InputElement) -> Result<()> {
///     Ok(())
/// }
/// ```
///
/// A reference's expected type determines the resolved handle's type:
///
/// ```compile_fail
/// use topcoat::runtime::{DialogElement, ElementRef, InputElement, expr};
/// let input = ElementRef::<InputElement>::new("field");
/// let _ = expr!(|| -> DialogElement { input.resolve().unwrap() });
/// ```
pub trait ElementType: private::Sealed + Surrogated<Surrogate = Self> {}

mod private {
    pub trait Sealed {
        const KIND: &'static str;
    }
}

macro_rules! element_type {
    ($name:ident, $kind:literal, $doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone)]
        pub struct $name {
            _browser_only: std::convert::Infallible,
        }

        impl private::Sealed for $name {
            const KIND: &'static str = $kind;
        }

        impl ElementType for $name {}

        impl Surrogated for $name {
            type Surrogate = Self;

            fn into_surrogate(self) -> Self {
                self
            }
        }

        impl Surrogate for $name {
            type Real = Self;

            fn into_real(self) -> Self {
                self
            }
        }
    };
}

element_type!(
    HtmlElement,
    "html",
    "A browser-only handle to an HTML element."
);
element_type!(
    InputElement,
    "input",
    "A browser-only handle to an HTML input element."
);
element_type!(
    DialogElement,
    "dialog",
    "A browser-only handle to an HTML dialog element."
);

/// The reason an [`ElementRef`] could not resolve in the browser.
///
/// Inspect this value inside a runtime expression with [`is_missing`](Self::is_missing)
/// or [`is_wrong_type`](Self::is_wrong_type).
#[derive(Debug)]
pub struct ElementRefError {
    browser_only: std::convert::Infallible,
}

impl ElementRefError {
    /// Whether no element in the document has the reference's ID.
    #[must_use]
    pub fn is_missing(&self) -> BoolSurrogate {
        match self.browser_only {}
    }

    /// Whether the element has the ID but not the expected native HTML type.
    #[must_use]
    pub fn is_wrong_type(&self) -> BoolSurrogate {
        match self.browser_only {}
    }
}

impl Surrogated for ElementRefError {
    type Surrogate = Self;

    fn into_surrogate(self) -> Self {
        self
    }
}

impl Surrogate for ElementRefError {
    type Real = Self;

    fn into_real(self) -> Self {
        self
    }
}
