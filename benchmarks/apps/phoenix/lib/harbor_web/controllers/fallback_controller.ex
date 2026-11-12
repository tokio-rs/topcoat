defmodule HarborWeb.FallbackController do
  @moduledoc """
  Translates `{:error, reason}` results of controller actions into error responses: the HTML
  error page for pages, `{"error": "..."}` for the JSON API.

  See `Phoenix.Controller.action_fallback/1`.
  """
  use HarborWeb, :controller

  def call(conn, {:error, reason}) do
    {status, message} = error(reason)

    conn
    |> put_status(status)
    |> put_view(html: HarborWeb.ErrorHTML, json: HarborWeb.ErrorJSON)
    |> render(:"#{status}", status: status, error: message)
  end

  defp error(:not_found), do: {404, "product not found"}
  defp error(:category_not_found), do: {404, "category not found"}
  defp error(:page_not_found), do: {404, "page not found"}
  defp error(:invalid_sort), do: {422, "invalid sort"}
  defp error(:invalid_page), do: {422, "invalid page"}
  defp error(:invalid_query), do: {422, "invalid query"}
  defp error({:invalid, message}), do: {422, message}
end
