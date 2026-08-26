use topcoat::{
    Result,
    view::{Attributes, Child, View, attributes, component, view},
};

use super::dialog::dialog;

/// An alert dialog component: a dialog interrupting the page for an answer it
/// will not go on without.
///
/// It is the [`dialog`] with the role that says so, which is what has
/// assistive technology announce it as a question rather than as another
/// panel. Everything else is the dialog's: build the inside out of
/// [`dialog_content`](super::dialog::dialog_content),
/// [`dialog_header`](super::dialog::dialog_header) and the rest, and give the
/// footer the choice to make. Leave out any way of closing it that is not one
/// of the answers, since a reader who dismisses the question is left where
/// they started.
///
/// Naming the panel for assistive technology takes an `aria-labelledby` among
/// the `attrs` pointing at the title, and an `aria-describedby` pointing at
/// the description. The `attrs` are otherwise forwarded to the underlying
/// `<dialog>`.
///
/// ```ignore
/// view! {
///     alert_dialog(
///         open: confirming,
///         dialog_content(
///             dialog_header(
///                 dialog_title("Delete this workspace?")
///                 dialog_description("Its projects and deploys go with it.")
///             )
///             dialog_footer(
///                 <a href="/workspace" class=(button_variants(
///                     ButtonVariant::Ghost,
///                     ButtonSize::Md,
///                 ))>"Keep it"</a>
///                 button(variant: ButtonVariant::Destructive, "Delete")
///             )
///         )
///     )
/// }
/// ```
#[component]
pub async fn alert_dialog(
    /// Whether the alert dialog shows.
    open: bool,
    /// Extra attributes for the `<dialog>` element.
    #[default]
    attrs: Attributes,
    /// The alert dialog's content.
    #[default]
    child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        dialog(open: open, attrs: attributes! { role="alertdialog" (attrs) }, (child))
    })
}
