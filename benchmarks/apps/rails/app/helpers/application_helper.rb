module ApplicationHelper
  # Integer cents as `$1,234.50`.
  def money(cents)
    number_to_currency(cents / 100r)
  end

  # A rating with one decimal, like `4.0`.
  def format_rating(value)
    format("%.1f", value)
  end

  # The text of the header cart badge.
  def cart_badge(count)
    count > 99 ? "99+" : count
  end
end
