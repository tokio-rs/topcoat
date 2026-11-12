class ApplicationController < ActionController::Base
  # Changes to the importmap will invalidate the etag for HTML responses
  stale_when_importmap_changes

  helper_method :catalog, :cart_count

  rescue_from Listing::Error do |error|
    render_error error.status
  end

  private

  def catalog = Catalog.instance

  # The number of items in the cart, from the `cart_count` cookie that the add-to-cart Stimulus
  # controller writes. Anything but 1 to 9 digits counts as an empty cart.
  def cart_count
    value = cookies[:cart_count]
    value&.match?(/\A[0-9]{1,9}\z/) ? value.to_i : 0
  end

  # Renders the 404 or 422 page in the site layout.
  def render_error(status)
    render "errors/show", status:, locals: { code: Rack::Utils.status_code(status) }
  end
end
