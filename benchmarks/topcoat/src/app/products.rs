mod id;

use topcoat::{
    Result,
    context::{Cx, app_context},
    router::{page, query_params},
    view::{class, view},
};

use crate::{
    app::_components::{pagination, product_card},
    catalog::Catalog,
    urls::{normalize_sort, products_url},
};

const SORT_OPTIONS: [(Option<&str>, &str); 5] = [
    (None, "Default"),
    (Some("name"), "Name"),
    (Some("price"), "Price: Low to high"),
    (Some("price-desc"), "Price: High to low"),
    (Some("rating"), "Rating"),
];

const CHIP_ACTIVE: &str = "rounded-full bg-slate-900 px-3 py-1 font-medium text-white";
const CHIP_INACTIVE: &str =
    "rounded-full bg-white px-3 py-1 font-medium text-slate-600 shadow-sm hover:bg-slate-100";

#[query_params(error = bad_request)]
struct ProductsQuery {
    page: Option<usize>,
    sort: Option<String>,
    category: Option<String>,
}

#[page]
async fn products(cx: &Cx) -> Result {
    let query = query_params::<ProductsQuery>(cx)?;
    let catalog = app_context::<Catalog>(cx);
    let sort = normalize_sort(query.sort.as_deref());
    let category = query.category.as_deref();
    let page = catalog.page(query.page.unwrap_or(1), sort, category);

    view! {
        <div class="flex flex-wrap items-baseline justify-between gap-4">
            <h1 class="text-3xl font-bold tracking-tight">"All products"</h1>
            <p class="text-sm text-slate-500">
                (page.total)
                " products"
            </p>
        </div>
        <div class="mt-6 flex flex-wrap items-center gap-2 text-sm">
            <span class="font-medium text-slate-500">"Sort:"</span>
            for (value, label) in SORT_OPTIONS {
                <a
                    href=(products_url(1, value, category))
                    class=(class!(CHIP_ACTIVE if value == sort else CHIP_INACTIVE))
                >
                    (label)
                </a>
            }
        </div>
        <div class="mt-3 flex flex-wrap items-center gap-2 text-sm">
            <span class="font-medium text-slate-500">"Category:"</span>
            <a
                href=(products_url(1, sort, None))
                class=(class!(CHIP_ACTIVE if category.is_none() else CHIP_INACTIVE))
            >
                "All"
            </a>
            for entry in catalog.categories() {
                <a
                    href=(products_url(1, sort, Some(&entry.slug)))
                    class=(class!(
                        CHIP_ACTIVE if category == Some(entry.slug.as_str()) else CHIP_INACTIVE,
                    ))
                >
                    (&entry.name)
                </a>
            }
        </div>
        if page.total == 0 {
            <p class="mt-8 text-slate-500">"No products found."</p>
        } else {
            <div class="mt-8 grid grid-cols-2 gap-6 md:grid-cols-3 lg:grid-cols-4">
                for product in &page.items {
                    product_card(product: product)
                }
            </div>
            pagination(
                current: page.current,
                page_count: page.page_count,
                sort: sort,
                category: category
            )
        }
    }
}
