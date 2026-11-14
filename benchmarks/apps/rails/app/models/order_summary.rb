# Totals over all orders, computed once at boot like an aggregate query.
class OrderSummary
  attr_reader :count, :revenue_cents, :average_cents, :refunded_cents

  def initialize(orders)
    revenue = orders.select(&:revenue?)
    @count = orders.size
    @revenue_cents = revenue.sum(&:total_cents)
    # Rounded half up to a cent.
    @average_cents = revenue.empty? ? 0 : (@revenue_cents + revenue.size / 2) / revenue.size
    @refunded_cents = orders.select(&:refunded?).sum(&:total_cents)
    freeze
  end
end
