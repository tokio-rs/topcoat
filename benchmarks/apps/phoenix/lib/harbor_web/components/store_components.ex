defmodule HarborWeb.StoreComponents do
  @moduledoc """
  Function components shared between the storefront pages: product cards and tiles, star rows,
  breadcrumbs, pagination and status badges, plus the listing link builder.

  Class strings chosen by data are literal strings in the tables below so Tailwind finds them.
  """
  use Phoenix.Component

  use Phoenix.VerifiedRoutes,
    endpoint: HarborWeb.Endpoint,
    router: HarborWeb.Router,
    statics: HarborWeb.static_paths()

  import HarborWeb.Format

  alias Harbor.Catalog
  alias Harbor.Catalog.Product
  alias HarborWeb.Icons

  ## Links

  @doc """
  The path of a listing page. Parameters at their default (no category, `:newest`, page 1) are
  left out; the rest come in the order `category`, `sort`, `page`.
  """
  def listing_path(category_slug, sort \\ :newest, page \\ 1) do
    query =
      Enum.reject(
        [
          category: category_slug,
          sort: sort != :newest && Catalog.sort_param(sort),
          page: page != 1 && page
        ],
        fn {_key, value} -> value in [nil, false] end
      )

    ~p"/products?#{query}"
  end

  ## Colors

  @palette %{
    "charcoal" => {"Charcoal", "bg-stone-200", "bg-stone-700"},
    "sand" => {"Sand", "bg-amber-100", "bg-amber-300"},
    "sage" => {"Sage", "bg-green-100", "bg-green-700/70"},
    "navy" => {"Navy", "bg-blue-100", "bg-blue-900"},
    "terracotta" => {"Terracotta", "bg-orange-100", "bg-orange-700"},
    "ochre" => {"Ochre", "bg-yellow-100", "bg-yellow-600"},
    "plum" => {"Plum", "bg-purple-100", "bg-purple-800"},
    "sky" => {"Sky", "bg-sky-100", "bg-sky-500"}
  }

  @doc "The display name of a product color."
  def color_label(color), do: @palette |> Map.fetch!(color) |> elem(0)

  @doc "The background class of a product's image tile."
  def color_tile(color), do: @palette |> Map.fetch!(color) |> elem(1)

  @doc "The fill class of a product's shape and color swatch."
  def color_accent(color), do: @palette |> Map.fetch!(color) |> elem(2)

  ## Product tile and card

  @doc """
  The square product tile with the sale and sold-out badges.

  The dataset has no photos, so the tile draws a soft shape in the product's color instead.
  """
  attr :product, Product, required: true

  def product_image(assigns) do
    ~H"""
    <div class={"relative aspect-square overflow-hidden rounded-2xl " <> color_tile(@product.color)}>
      <div class={"absolute left-1/2 top-1/2 aspect-square w-2/5 -translate-x-1/2 -translate-y-1/2 rotate-6 rounded-[28%] shadow-xl transition group-hover:rotate-12 " <> color_accent(@product.color)}>
      </div>
      <div
        :if={@product.stock == 0 or @product.compare_at_cents}
        class="absolute left-3 top-3 flex gap-1.5"
      >
        <span
          :if={@product.stock == 0}
          class="rounded-full bg-stone-900 px-2.5 py-1 text-xs font-medium text-white"
        >
          Sold out
        </span>
        <span
          :if={@product.compare_at_cents}
          class="rounded-full bg-rose-600 px-2.5 py-1 text-xs font-medium text-white"
        >
          Sale
        </span>
      </div>
    </div>
    """
  end

  @doc "A product in a grid: tile, category, name, tagline, rating, price and color."
  attr :product, Product, required: true

  def product_card(assigns) do
    assigns = assign(assigns, :category, Catalog.category_of(assigns.product))

    ~H"""
    <article class="group relative flex flex-col">
      <.product_image product={@product} />
      <div class="mt-4 flex flex-1 flex-col">
        <p class="text-xs font-medium uppercase tracking-wide text-stone-500">{@category.name}</p>
        <h3 class="mt-1 text-sm font-semibold text-stone-900 group-hover:text-teal-800">
          <a href={~p"/products/#{@product.id}"}>
            <span class="absolute inset-0"></span>{@product.name}
          </a>
        </h3>
        <p class="mt-1 line-clamp-2 text-sm text-stone-600">{@product.tagline}</p>
        <div class="mt-2 flex items-center gap-1.5">
          <.stars
            filled={filled_stars(Product.rating_tenths(@product))}
            rating={rating(@product.rating)}
            size={:small}
          />
          <span class="text-xs text-stone-500">
            {rating(@product.rating)} ({count(@product.review_count)})
          </span>
        </div>
        <div class="mt-auto flex items-end justify-between gap-2 pt-3">
          <p class="flex items-baseline gap-2">
            <span class="font-semibold text-stone-900">{money(@product.price_cents)}</span>
            <s :if={@product.compare_at_cents} class="text-sm text-stone-400">
              {money(@product.compare_at_cents)}
            </s>
          </p>
          <span class="flex items-center gap-1.5 text-xs text-stone-500">
            <span class={"size-3 rounded-full ring-1 ring-black/10 " <> color_accent(@product.color)}></span>
            {color_label(@product.color)}
          </span>
        </div>
        <p :if={@product.stock in 1..9} class="mt-1 text-xs font-medium text-amber-700">
          Only {@product.stock} left
        </p>
      </div>
    </article>
    """
  end

  ## Stars

  @doc "The number of filled stars for a rating in tenths: the rating rounded half up."
  def filled_stars(rating_tenths), do: div(rating_tenths + 5, 10)

  @doc """
  A row of five stars with `filled` of them highlighted. The row is a single image to assistive
  technology, labelled with `rating`.
  """
  attr :filled, :integer, required: true
  attr :rating, :any, required: true
  attr :size, :atom, values: [:small, :large], required: true

  def stars(assigns) do
    ~H"""
    <span class="flex items-center gap-0.5" role="img" aria-label={"Rated #{@rating} out of 5"}>
      <svg :for={star <- 0..4} viewBox="0 0 20 20" class={star_class(@size, star < @filled)}>
        <path d="M10 1l3 6 6 1-4 4 1 6-6-3-6 3 1-6-4-4 6-1z"></path>
      </svg>
    </span>
    """
  end

  defp star_class(:small, true), do: "size-4 fill-amber-400"
  defp star_class(:small, false), do: "size-4 fill-stone-300"
  defp star_class(:large, true), do: "size-5 fill-amber-400"
  defp star_class(:large, false), do: "size-5 fill-stone-300"

  ## Navigation

  @doc "A breadcrumb trail of `{label, href}` links ending in the current page."
  attr :trail, :list, required: true
  attr :current, :string, required: true

  def breadcrumbs(assigns) do
    ~H"""
    <nav aria-label="Breadcrumb" class="text-sm">
      <ol class="flex flex-wrap items-center gap-1.5 text-stone-500">
        <li :for={{label, href} <- @trail} class="flex items-center gap-1.5">
          <a href={href} class="hover:text-stone-900">{label}</a>
          <Icons.chevron_right />
        </li>
        <li>
          <span aria-current="page" class="font-medium text-stone-900">{@current}</span>
        </li>
      </ol>
    </nav>
    """
  end

  @step "flex items-center gap-2 rounded-full px-3 py-2 text-sm font-medium text-stone-700 hover:bg-stone-100"
  @step_disabled "flex items-center gap-2 rounded-full px-3 py-2 text-sm font-medium text-stone-300"
  @page "flex size-10 items-center justify-center rounded-full text-sm font-medium text-stone-600 hover:bg-stone-100"
  @page_current "flex size-10 items-center justify-center rounded-full bg-stone-900 text-sm font-semibold text-white"

  @doc "Previous and next links with a link to every page of the listing."
  attr :listing, Harbor.Catalog.Listing, required: true

  def pagination(assigns) do
    assigns =
      assign(assigns,
        slug: assigns.listing.category && assigns.listing.category.slug,
        step: @step,
        step_disabled: @step_disabled,
        page_class: @page,
        page_current: @page_current
      )

    ~H"""
    <nav
      aria-label="Pagination"
      class="mt-12 flex items-center justify-between gap-4 border-t border-stone-200 pt-6"
    >
      <%= if @listing.page > 1 do %>
        <a href={listing_path(@slug, @listing.sort, @listing.page - 1)} class={@step}>
          <Icons.arrow_left />Previous
        </a>
      <% else %>
        <span aria-disabled="true" class={@step_disabled}><Icons.arrow_left />Previous</span>
      <% end %>
      <p class="text-sm text-stone-600 sm:hidden">
        Page {@listing.page} of {@listing.page_count}
      </p>
      <ul class="hidden items-center gap-1 sm:flex">
        <li :for={page <- 1..@listing.page_count}>
          <%= if page == @listing.page do %>
            <a
              href={listing_path(@slug, @listing.sort, page)}
              aria-current="page"
              class={@page_current}
            >
              {page}
            </a>
          <% else %>
            <a href={listing_path(@slug, @listing.sort, page)} class={@page_class}>{page}</a>
          <% end %>
        </li>
      </ul>
      <%= if @listing.page < @listing.page_count do %>
        <a href={listing_path(@slug, @listing.sort, @listing.page + 1)} class={@step}>
          Next<Icons.arrow_right />
        </a>
      <% else %>
        <span aria-disabled="true" class={@step_disabled}>Next<Icons.arrow_right /></span>
      <% end %>
    </nav>
    """
  end

  ## Orders

  @status_badges %{
    "pending" => {"Pending", "bg-amber-50 text-amber-800 ring-amber-600/20"},
    "paid" => {"Paid", "bg-sky-50 text-sky-800 ring-sky-600/20"},
    "shipped" => {"Shipped", "bg-indigo-50 text-indigo-700 ring-indigo-600/20"},
    "delivered" => {"Delivered", "bg-emerald-50 text-emerald-800 ring-emerald-600/20"},
    "refunded" => {"Refunded", "bg-stone-100 text-stone-700 ring-stone-500/20"},
    "cancelled" => {"Cancelled", "bg-rose-50 text-rose-700 ring-rose-600/20"}
  }

  @doc "A colored pill showing an order's status."
  attr :status, :string, required: true

  def status_badge(assigns) do
    {label, colors} = Map.fetch!(@status_badges, assigns.status)
    assigns = assign(assigns, label: label, colors: colors)

    ~H"""
    <span class={"inline-flex rounded-full px-2 py-0.5 text-xs font-medium ring-1 ring-inset " <> @colors}>
      {@label}
    </span>
    """
  end
end
