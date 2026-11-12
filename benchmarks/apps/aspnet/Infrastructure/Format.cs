using System.Globalization;

namespace Harbor.Infrastructure;

/// <summary>
/// Formatting for the storefront with .NET's standard format strings. The culture is always passed
/// explicitly, so the output does not depend on the machine's locale.
/// </summary>
public static class Format
{
    private static readonly CultureInfo EnUs = CultureInfo.GetCultureInfo("en-US");

    /// <summary>An amount of money in cents, like <c>$1,234.50</c>.</summary>
    public static string Money(long cents) => (cents / 100m).ToString("C", EnUs);

    /// <summary>An integer with thousands separators, like <c>1,204</c>.</summary>
    public static string Count(long value) => value.ToString("N0", EnUs);

    /// <summary>A rating with one decimal, like <c>4.0</c>.</summary>
    public static string Rating(double rating) => rating.ToString("F1", EnUs);

    /// <summary>A calendar date, like <c>Mar 4, 2026</c>.</summary>
    public static string Date(DateOnly date) => date.ToString("MMM d, yyyy", EnUs);

    /// <summary>A date in ISO 8601 form, like <c>2026-03-04</c>, for <c>datetime</c> attributes.</summary>
    public static string IsoDate(DateOnly date) => date.ToString("yyyy-MM-dd", CultureInfo.InvariantCulture);

    /// <summary>A UTC timestamp, like <c>Mar 4, 2026 14:05</c>.</summary>
    public static string DateTime(DateTimeOffset time) =>
        time.UtcDateTime.ToString("MMM d, yyyy HH:mm", EnUs);

    /// <summary>A UTC timestamp in ISO 8601 form, like <c>2026-03-04T14:05Z</c>.</summary>
    public static string IsoDateTime(DateTimeOffset time) =>
        time.UtcDateTime.ToString("yyyy-MM-dd'T'HH:mm'Z'", CultureInfo.InvariantCulture);
}
