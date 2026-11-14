defmodule HarborWeb.API.ProductJSON do
  alias Harbor.Catalog.Product

  @doc "A page of products; items leave out the description, specs, reviews and related ids."
  def index(%{listing: listing}) do
    %{
      items: for(product <- listing.products, do: summary(product)),
      page: listing.page,
      page_count: listing.page_count,
      total: listing.total
    }
  end

  @doc "A product with all its fields."
  def show(%{product: product}) do
    product
    |> summary()
    |> Map.merge(%{
      description: product.description,
      specs: product.specs,
      related: product.related,
      reviews: product.reviews
    })
  end

  defp summary(%Product{} = product) do
    %{
      id: product.id,
      slug: product.slug,
      name: product.name,
      category: product.category,
      price_cents: product.price_cents,
      compare_at_cents: product.compare_at_cents,
      rating: product.rating,
      review_count: product.review_count,
      stock: product.stock,
      color: product.color,
      tagline: product.tagline,
      created_at: product.created_at,
      featured: product.featured
    }
  end
end
