defmodule HarborWeb.OrderHTML do
  @moduledoc """
  This module contains the order admin page rendered by OrderController.

  See the `order_html` directory for all templates available.
  """
  use HarborWeb, :html

  embed_templates "order_html/*"

  defp stats(summary) do
    [
      {"Orders", count(summary.count), "All time"},
      {"Revenue", money(summary.revenue_cents), "Paid, shipped and delivered"},
      {"Average order", money(summary.average_cents), "Across revenue orders"},
      {"Refunded", money(summary.refunded_cents), "Returned to customers"}
    ]
  end
end
