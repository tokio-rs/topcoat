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
    /// Classes for the avatar dimensions and fallback text size.
    fn classes(self) -> StaticClass {
        match self {
            Self::Sm => class!("size-8 text-xs"),
            Self::Md => class!("size-10 text-sm"),
            Self::Lg => class!("size-12 text-base"),
        }
    }
}

/// Classes that clip the avatar to a circle and position its image over the fallback.
const AVATAR: StaticClass = class!("relative flex shrink-0 overflow-hidden rounded-full");

/// A circular image with optional fallback content.
///
/// Add an `avatar_image`, an `avatar_fallback`, or both. The fallback sits behind the
/// image and remains visible if the image cannot load. `size` defaults to `Md`.
///
/// `attrs` are forwarded to the outer `<span>`. Extra classes are added to its classes.
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

/// An image that fills the avatar and covers its fallback.
///
/// Pass `src` in `attrs`. The image is cropped to fit without changing its aspect
/// ratio.
#[component]
pub async fn avatar_image(
    /// Alternative text for the image.
    ///
    /// Defaults to empty text, suitable when an adjacent name already identifies the
    /// person. Empty text also prevents a failed image from drawing text over the
    /// fallback.
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

/// Content displayed behind the avatar image while it loads or when no image is
/// available.
///
/// Pass initials or another small view as children.
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
