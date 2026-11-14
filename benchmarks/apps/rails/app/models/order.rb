# An order from `orders.json`, with `placed_at` parsed at boot.
class Order < Data.define(
  :id, :number, :customer, :email, :placed_at, :status, :items, :total_cents, :country
)
  STATUSES = %w[pending paid shipped delivered refunded cancelled].freeze
  REVENUE_STATUSES = %w[paid shipped delivered].freeze

  def self.from_json(row)
    new(
      id: row["id"],
      number: row["number"],
      customer: row["customer"],
      email: row["email"],
      placed_at: Time.iso8601(row["placed_at"]).utc,
      status: row["status"],
      items: row["items"],
      total_cents: row["total_cents"],
      country: row["country"]
    )
  end

  def revenue? = REVENUE_STATUSES.include?(status)

  def refunded? = status == "refunded"
end
