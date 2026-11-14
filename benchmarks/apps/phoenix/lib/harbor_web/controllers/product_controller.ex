defmodule HarborWeb.ProductController do
  use HarborWeb, :controller

  alias Harbor.Catalog

  action_fallback HarborWeb.FallbackController

  @doc "`GET /products`: one page of products with category sidebar, sort links and pagination."
  def index(conn, params) do
    with {:ok, listing} <- Catalog.list_products(params) do
      title = if listing.category, do: listing.category.name, else: "All products"

      render(conn, :index,
        page_title: title,
        listing: listing,
        categories: Catalog.list_categories(),
        product_count: Catalog.product_count()
      )
    end
  end

  @doc "`GET /products/:id`: product details, specifications, reviews and related products."
  def show(conn, %{"id" => id}) do
    with {id, ""} <- Integer.parse(id),
         %Catalog.Product{} = product <- Catalog.get_product(id) do
      render(conn, :show,
        page_title: product.name,
        product: product,
        category: Catalog.category_of(product),
        related: Catalog.related_products(product)
      )
    else
      _ -> {:error, :not_found}
    end
  end
end
