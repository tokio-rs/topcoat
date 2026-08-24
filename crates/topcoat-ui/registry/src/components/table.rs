use topcoat::{
    Result,
    view::{Attributes, View, class, component, view},
};

/// A table component: rows and columns of related data.
///
/// The table is wrapped in a scrolling container, so a table wider than its
/// surroundings scrolls sideways on its own instead of stretching the page.
/// Child nodes are the table's sections: a [`table_header`], a
/// [`table_body`], and optionally a [`table_footer`] and a [`table_caption`].
/// The `attrs` (such as `class`) are forwarded to the `<table>` itself; a
/// `class` among them is appended to the computed classes.
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
pub async fn table(#[default] mut attrs: Attributes, #[default] child: View) -> Result {
    view! {
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
    }
}

/// The heading section of a [`table`], holding the row of column headers.
#[component]
pub async fn table_header(#[default] mut attrs: Attributes, #[default] child: View) -> Result {
    view! {
        <thead class=(class!("[&_tr]:border-b", attrs.remove("class"))) (attrs)>
            (child)
        </thead>
    }
}

/// The main section of a [`table`], holding its rows of data.
///
/// The last row's rule is dropped, so the table ends on its own edge rather
/// than on a line.
#[component]
pub async fn table_body(#[default] mut attrs: Attributes, #[default] child: View) -> Result {
    view! {
        <tbody
            class=(class!("[&_tr:last-child]:border-0", attrs.remove("class")))
            (attrs)
        >
            (child)
        </tbody>
    }
}

/// The closing section of a [`table`], for totals and other summaries.
#[component]
pub async fn table_footer(#[default] mut attrs: Attributes, #[default] child: View) -> Result {
    view! {
        <tfoot
            class=(class!(
                "border-t border-border bg-foreground/5 font-medium [&>tr]:last:border-b-0",
                attrs.remove("class"),
            ))
            (attrs)
        >
            (child)
        </tfoot>
    }
}

/// One row of a [`table`], in any of its sections.
///
/// The row is ruled off from the next one and tints on hover, so the eye can
/// follow it across wide tables.
#[component]
pub async fn table_row(#[default] mut attrs: Attributes, #[default] child: View) -> Result {
    view! {
        <tr
            class=(class!(
                "border-b border-border transition-colors hover:bg-foreground/5",
                attrs.remove("class"),
            ))
            (attrs)
        >
            (child)
        </tr>
    }
}

/// A column header in a [`table_header`]'s row.
#[component]
pub async fn table_head(#[default] mut attrs: Attributes, #[default] child: View) -> Result {
    view! {
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
    }
}

/// One cell of a [`table_row`].
#[component]
pub async fn table_cell(#[default] mut attrs: Attributes, #[default] child: View) -> Result {
    view! {
        <td
            class=(class!("p-3 align-middle whitespace-nowrap", attrs.remove("class")))
            (attrs)
        >
            (child)
        </td>
    }
}

/// A line under a [`table`] saying what it holds.
#[component]
pub async fn table_caption(#[default] mut attrs: Attributes, #[default] child: View) -> Result {
    view! {
        <caption
            class=(class!("mt-4 text-sm text-muted-foreground", attrs.remove("class")))
            (attrs)
        >
            (child)
        </caption>
    }
}
