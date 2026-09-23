use topcoat::{
    Result,
    runtime::Expr,
    view::{Attributes, Child, View, attributes, component, view},
};

use super::dialog::dialog;

/// A dialog that asks the user to make a decision before going on.
///
/// It is a [`dialog`] with `role="alertdialog"`, so assistive technology
/// announces it as a question. Build its content from
/// [`dialog_content`](super::dialog::dialog_content),
/// [`dialog_header`](super::dialog::dialog_header), and the other dialog
/// components, and put the choices in the footer. Do not offer a way to close
/// it other than the choices.
///
/// To give the dialog an accessible name, pass `aria-labelledby` with the id
/// of the title and `aria-describedby` with the id of the description in
/// `attrs`. The `attrs` are forwarded to the `<dialog>`.
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
