use topcoat::{
    Result,
    runtime::Expr,
    view::{Attributes, Child, View, attributes, component, view},
};

use super::dialog::dialog;

/// A dialog that asks the user to make a decision.
///
/// Build its content with the dialog components and provide explicit actions.
/// Set `aria-labelledby` and `aria-describedby` in `attrs` to the IDs of its
/// title and description. Attributes are forwarded to the `<dialog>`.
///
/// Like [`dialog`], it needs application scripting for focus trapping.
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
