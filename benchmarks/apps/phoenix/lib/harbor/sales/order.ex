defmodule Harbor.Sales.Order do
  @moduledoc """
  An order from `orders.json`. `placed_at` is a UTC `DateTime`; `status` is one of `"pending"`,
  `"paid"`, `"shipped"`, `"delivered"`, `"refunded"` or `"cancelled"`.
  """

  defstruct [:id, :number, :customer, :email, :placed_at, :status, :items, :total_cents, :country]

  @doc false
  def from_json(json) do
    {:ok, placed_at, 0} = DateTime.from_iso8601(json["placed_at"])

    %__MODULE__{
      id: json["id"],
      number: json["number"],
      customer: json["customer"],
      email: json["email"],
      placed_at: placed_at,
      status: json["status"],
      items: json["items"],
      total_cents: json["total_cents"],
      country: json["country"]
    }
  end
end
