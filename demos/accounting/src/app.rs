mod dashboard;
mod invoices;

use topcoat::{
    Result,
    asset::{AssetBundle, RouterBuilderAssetExt},
    context::Cx,
    font::{Font, fontsource::fontsource_font},
    router::{Router, RouterBuilderDiscoverExt, Slot, href, layout, module_router, page},
    runtime::{RouterBuilderRuntimeExt, signal},
    tailwind,
    view::{View, class, view},
};

const GEIST: Font = fontsource_font!(GEIST, host: Asset);

pub fn router(db: toasty::Db) -> Router {
    module_router!()
        .runtime()
        .discover()
        .assets(AssetBundle::load().unwrap())
        .app_context(db)
        .build()
}

#[layout]
async fn shell(cx: &Cx, slot: Slot<'_>) -> Result<impl View> {
    let menu_open = signal(cx, || false);
    let on_dashboard = href!(page).is_current(cx);
    Ok(view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8">
                <meta name="viewport" content="width=device-width, initial-scale=1">
                <title>"Ledger | Accounting"</title>
                topcoat::dev::script()
                topcoat::runtime::script()
                topcoat::font::link(font: GEIST)
                <link rel="stylesheet" href=(tailwind::stylesheet!())>
            </head>
            <body>
                <button
                    class="m-4 rounded-lg border px-4 py-2 md:hidden"
                    @click=$(|_e| menu_open.toggle())
                    :aria-expanded=$(menu_open.get())
                    aria-controls="sidebar"
                >
                    "Menu"
                </button>
                <aside id="sidebar" class="sidebar" :data-open=$(menu_open.get())>
                    <a
                        href=(href!(page))
                        class="flex items-center gap-2.5 px-2 py-2 text-base font-semibold tracking-tight"
                    >
                        <span
                            class="grid size-7 place-items-center rounded-md bg-primary text-sm text-primary-foreground"
                        >
                            "L"
                        </span>
                        "Ledger"
                    </a>
                    <div class="mt-8 px-3 text-xs font-medium text-muted-foreground">
                        "Workspace"
                    </div>
                    <nav aria-label="Main navigation" class="mt-2 flex flex-col gap-1">
                        <a
                            href=(href!(page))
                            aria-current=(on_dashboard.then_some("page"))
                            class=(class!("nav-link", "nav-active" if on_dashboard))
                        >
                            "Overview"
                        </a>
                        <a
                            href=(href!(invoices::page))
                            aria-current=((!on_dashboard).then_some("page"))
                            class=(class!("nav-link", "nav-active" if !on_dashboard))
                        >
                            "Invoices"
                        </a>
                    </nav>
                    <div class="mt-auto border-t border-border px-3 pt-4">
                        <p class="text-sm font-medium">"Studio Collective"</p>
                        <p class="mt-1 text-xs text-muted-foreground">
                            "Demo workspace / USD"
                        </p>
                    </div>
                </aside>
                <div class="md:ml-56">
                    <main class="mx-auto max-w-7xl px-5 py-8 lg:px-10 lg:py-10">
                        (slot)
                    </main>
                </div>
            </body>
        </html>
    })
}

#[page]
pub async fn page() -> Result<impl View> {
    Ok(view! { dashboard::overview() })
}
