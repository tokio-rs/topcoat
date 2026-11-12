CART_COOKIE = "cart_count"


def cart(request):
    """The cart badge in the header, read from the `cart_count` cookie on every page.

    The cookie counts only if it is 1 to 9 ASCII digits; anything else is an empty cart. The badge
    caps at `99+`.
    """
    value = request.COOKIES.get(CART_COOKIE, "")
    valid = 1 <= len(value) <= 9 and value.isascii() and value.isdigit()
    count = int(value) if valid else 0
    return {"cart_badge": "99+" if count > 99 else str(count)}
