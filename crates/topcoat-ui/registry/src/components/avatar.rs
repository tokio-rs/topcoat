use topcoat::{
    Result,
    view::{Attributes, View, class, component, view},
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
    ///
    /// Each size sets a text size along with the dimensions, which is what
    /// scales the initials in an [`avatar_fallback`].
    fn classes(self) -> &'static str {
        match self {
            Self::Sm => "size-8 text-xs",
            Self::Md => "size-10 text-sm",
            Self::Lg => "size-12 text-base",
        }
    }
}

/// The classes shared by every avatar, regardless of size.
///
/// The circle clips whatever is inside it, and is positioned so that an
/// [`avatar_image`] can cover the [`avatar_fallback`] behind it.
const AVATAR: &str = "relative flex shrink-0 overflow-hidden rounded-full";

/// An avatar component: a circular portrait of a person or an organization.
///
/// An avatar holds an [`avatar_image`], an [`avatar_fallback`], or both. With
/// both, the fallback shows until the image has loaded, and keeps showing if
/// the image never arrives. The `size` parameter selects the dimensions,
/// defaulting to `Md`. The `attrs` (such as `class` or `title`) are forwarded
/// to the underlying `<span>`; a `class` among them is appended to the
/// computed classes.
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
    child: View,
) -> Result {
    view! {
        <span class=(class!(AVATAR, size.classes(), attrs.remove("class"))) (attrs)>
            (child)
        </span>
    }
}

/// The portrait of an [`avatar`], covering the fallback behind it.
///
/// The image fills the circle and is cropped to it rather than squashed, so
/// portraits of any aspect ratio stay undistorted. Pass the `src` among the
/// `attrs`.
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
) -> Result {
    view! {
        <img
            alt=(alt)
            class=(class!(
                "absolute inset-0 size-full object-cover",
                attrs.remove("class"),
            ))
            (attrs)
        >
    }
}

/// What an [`avatar`] shows in place of its image: initials, or any small
/// view such as an icon.
///
/// It fills the circle and centers its content on a tinted background, and is
/// laid out behind the [`avatar_image`], so it stands in while the image
/// loads and whenever there is none.
#[component]
pub async fn avatar_fallback(#[default] mut attrs: Attributes, #[default] child: View) -> Result {
    view! {
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
    }
}
