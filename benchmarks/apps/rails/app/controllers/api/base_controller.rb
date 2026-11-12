# JSON endpoints. `ActionController::API` leaves out cookies, sessions, views and CSRF protection,
# which an API authenticated by other means does not need.
class Api::BaseController < ActionController::API
  rescue_from Listing::Error do |error|
    render_error error.message, status: error.status
  end

  private

  def catalog = Catalog.instance

  def render_error(message, status:)
    render json: { error: message }, status:
  end
end
