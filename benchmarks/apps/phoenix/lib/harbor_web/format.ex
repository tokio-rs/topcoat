defmodule HarborWeb.Format do
  @moduledoc """
  Formatting helpers for money, counts, ratings and dates, imported into every template.

  Dates go through `Calendar.strftime/2`. Elixir's standard library has no number formatter
  with thousands separators, so money and counts are small hand-written functions.
  """

  @doc """
  Formats an amount of cents as dollars with thousands separators: `123450` is `"$1,234.50"`.
  """
  def money(cents) do
    cents_part = cents |> rem(100) |> Integer.to_string() |> String.pad_leading(2, "0")
    "$#{count(div(cents, 100))}.#{cents_part}"
  end

  @doc ~S'Formats an integer with thousands separators: `1204` is `"1,204"`.'
  def count(number) when number < 1000, do: Integer.to_string(number)

  def count(number) do
    group = number |> rem(1000) |> Integer.to_string() |> String.pad_leading(3, "0")
    "#{count(div(number, 1000))},#{group}"
  end

  @doc ~S'Formats a rating with one decimal: `4.0` is `"4.0"`.'
  def rating(rating), do: :erlang.float_to_binary(rating, decimals: 1)

  @doc ~S'Formats a date like `"Mar 4, 2026"`.'
  def short_date(%Date{} = date), do: Calendar.strftime(date, "%b %-d, %Y")

  @doc ~S'Formats a UTC timestamp like `"Mar 4, 2026 14:05"`.'
  def short_datetime(%DateTime{} = datetime), do: Calendar.strftime(datetime, "%b %-d, %Y %H:%M")

  @doc ~S'Formats a date as ISO 8601 for `datetime` attributes: `"2026-03-04"`.'
  def iso_date(%Date{} = date), do: Calendar.strftime(date, "%Y-%m-%d")

  @doc ~S'Formats a UTC timestamp to the minute for `datetime` attributes: `"2026-03-04T14:05Z"`.'
  def iso_datetime(%DateTime{} = datetime), do: Calendar.strftime(datetime, "%Y-%m-%dT%H:%MZ")
end
