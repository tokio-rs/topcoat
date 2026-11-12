defmodule HarborWeb.OrderController do
  use HarborWeb, :controller

  alias Harbor.Sales

  @doc "`GET /admin/orders`: summary cards and a table of every order, newest first."
  def index(conn, _params) do
    render(conn, :index,
      page_title: "Orders",
      orders: Sales.list_orders(),
      summary: Sales.order_summary()
    )
  end
end
