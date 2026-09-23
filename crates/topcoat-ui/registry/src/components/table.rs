use topcoat::{
    Result,
    view::{Attributes, Child, View, class, component, view},
};

/// A table of rows and columns.
///
/// The table is wrapped in a container that scrolls horizontally, so a wide
/// table scrolls instead of stretching the page. Child nodes become the
/// table's sections: a [`table_header`], a [`table_body`], and optionally a
/// [`table_footer`] and a [`table_caption`].
///
/// The `attrs` (such as `class`) are forwarded to the `<table>`, not to the
/// wrapping `<div>`. A `class` among them is appended to the component's
/// classes. The other table components also forward their `attrs` to their
/// element and append a `class`.
///
/// ```ignore
/// view! {
///     table(
///         table_header(
///             table_row(
///                 table_head("Environment")
///                 table_head("Status")
///             )
///         )
///         table_body(
///             table_row(
///                 table_cell("production")
///                 table_cell("Live")
///             )
///         )
///     )
/// }
/// ```
#[component]
pub async fn table(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <div class="w-full overflow-x-auto">
            <table
                class=(class!(
                    "w-full caption-bottom border-collapse text-sm",
                    attrs.remove("class"),
                ))
                (attrs)
            >
                (child)
            </table>
        </div>
    })
}

/// The header section of a [`table`], rendered as a `<thead>` that holds the
/// row of column headers.
#[component]
pub async fn table_header(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <thead class=(class!("[&_tr]:border-b", attrs.remove("class"))) (attrs)>
            (child)
        </thead>
    })
}

/// The main section of a [`table`], rendered as a `<tbody>` that holds the
/// data rows.
///
/// The last row has no bottom border.
#[component]
pub async fn table_body(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <tbody
            class=(class!("[&_tr:last-child]:border-0", attrs.remove("class")))
            (attrs)
        >
            (child)
        </tbody>
    })
}

/// The footer section of a [`table`], rendered as a `<tfoot>`, for totals and
/// other summaries.
#[component]
pub async fn table_footer(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <tfoot
            class=(class!(
                "border-t border-border bg-foreground/5 font-medium [&>tr]:last:border-b-0",
                attrs.remove("class"),
            ))
            (attrs)
        >
            (child)
        </tfoot>
    })
}

/// A row of a [`table`], in any of its sections.
///
/// Rows have a bottom border and are tinted on hover, which makes them easier
/// to follow across wide tables.
#[component]
pub async fn table_row(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <tr
            class=(class!(
                "border-b border-border transition-colors hover:bg-foreground/5",
                attrs.remove("class"),
            ))
            (attrs)
        >
            (child)
        </tr>
    })
}

/// A column header in the row of a [`table_header`], rendered as a `<th>`.
#[component]
pub async fn table_head(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <th
            class=(class!(
                "h-10 px-3 text-left align-middle font-medium whitespace-nowrap \
                 text-muted-foreground",
                attrs.remove("class"),
            ))
            (attrs)
        >
            (child)
        </th>
    })
}

/// A cell of a [`table_row`], rendered as a `<td>`.
#[component]
pub async fn table_cell(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <td
            class=(class!("p-3 align-middle whitespace-nowrap", attrs.remove("class")))
            (attrs)
        >
            (child)
        </td>
    })
}

/// A caption shown below a [`table`] that says what it contains.
#[component]
pub async fn table_caption(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <caption
            class=(class!("mt-4 text-sm text-muted-foreground", attrs.remove("class")))
            (attrs)
        >
            (child)
        </caption>
    })
}
