use topcoat::{
    Result,
    context::Cx,
    runtime::{Event, Signal},
    view::{View, attributes, class, component, view},
};

use super::{
    pagination::{
        pagination, pagination_content, pagination_ellipsis, pagination_item, pagination_link,
        pagination_next, pagination_previous,
    },
    t,
};

/// Item counts and page boundaries for a paginated list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ListPage {
    pub count: usize,
    /// Current page, starting at 1.
    pub number: usize,
    per_page: usize,
}

impl ListPage {
    /// Clamps `number` to the available pages. Empty lists have one page.
    ///
    /// # Panics
    ///
    /// Panics when `per_page` is zero.
    pub fn new(count: usize, per_page: usize, number: usize) -> Self {
        assert!(per_page > 0, "a page holds at least one item");
        let pages = count.div_ceil(per_page).max(1);
        Self {
            count,
            number: number.clamp(1, pages),
            per_page,
        }
    }

    pub fn pages(self) -> usize {
        self.count.div_ceil(self.per_page).max(1)
    }

    /// Position of the first item, counting from 1.
    pub fn first(self) -> usize {
        (self.number - 1) * self.per_page + 1
    }

    /// Position of the last item, counting from 1. Returns 0 for an empty list.
    pub fn last(self) -> usize {
        (self.number * self.per_page).min(self.count)
    }

    pub fn items(self) -> usize {
        self.last() + 1 - self.first()
    }

    /// Page links with `None` for gaps. Always includes both ends and the current
    /// page's neighbors. A single missing page gets a link instead of an ellipsis.
    fn window(self) -> Vec<Option<usize>> {
        let pages = self.pages();
        let shown =
            |number: usize| number == 1 || number == pages || number.abs_diff(self.number) <= 1;
        let mut window = Vec::new();
        for number in 1..=pages {
            if shown(number) || (shown(number - 1) && shown(number + 1)) {
                window.push(Some(number));
            } else if window.last() != Some(&None) {
                window.push(None);
            }
        }
        window
    }
}

/// Shows the item range and page links. Clicks update the `number` signal in place.
/// Each link also gets a URL from `href`, so it can be opened in a new tab.
#[component]
pub async fn list_pagination(
    cx: &Cx,
    page: ListPage,
    href: impl Fn(usize) -> String + Send + Sync,
    number: &Signal<usize>,
) -> Result<impl View> {
    let current = page.number;
    let pages = page.pages();
    let previous = current.saturating_sub(1).max(1);
    let next = (current + 1).min(pages);

    Ok(view! {
        <div class="flex flex-col items-center gap-3 px-6 py-3 sm:flex-row">
            <p class="shrink-0 text-sm text-muted-foreground tabular-nums">
                (t!(
                    pagination_summary,
                    first: page.first(),
                    last: page.last(),
                    count: page.count,
                ))
            </p>
            // Offset the Next link's padding to align its arrow with the table content.
            pagination(
                attrs: attributes! { class="sm:-mr-3 sm:justify-end" },
                pagination_content(
                    pagination_item(
                        pagination_previous(
                            label: t!(pagination_previous).resolve(cx).into_owned(),
                            attrs: attributes! {
                                href=(href(previous))
                                aria-disabled=((current == 1).then_some("true"))
                                class=(class!("pointer-events-none opacity-50" if current == 1))
                                @click=$(|e: Event| {
                                    e.prevent_default();
                                    number.set(previous);
                                })
                            }
                        )
                    )
                    for shown in page.window() {
                        pagination_item(
                            attrs: attributes! { class="hidden sm:block" },
                            if let Some(shown) = shown {
                                pagination_link(
                                    active: shown == current,
                                    attrs: attributes! {
                                        href=(href(shown))
                                        @click=$(|e: Event| {
                                            e.prevent_default();
                                            number.set(shown);
                                        })
                                    },
                                    (shown)
                                )
                            } else {
                                pagination_ellipsis()
                            }
                        )
                    }
                    pagination_item(
                        pagination_next(
                            label: t!(pagination_next).resolve(cx).into_owned(),
                            attrs: attributes! {
                                href=(href(next))
                                aria-disabled=((current == pages).then_some("true"))
                                class=(class!("pointer-events-none opacity-50" if current == pages))
                                @click=$(|e: Event| {
                                    e.prevent_default();
                                    number.set(next);
                                })
                            }
                        )
                    )
                )
            )
        </div>
    })
}

#[cfg(test)]
mod tests {
    use super::ListPage;

    #[test]
    fn a_page_shows_its_share_of_the_items() {
        let page = ListPage::new(45, 20, 2);
        assert_eq!((page.first(), page.last(), page.items()), (21, 40, 20));

        let last = ListPage::new(45, 20, 3);
        assert_eq!((last.first(), last.last(), last.items()), (41, 45, 5));
    }

    #[test]
    fn a_page_out_of_range_gives_the_nearest_page() {
        assert_eq!(ListPage::new(45, 20, 0).number, 1);
        assert_eq!(ListPage::new(45, 20, 9).number, 3);
    }

    #[test]
    fn an_empty_list_has_one_empty_page() {
        let page = ListPage::new(0, 20, 4);
        assert_eq!((page.number, page.pages(), page.items()), (1, 1, 0));
    }

    #[test]
    fn the_window_keeps_both_ends_and_the_neighbors() {
        let page = ListPage::new(200, 20, 5);
        let window = page.window();
        assert_eq!(
            window,
            [Some(1), None, Some(4), Some(5), Some(6), None, Some(10)]
        );
    }

    #[test]
    fn the_window_never_hides_a_single_page() {
        let window = ListPage::new(200, 20, 4).window();
        assert_eq!(window[..4], [Some(1), Some(2), Some(3), Some(4)]);
    }
}
