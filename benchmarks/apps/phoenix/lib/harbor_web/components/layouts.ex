defmodule HarborWeb.Layouts do
  @moduledoc """
  The root layout (the document shell in `layouts/root.html.heex`) and the app layout with the
  announcement bar, header, main content and footer that every page renders inside.
  """
  use HarborWeb, :html

  embed_templates "layouts/*"

  # The document title: `<page title> | Harbor Market`, or just the store name on the home page.
  # Error pages rendered by the endpoint for unknown routes only carry their status.
  defp page_title(%{page_title: title}), do: "#{title} | Harbor Market"
  defp page_title(%{status: status}), do: "#{HarborWeb.ErrorHTML.title(status)} | Harbor Market"
  defp page_title(_assigns), do: "Harbor Market"

  @doc """
  Renders the app layout around a page.

      <Layouts.app cart_count={@cart_count}>
        <h1>Content</h1>
      </Layouts.app>
  """
  attr :cart_count, :integer, required: true, doc: "the number of items in the cart cookie"
  slot :inner_block, required: true

  def app(assigns) do
    ~H"""
    <a
      href="#main"
      class="sr-only focus:not-sr-only focus:absolute focus:left-4 focus:top-4 focus:z-50 focus:rounded-md focus:bg-white focus:px-4 focus:py-2 focus:text-sm focus:font-medium focus:shadow"
    >
      Skip to content
    </a>
    <.site_header cart_count={@cart_count} />
    <main id="main" class="flex-1">
      {render_slot(@inner_block)}
    </main>
    <.site_footer />
    """
  end

  defp nav_links do
    [
      {~p"/products", "All products"},
      {listing_path("outdoor"), "Outdoor"},
      {listing_path("kitchen"), "Kitchen"},
      {listing_path("electronics"), "Electronics"},
      {~p"/about", "About"}
    ]
  end

  attr :cart_count, :integer, required: true

  defp site_header(assigns) do
    assigns = assign(assigns, :nav_links, nav_links())

    ~H"""
    <div class="bg-teal-900 px-4 py-2 text-center text-xs font-medium text-teal-50 sm:text-sm">
      Free shipping on orders over $75 and free 30-day returns
    </div>
    <header class="sticky top-0 z-20 border-b border-stone-200 bg-white/95 backdrop-blur">
      <div class="mx-auto flex max-w-7xl flex-wrap items-center gap-x-10 gap-y-3 px-4 py-3 sm:px-6 lg:px-8">
        <a
          href={~p"/"}
          class="flex items-center gap-2.5 text-lg font-semibold tracking-tight text-stone-900"
        >
          <Icons.logo />Harbor Market
        </a>
        <nav
          id="site-nav"
          aria-label="Main"
          class="order-last hidden w-full md:order-none md:block md:w-auto"
        >
          <ul class="flex flex-col gap-1 border-t border-stone-100 pt-3 md:flex-row md:gap-8 md:border-0 md:pt-0">
            <li :for={{href, label} <- @nav_links}>
              <a
                href={href}
                class="block rounded-md px-2 py-2 text-sm font-medium text-stone-600 hover:bg-stone-100 hover:text-stone-900 md:p-0 md:hover:bg-transparent"
              >
                {label}
              </a>
            </li>
          </ul>
        </nav>
        <div class="ml-auto flex items-center gap-2">
          <div class="flex items-center gap-2 rounded-full border border-stone-200 bg-white py-1.5 pl-3 pr-1.5 text-sm font-medium text-stone-700">
            <Icons.bag />
            <span class="hidden sm:inline">Cart</span>
            <span
              data-cart-count
              class="min-w-6 rounded-full bg-teal-700 px-2 py-0.5 text-center text-xs font-semibold text-white"
            >
              {HarborWeb.CartCount.badge(@cart_count)}
            </span>
          </div>
          <button
            type="button"
            data-menu-toggle
            aria-controls="site-nav"
            aria-expanded="false"
            class="rounded-md p-2 text-stone-600 hover:bg-stone-100 md:hidden"
          >
            <span class="sr-only">Toggle menu</span>
            <Icons.menu />
          </button>
        </div>
      </div>
    </header>
    """
  end

  defp footer_columns do
    [
      {"Shop",
       [
         {~p"/products", "All products"},
         {listing_path("outdoor"), "Outdoor & Camping"},
         {listing_path("kitchen"), "Kitchen & Dining"},
         {listing_path("audio"), "Audio"},
         {listing_path("toys"), "Toys & Games"}
       ]},
      {"Help",
       [
         {~p"/about#shipping", "Shipping & returns"},
         {~p"/about#warranty", "Warranty & repairs"},
         {~p"/about#contact", "Contact us"}
       ]},
      {"Company",
       [
         {~p"/about", "About us"},
         {~p"/admin/orders", "Order admin"},
         {~p"/api/products", "Product API"}
       ]}
    ]
  end

  defp site_footer(assigns) do
    assigns = assign(assigns, :columns, footer_columns())

    ~H"""
    <footer class="mt-24 bg-stone-900 text-stone-400">
      <div class="mx-auto max-w-7xl px-4 py-16 sm:px-6 lg:px-8">
        <div class="grid gap-12 md:grid-cols-5">
          <div class="md:col-span-2">
            <a
              href={~p"/"}
              class="flex items-center gap-2.5 text-lg font-semibold tracking-tight text-white"
            >
              <Icons.logo />Harbor Market
            </a>
            <p class="mt-4 max-w-xs text-sm leading-6">
              Well-made gear for every harbor day. Shipped from Portland, Oregon since 2016.
            </p>
          </div>
          <div :for={{title, links} <- @columns}>
            <h2 class="text-sm font-semibold text-white">{title}</h2>
            <ul class="mt-4 space-y-3 text-sm">
              <li :for={{href, label} <- links}>
                <a href={href} class="hover:text-white">{label}</a>
              </li>
            </ul>
          </div>
        </div>
        <div class="mt-12 flex flex-col gap-2 border-t border-stone-800 pt-8 text-xs sm:flex-row sm:justify-between">
          <p>Copyright 2026 Harbor Market. All rights reserved.</p>
          <p>A demo storefront for comparing web frameworks.</p>
        </div>
      </div>
    </footer>
    """
  end
end
