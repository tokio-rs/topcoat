use topcoat::{
    Result,
    view::{Attributes, Child, StaticClass, View, class, component, view},
};

/// The size of an [`avatar`].
///
/// The default is `AvatarSize::Md`.
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
    ///
    /// Each size also sets a text size, which scales the initials in an
    /// [`avatar_fallback`].
    fn classes(self) -> StaticClass {
        match self {
            Self::Sm => class!("size-8 text-xs"),
            Self::Md => class!("size-10 text-sm"),
            Self::Lg => class!("size-12 text-base"),
        }
    }
}

/// The classes shared by every avatar, regardless of size.
///
/// The circle clips its content. It is positioned so that an
/// [`avatar_image`] can cover the [`avatar_fallback`] behind it.
const AVATAR: StaticClass = class!("relative flex shrink-0 overflow-hidden rounded-full");

/// A circular picture of a person or an organization.
///
/// An avatar holds an [`avatar_image`], an [`avatar_fallback`], or both. With
/// both, the fallback shows until the image has loaded, and stays visible if
/// the image fails to load. `size` sets the dimensions and defaults to `Md`.
/// The `attrs` (such as `class` or `title`) are forwarded to the `<span>`. A
/// `class` among them is appended to the component's classes.
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

/// The image of an [`avatar`], shown on top of the fallback.
///
/// The image fills the circle and is cropped, not stretched, so images of
/// any aspect ratio keep their proportions. Pass the `src` in `attrs`. The
/// `attrs` are forwarded to the `<img>`, and a `class` among them is appended
/// to the component's classes.
#[component]
pub async fn avatar_image(
    /// The image's alternative text.
    ///
    /// It is empty by default. This fits the usual case of an avatar next to
    /// the name it belongs to, where the image adds nothing to the name. Empty
    /// alternative text also keeps the [`avatar_fallback`] visible when the
    /// image fails to load. Otherwise the browser would draw the text over it.
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

/// What an [`avatar`] shows instead of its image, such as initials or an
/// icon.
///
/// It fills the circle and centers its content on a tinted background. It
/// sits behind the [`avatar_image`], so it is visible while the image loads
/// and when there is no image. Child nodes become its content. The `attrs`
/// are forwarded to the `<span>`, and a `class` among them is appended to the
/// component's classes.
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
