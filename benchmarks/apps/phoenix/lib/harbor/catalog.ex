defmodule Harbor.Catalog do
  @moduledoc """
  The product catalog: categories, products and the listing queries.

  The dataset is read-only reference data, so `load!/1` reads it once at startup, builds every
  index (lookup by id and slug, sort orders per category, featured products, category counts)
  and stores the results in `:persistent_term`. Reads are then constant-time and copy-free from
  every request process, standing in for a database that already did its job.
  """

  alias Harbor.Catalog.{Category, Listing, Product}

  @page_size 24
  @sorts [:newest, :price_asc, :price_desc, :rating]

  ## Loading

  @doc """
  Loads `categories.json` and `products.json` from `dir` and builds the indexes.
  """
  def load!(dir) do
    products =
      Path.join(dir, "products.json")
      |> read_json!()
      |> Enum.map(&Product.from_json/1)
      |> Enum.sort_by(& &1.id)

    counts = Enum.frequencies_by(products, & &1.category)

    categories =
      Path.join(dir, "categories.json")
      |> read_json!()
      |> Enum.map(&Category.from_json(&1, Map.get(counts, &1["slug"], 0)))

    category_slugs = Enum.map(categories, & &1.slug)

    sorted =
      for category <- [nil | category_slugs], sort <- @sorts, into: %{} do
        ids =
          products
          |> Enum.filter(&(category == nil or &1.category == category))
          |> Enum.sort_by(&sort_key(sort, &1))
          |> Enum.map(& &1.id)

        {{category, sort}, List.to_tuple(ids)}
      end

    put(:products, List.to_tuple(products))
    put(:categories, categories)
    put(:categories_by_slug, Map.new(categories, &{&1.slug, &1}))
    put(:featured, for(product <- products, product.featured, do: product.id))
    put(:sorted, sorted)
    :ok
  end

  defp read_json!(path), do: path |> File.read!() |> Jason.decode!()

  # Sort keys ascending; ties are broken by ascending id.
  defp sort_key(:newest, product), do: {-Date.to_gregorian_days(product.created_at), product.id}
  defp sort_key(:price_asc, product), do: {product.price_cents, product.id}
  defp sort_key(:price_desc, product), do: {-product.price_cents, product.id}

  defp sort_key(:rating, product),
    do: {-Product.rating_tenths(product), -product.review_count, product.id}

  defp put(key, value), do: :persistent_term.put({__MODULE__, key}, value)
  defp get(key), do: :persistent_term.get({__MODULE__, key})

  ## Queries

  @doc "All categories, in data order."
  def list_categories, do: get(:categories)

  @doc "The category with the given slug, or `nil`."
  def get_category(slug), do: Map.get(get(:categories_by_slug), slug)

  @doc "The category a product belongs to."
  def category_of(%Product{category: slug}), do: Map.fetch!(get(:categories_by_slug), slug)

  @doc "Number of products in the catalog."
  def product_count, do: tuple_size(get(:products))

  @doc "The product with the given id, or `nil`."
  def get_product(id) when is_integer(id) and id >= 1 do
    products = get(:products)
    if id <= tuple_size(products), do: elem(products, id - 1)
  end

  def get_product(_id), do: nil

  @doc "The featured products, by ascending id."
  def featured_products, do: Enum.map(get(:featured), &get_product/1)

  @doc "The related products of a product, in data order."
  def related_products(%Product{related: ids}), do: Enum.map(ids, &get_product/1)

  @doc "The listing sort orders, in display order."
  def sorts, do: @sorts

  @doc "The `sort` query parameter value of a sort order."
  def sort_param(:newest), do: "newest"
  def sort_param(:price_asc), do: "price-asc"
  def sort_param(:price_desc), do: "price-desc"
  def sort_param(:rating), do: "rating"

  @doc "The label of a sort order."
  def sort_label(:newest), do: "Newest"
  def sort_label(:price_asc), do: "Price: low to high"
  def sort_label(:price_desc), do: "Price: high to low"
  def sort_label(:rating), do: "Top rated"

  @doc """
  Resolves the `category`, `sort` and `page` listing parameters to one page of products.

  Empty values count as absent. Parameters are validated in order, and the first failure is
  returned as `{:error, reason}` with `reason` one of `:category_not_found`, `:invalid_sort`,
  `:invalid_page`, `:page_not_found` or `:invalid_query` (a parameter that is not a string).
  """
  def list_products(params) do
    with {:ok, category} <- param(params, "category", nil, &parse_category/1),
         {:ok, sort} <- param(params, "sort", :newest, &parse_sort/1),
         {:ok, page} <- param(params, "page", 1, &parse_page/1) do
      ids = Map.fetch!(get(:sorted), {category && category.slug, sort})
      total = tuple_size(ids)
      page_count = max(1, div(total + @page_size - 1, @page_size))

      if page > page_count do
        {:error, :page_not_found}
      else
        first = (page - 1) * @page_size
        last = min(first + @page_size, total) - 1

        {:ok,
         %Listing{
           category: category,
           sort: sort,
           page: page,
           page_count: page_count,
           total: total,
           first_position: first + 1,
           last_position: last + 1,
           products: for(index <- first..last//1, do: get_product(elem(ids, index)))
         }}
      end
    end
  end

  defp param(params, key, default, parse) do
    case Map.get(params, key) do
      nil -> {:ok, default}
      "" -> {:ok, default}
      value when is_binary(value) -> parse.(value)
      _ -> {:error, :invalid_query}
    end
  end

  defp parse_category(slug) do
    case get_category(slug) do
      nil -> {:error, :category_not_found}
      category -> {:ok, category}
    end
  end

  defp parse_sort("newest"), do: {:ok, :newest}
  defp parse_sort("price-asc"), do: {:ok, :price_asc}
  defp parse_sort("price-desc"), do: {:ok, :price_desc}
  defp parse_sort("rating"), do: {:ok, :rating}
  defp parse_sort(_), do: {:error, :invalid_sort}

  # Any run of digits is a page number; numbers past the last page are a 404, never clamped.
  defp parse_page(value) do
    if digits?(value) do
      case String.to_integer(value) do
        0 -> {:error, :invalid_page}
        page -> {:ok, page}
      end
    else
      {:error, :invalid_page}
    end
  end

  defp digits?(<<digit, rest::binary>>) when digit in ?0..?9, do: rest == "" or digits?(rest)
  defp digits?(_), do: false
end
