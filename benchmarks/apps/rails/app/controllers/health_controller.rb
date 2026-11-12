class HealthController < ActionController::Base
  def show
    render plain: "ok"
  end
end
