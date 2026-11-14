defmodule HarborWeb.HealthController do
  use HarborWeb, :controller

  @doc "`GET /health`: the readiness probe."
  def show(conn, _params), do: text(conn, "ok")
end
