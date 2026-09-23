use topcoat::{
    Result,
    view::{Attributes, Child, StaticClass, View, class, component, view},
};

/// The size of an [`avatar`].
///
/// [`Default`] is `AvatarSize::Md`, used when no size is given.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(dead_code)]
pub enum AvatarSize {
    /// A compact avatar for dense lists.
    Sm,
    /// The standard avatar size.
    #[default]
    Md,
    /// A prominent avatar for profile headers.
    Lg,
}

impl AvatarSize {
    /// The Tailwind classes for this size.
    fn classes(self) -> StaticClass {
        match self {
            Self::Sm => class!("size-8 text-xs"),
            Self::Md => class!("size-10 text-sm"),
            Self::Lg => class!("size-12 text-base"),
        }
    }
}

/// The classes shared by every avatar, regardless of size.
const AVATAR: StaticClass = class!("relative flex shrink-0 overflow-hidden rounded-full");

/// A circular portrait with an optional fallback.
///
/// Place an image over fallback content to show the fallback while the image
/// loads or if loading fails. `size` defaults to `Md`. Attributes are forwarded
/// to the `<span>`, and classes are appended.
///
/// ```ignore
/// view! {
///     avatar(
///         avatar_image(attrs: attributes! { src="/avatars/ada.jpg" })
///         avatar_fallback("AL")
///     )
/// }
/// ```
#[component]
pub async fn avatar(
    /// The dimensions of the circle.
    #[default]
    size: AvatarSize,
    /// Extra attributes for the `<span>` element.
    #[default]
    mut attrs: Attributes,
    /// The avatar's image, fallback, or both.
    #[default]
    child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <span class=(class!(AVATAR, size.classes(), attrs.remove("class"))) (attrs)>
            (child)
        </span>
    })
}

/// An avatar image that fills the circle without stretching.
///
/// Pass its `src` through `attrs`.
#[component]
pub async fn avatar_image(
    /// The image's alternative text.
    ///
    /// It is empty by default, which suits the usual case: an avatar beside
    /// the name it belongs to says nothing the name does not, and empty
    /// alternative text is also what leaves the [`avatar_fallback`] visible
    /// when the image fails to load, where a caption would be painted over it
    /// instead.
    #[into]
    #[default]
    alt: String,
    /// Extra attributes for the `<img>` element.
    #[default]
    mut attrs: Attributes,
) -> Result<impl View> {
    Ok(view! {
        <img
            alt=(alt)
            class=(class!(
                "absolute inset-0 size-full object-cover",
                attrs.remove("class"),
            ))
            (attrs)
        >
    })
}

/// Fallback content shown while the avatar image is unavailable.
///
/// Use initials or an icon. The content is centered behind the image.
#[component]
pub async fn avatar_fallback(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <span
            class=(class!(
                "flex size-full items-center justify-center bg-foreground/10 font-medium \
                 text-foreground select-none",
                attrs.remove("class"),
            ))
            (attrs)
        >
            (child)
        </span>
    })
}
