defmodule Harbor.Catalog.Listing do
  @moduledoc """
  One page of a product listing, as returned by `Harbor.Catalog.list_products/1`.

  `category` is `nil` for the whole catalog. `first_position` and `last_position` are the 1-based
  positions of the first and last product on the page.
  """

  defstruct [
    :category,
    :sort,
    :page,
    :page_count,
    :total,
    :first_position,
    :last_position,
    :products
  ]
end
