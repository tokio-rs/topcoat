defmodule HarborWeb.CartCount do
  @moduledoc """
  Assigns `:cart_count` from the `cart_count` cookie, which the add-to-cart button writes.

  The cookie is valid only if it is 1 to 9 ASCII digits; anything else counts as an empty cart.
  """

  @behaviour Plug

  import Plug.Conn

  @impl true
  def init(opts), do: opts

  @impl true
  def call(conn, _opts), do: assign(conn, :cart_count, read(conn))

  @doc "Reads the cart count from the request cookies."
  def read(conn) do
    case fetch_cookies(conn).cookies do
      %{"cart_count" => value} when byte_size(value) in 1..9 ->
        if digits?(value), do: String.to_integer(value), else: 0

      _ ->
        0
    end
  end

  defp digits?(<<digit, rest::binary>>) when digit in ?0..?9, do: rest == "" or digits?(rest)
  defp digits?(_), do: false

  @doc "The header badge text: the count, capped at `99+`."
  def badge(count) when count > 99, do: "99+"
  def badge(count), do: Integer.to_string(count)
end
