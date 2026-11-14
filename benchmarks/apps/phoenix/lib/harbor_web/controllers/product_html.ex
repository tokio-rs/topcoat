defmodule HarborWeb.ProductHTML do
  @moduledoc """
  This module contains the listing and detail pages rendered by ProductController.

  See the `product_html` directory for all templates available.
  """
  use HarborWeb, :html

  alias Harbor.Catalog
  alias Harbor.Catalog.Product

  embed_templates "product_html/*"

  @sidebar_link "flex items-center justify-between gap-3 whitespace-nowrap rounded-lg px-3 py-2 text-sm font-medium text-stone-600 hover:bg-stone-100 hover:text-stone-900"
  @sidebar_link_current "flex items-center justify-between gap-3 whitespace-nowrap rounded-lg bg-teal-50 px-3 py-2 text-sm font-semibold text-teal-900"
  @sort_link "block rounded-full bg-white px-3 py-1.5 text-sm font-medium text-stone-600 ring-1 ring-inset ring-stone-200 hover:bg-stone-100"
  @sort_link_current "block rounded-full bg-stone-900 px-3 py-1.5 text-sm font-medium text-white ring-1 ring-inset ring-stone-900"

  @doc "A category entry in the listing sidebar: the name and its product count."
  attr :href, :string, required: true
  attr :current, :boolean, required: true
  attr :count, :integer, required: true
  slot :inner_block, required: true

  def sidebar_link(assigns) do
    ~H"""
    <%= if @current do %>
      <a href={@href} aria-current="page" class={sidebar_link_class(true)}>
        {render_slot(@inner_block)}<span class="text-xs text-teal-700">{count(@count)}</span>
      </a>
    <% else %>
      <a href={@href} class={sidebar_link_class(false)}>
        {render_slot(@inner_block)}<span class="text-xs text-stone-400">{count(@count)}</span>
      </a>
    <% end %>
    """
  end

  defp sidebar_link_class(true), do: @sidebar_link_current
  defp sidebar_link_class(false), do: @sidebar_link

  @doc "A sort order link above the product grid."
  attr :href, :string, required: true
  attr :current, :boolean, required: true
  attr :sort, :atom, required: true

  def sort_link(assigns) do
    ~H"""
    <%= if @current do %>
      <a href={@href} aria-current="true" class={sort_link_class(true)}>
        {Catalog.sort_label(@sort)}
      </a>
    <% else %>
      <a href={@href} class={sort_link_class(false)}>{Catalog.sort_label(@sort)}</a>
    <% end %>
    """
  end

  defp sort_link_class(true), do: @sort_link_current
  defp sort_link_class(false), do: @sort_link

  defp category_slug(%{category: nil}), do: nil
  defp category_slug(%{category: category}), do: category.slug

  defp savings_cents(product), do: product.compare_at_cents - product.price_cents

  # Rounded down, like `Save $41.99 (26%)`.
  defp savings_percent(product), do: div(savings_cents(product) * 100, product.compare_at_cents)

  @stepper_button "rounded-full p-2.5 text-stone-700 hover:bg-stone-100 disabled:cursor-not-allowed disabled:opacity-30 disabled:hover:bg-transparent"

  defp stepper_button_class, do: @stepper_button

  @doc "The availability line of the detail page."
  attr :stock, :integer, required: true

  def availability(%{stock: 0} = assigns) do
    ~H"""
    <dd class="font-medium text-stone-500">Sold out</dd>
    """
  end

  def availability(%{stock: stock} = assigns) when stock < 10 do
    ~H"""
    <dd class="font-medium text-amber-700">Only {@stock} left in stock</dd>
    """
  end

  def availability(assigns) do
    ~H"""
    <dd class="font-medium text-emerald-700">In stock</dd>
    """
  end
end
