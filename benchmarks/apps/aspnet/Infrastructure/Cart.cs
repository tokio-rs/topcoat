using System.Globalization;

namespace Harbor.Infrastructure;

/// <summary>The number of items in the cart, read from the <c>cart_count</c> cookie.</summary>
public static class Cart
{
    public const string CookieName = "cart_count";

    /// <summary>
    /// Reads the cookie. It is valid only if it has 1 to 9 ASCII digits; a missing or malformed cookie
    /// counts as an empty cart.
    /// </summary>
    public static int Count(HttpRequest request)
    {
        var value = request.Cookies[CookieName];
        if (value is not { Length: >= 1 and <= 9 } || !value.All(char.IsAsciiDigit))
        {
            return 0;
        }
        return int.Parse(value, NumberStyles.None, CultureInfo.InvariantCulture);
    }

    /// <summary>The badge text: the count, or <c>99+</c> above 99.</summary>
    public static string BadgeText(int count) =>
        count > 99 ? "99+" : count.ToString(CultureInfo.InvariantCulture);
}
