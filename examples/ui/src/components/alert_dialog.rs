use topcoat::{
    Result,
    runtime::Expr,
    view::{Attributes, Child, View, attributes, component, view},
};

use super::dialog::dialog;

/// A dialog that asks the user to respond to an important message.
///
/// Build its content with the dialog components and provide actions for answering or
/// cancelling. It uses `role="alertdialog"` and has the same focus and dismissal
/// requirements as [`dialog`].
///
/// Pass `aria-labelledby` and `aria-describedby` in `attrs`, pointing to the title and
/// description IDs. Other attributes are forwarded to the `<dialog>`.
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
    #[into]
    open: Expr<bool>,
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
