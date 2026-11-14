defmodule Harbor.Catalog.Category do
  @moduledoc "A product category from `categories.json`, with its product count."

  defstruct [:slug, :name, :description, :product_count]

  @doc false
  def from_json(json, product_count) do
    %__MODULE__{
      slug: json["slug"],
      name: json["name"],
      description: json["description"],
      product_count: product_count
    }
  end
end
