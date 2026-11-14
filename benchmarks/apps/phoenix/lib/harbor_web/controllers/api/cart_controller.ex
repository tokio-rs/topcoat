defmodule HarborWeb.API.CartController do
  use HarborWeb, :controller

  alias Harbor.Catalog
  alias Harbor.Catalog.Product

  action_fallback HarborWeb.FallbackController

  @doc """
  `POST /api/cart`: validates a cart line and prices it. Nothing is stored.

  The body must be a JSON object with integer `product_id` and `quantity`; every failure is a
  422 with an `{"error": "..."}` body.
  """
  def create(conn, %{"product_id" => product_id, "quantity" => quantity})
      when is_integer(product_id) and is_integer(quantity) do
    with {:ok, product} <- fetch_product(product_id),
         :ok <- validate_quantity(product, quantity) do
      render(conn, :create, product: product, quantity: quantity)
    end
  end

  def create(_conn, _params), do: {:error, {:invalid, "invalid request body"}}

  defp fetch_product(id) do
    case Catalog.get_product(id) do
      nil -> {:error, {:invalid, "product not found"}}
      %Product{stock: 0} -> {:error, {:invalid, "product is sold out"}}
      product -> {:ok, product}
    end
  end

  defp validate_quantity(product, quantity) do
    max = Product.max_quantity(product)

    if quantity in 1..max//1 do
      :ok
    else
      {:error, {:invalid, "quantity must be between 1 and #{max}"}}
    end
  end
end
