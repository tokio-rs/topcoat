defmodule HarborWeb.PageHTML do
  @moduledoc """
  This module contains pages rendered by PageController.

  See the `page_html` directory for all templates available.
  """
  use HarborWeb, :html

  embed_templates "page_html/*"

  @doc "A store perk on the home page: an icon, a title and a line of text."
  attr :title, :string, required: true
  attr :text, :string, required: true
  slot :icon, required: true

  def perk(assigns) do
    ~H"""
    <li class="flex gap-4">
      <span class="flex size-11 shrink-0 items-center justify-center rounded-full bg-teal-50 text-teal-700">
        {render_slot(@icon)}
      </span>
      <div>
        <p class="font-semibold text-stone-900">{@title}</p>
        <p class="mt-1 text-sm text-stone-600">{@text}</p>
      </div>
    </li>
    """
  end

  defp facts do
    [
      {"Founded", "2016"},
      {"Workshops", "38"},
      {"Repairs last year", "2,140"},
      {"Orders shipped", "96,000+"}
    ]
  end
end
