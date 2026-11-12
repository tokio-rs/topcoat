defmodule HarborWeb.API.CartJSON do
  @doc "A priced cart line."
  def create(%{product: product, quantity: quantity}) do
    %{
      product_id: product.id,
      name: product.name,
      quantity: quantity,
      unit_price_cents: product.price_cents,
      line_total_cents: product.price_cents * quantity
    }
  end
end
