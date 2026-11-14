defmodule HarborWeb.ErrorJSON do
  @moduledoc """
  This module is invoked by your endpoint in case of errors on JSON requests, and by
  `HarborWeb.FallbackController` for API errors. Every error body is `{"error": "<message>"}`.

  See config/config.exs.
  """

  # The fallback controller passes a specific message; errors raised elsewhere use the status
  # message from the template name, for example "404.json" becomes "Not Found".
  def render(template, assigns) do
    %{error: assigns[:error] || Phoenix.Controller.status_message_from_template(template)}
  end
end
