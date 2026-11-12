defmodule HarborWeb.API.ProductController do
  use HarborWeb, :controller

  alias Harbor.Catalog

  action_fallback HarborWeb.FallbackController

  @doc "`GET /api/products`: the same query as the listing page, as JSON."
  def index(conn, params) do
    with {:ok, listing} <- Catalog.list_products(params) do
      render(conn, :index, listing: listing)
    end
  end

  @doc "`GET /api/products/:id`: one product with all its fields."
  def show(conn, %{"id" => id}) do
    with {id, ""} <- Integer.parse(id),
         %Catalog.Product{} = product <- Catalog.get_product(id) do
      render(conn, :show, product: product)
    else
      _ -> {:error, :not_found}
    end
  end
end
