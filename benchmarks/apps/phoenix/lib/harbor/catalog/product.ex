defmodule Harbor.Catalog.Product do
  @moduledoc """
  A product from `products.json`.

  Dates are parsed into `Date` structs at load time, like typed database columns. `specs` is a
  list of `%{label, value}` maps and `reviews` a list of
  `%{author, rating, title, body, date}` maps, newest first.
  """

  defstruct [
    :id,
    :slug,
    :name,
    :category,
    :price_cents,
    :compare_at_cents,
    :rating,
    :review_count,
    :stock,
    :color,
    :tagline,
    :description,
    :specs,
    :created_at,
    :featured,
    :related,
    :reviews
  ]

  @doc false
  def from_json(json) do
    %__MODULE__{
      id: json["id"],
      slug: json["slug"],
      name: json["name"],
      category: json["category"],
      price_cents: json["price_cents"],
      compare_at_cents: json["compare_at_cents"],
      # Whole ratings are written as integers in the JSON file.
      rating: json["rating"] / 1,
      review_count: json["review_count"],
      stock: json["stock"],
      color: json["color"],
      tagline: json["tagline"],
      description: json["description"],
      specs: Enum.map(json["specs"], &%{label: &1["label"], value: &1["value"]}),
      created_at: Date.from_iso8601!(json["created_at"]),
      featured: json["featured"],
      related: json["related"],
      reviews:
        Enum.map(json["reviews"], fn review ->
          %{
            author: review["author"],
            rating: review["rating"],
            title: review["title"],
            body: review["body"],
            date: Date.from_iso8601!(review["date"])
          }
        end)
    }
  end

  @doc "The rating in tenths of a star, for example `43` for a 4.3 rating."
  def rating_tenths(%__MODULE__{rating: rating}), do: round(rating * 10)

  @doc "The most a customer can add to the cart at once."
  def max_quantity(%__MODULE__{stock: stock}), do: min(stock, 10)

  @doc "The description split into its paragraphs."
  def paragraphs(%__MODULE__{description: description}), do: String.split(description, "\n\n")
end
