defmodule Harbor.Sales do
  @moduledoc """
  Orders and their summary figures.

  Like `Harbor.Catalog`, the orders are loaded once at startup, sorted newest first and
  aggregated (like a database query would), and kept in `:persistent_term`.
  """

  alias Harbor.Sales.Order

  @doc "Loads `orders.json` from `dir`, sorts the orders and computes the summary."
  def load!(dir) do
    orders =
      Path.join(dir, "orders.json")
      |> File.read!()
      |> Jason.decode!()
      |> Enum.map(&Order.from_json/1)
      |> Enum.sort_by(&{DateTime.to_unix(&1.placed_at), &1.id}, :desc)

    :persistent_term.put({__MODULE__, :orders}, orders)
    :persistent_term.put({__MODULE__, :summary}, summarize(orders))
    :ok
  end

  @doc "All orders, newest first, ties by descending id."
  def list_orders, do: :persistent_term.get({__MODULE__, :orders})

  @doc """
  Totals over all orders: the order count, revenue from paid, shipped and delivered orders, the
  average revenue order (rounded half up to a cent) and the refunded total.
  """
  def order_summary, do: :persistent_term.get({__MODULE__, :summary})

  defp summarize(orders) do
    revenue =
      for order <- orders, order.status in ~w(paid shipped delivered), do: order.total_cents

    refunded = for order <- orders, order.status == "refunded", do: order.total_cents
    revenue_cents = Enum.sum(revenue)

    average_cents =
      case length(revenue) do
        0 -> 0
        count -> div(revenue_cents + div(count, 2), count)
      end

    %{
      count: length(orders),
      revenue_cents: revenue_cents,
      average_cents: average_cents,
      refunded_cents: Enum.sum(refunded)
    }
  end
end
