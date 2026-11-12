defmodule HarborWeb.PageController do
  use HarborWeb, :controller

  alias Harbor.Catalog

  @doc "`GET /`: hero, store perks, category links and featured products."
  def home(conn, _params) do
    render(conn, :home,
      categories: Catalog.list_categories(),
      featured: Catalog.featured_products()
    )
  end

  @doc "`GET /about`: a small, static page in the shared layout."
  def about(conn, _params) do
    render(conn, :about, page_title: "About")
  end
end
