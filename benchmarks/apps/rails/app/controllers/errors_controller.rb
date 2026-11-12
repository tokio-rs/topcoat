# The catch-all route for unknown paths, whatever the method.
class ErrorsController < ApplicationController
  skip_forgery_protection

  def not_found
    render_error :not_found
  end
end
