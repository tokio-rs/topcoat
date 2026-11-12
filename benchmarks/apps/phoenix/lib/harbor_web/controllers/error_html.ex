defmodule HarborWeb.ErrorHTML do
  @moduledoc """
  This module is invoked by your endpoint in case of errors on HTML requests, and by
  `HarborWeb.FallbackController` for unknown products and invalid listing links.

  The 404 and 422 pages render in the full layout, so the cart badge works there too.

  See config/config.exs.
  """
  use HarborWeb, :html

  @doc "The title and heading of an error page."
  def title(404), do: "Page not found"
  def title(422), do: "Invalid request"
  def title(status), do: Plug.Conn.Status.reason_phrase(status)

  def render(template, assigns) when template in ["404.html", "422.html"] do
    error_page(assigns)
  end

  # Other errors render a plain text page based on the template name, for example
  # "500.html" becomes "Internal Server Error".
  def render(template, _assigns) do
    Phoenix.Controller.status_message_from_template(template)
  end

  defp error_page(assigns) do
    # Errors raised before the router's pipeline ran (unknown routes) have no cart count yet.
    assigns =
      assigns
      |> Map.put_new_lazy(:cart_count, fn -> HarborWeb.CartCount.read(assigns.conn) end)
      |> Map.put(:message, message(assigns.status))

    ~H"""
    <Layouts.app cart_count={@cart_count}>
      <div class="mx-auto max-w-2xl px-4 py-24 text-center sm:py-32">
        <p class="text-sm font-semibold text-teal-700">{@status}</p>
        <h1 class="mt-3 text-4xl font-semibold tracking-tight text-stone-900">{title(@status)}</h1>
        <p class="mt-4 text-stone-600">{@message}</p>
        <div class="mt-10 flex flex-wrap justify-center gap-4">
          <a
            href={~p"/"}
            class="rounded-full bg-teal-700 px-6 py-3 text-sm font-semibold text-white hover:bg-teal-800"
          >
            Back to home
          </a>
          <a
            href={~p"/products"}
            class="rounded-full px-6 py-3 text-sm font-semibold text-stone-700 ring-1 ring-inset ring-stone-300 hover:bg-stone-100"
          >
            Browse products
          </a>
        </div>
      </div>
    </Layouts.app>
    """
  end

  defp message(404), do: "Sorry, we couldn't find the page you're looking for."

  defp message(422),
    do: "Some of the filters in this link aren't valid. Try browsing from the catalog instead."
end
