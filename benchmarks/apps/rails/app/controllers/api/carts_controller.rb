# Prices a cart line. Nothing is stored: the cart lives in the `cart_count` cookie.
class Api::CartsController < Api::BaseController
  rescue_from ActionDispatch::Http::Parameters::ParseError do
    render_error "invalid request body", status: :unprocessable_content
  end

  def create
    product_id, quantity = params.values_at(:product_id, :quantity)
    unless product_id.is_a?(Integer) && quantity.is_a?(Integer)
      return render_error("invalid request body", status: :unprocessable_content)
    end

    product = catalog.product(product_id)
    return render_error("product not found", status: :unprocessable_content) unless product
    return render_error("product is sold out", status: :unprocessable_content) if product.sold_out?

    max = product.max_quantity
    unless quantity.between?(1, max)
      return render_error("quantity must be between 1 and #{max}", status: :unprocessable_content)
    end

    render json: {
      product_id: product.id,
      name: product.name,
      quantity:,
      unit_price_cents: product.price_cents,
      line_total_cents: product.price_cents * quantity
    }
  end
end
