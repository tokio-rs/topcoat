"""Formatting filters and the shared components of the storefront.

Class strings chosen by data are literals here so the Tailwind CLI finds them.
"""

from django import template

register = template.Library()


@register.filter
def money(cents):
    """Integer cents as dollars, like `$1,234.50`.

    Built-in number filters follow the active locale's grouping settings; the store always uses
    US formatting.
    """
    return f"${cents // 100:,}.{cents % 100:02}"


@register.filter
def count(value):
    """An integer with thousands separators, like `1,204`."""
    return f"{value:,}"


STAR_CLASSES = {
    "small": ("size-4 fill-amber-400", "size-4 fill-stone-300"),
    "large": ("size-5 fill-amber-400", "size-5 fill-stone-300"),
}


@register.inclusion_tag("store/includes/stars.html")
def stars(rating_tenths, label, size):
    """A row of five stars, labelled with the rating.

    The number of filled stars is the rating rounded half up.
    """
    filled = (rating_tenths + 5) // 10
    on, off = STAR_CLASSES[size]
    return {"label": label, "classes": [on] * filled + [off] * (5 - filled)}


@register.inclusion_tag("store/includes/product_card.html")
def product_card(product):
    return {"product": product}


@register.inclusion_tag("store/includes/product_image.html")
def product_image(product):
    return {"product": product}


@register.inclusion_tag("store/includes/breadcrumbs.html")
def breadcrumbs(trail, current):
    """A trail of `(label, url)` links ending in the current page."""
    return {"trail": trail, "current": current}


@register.inclusion_tag("store/includes/pagination.html")
def pagination(listing):
    """Previous and next links with a link to every page of the listing."""
    page = listing.page
    return {
        "page": page,
        "page_count": listing.page_count,
        "previous_url": listing.url(page - 1) if page > 1 else None,
        "next_url": listing.url(page + 1) if page < listing.page_count else None,
        "pages": [
            (number, listing.url(number), number == page)
            for number in range(1, listing.page_count + 1)
        ],
    }


STATUS_BADGES = {
    "pending": ("Pending", "bg-amber-50 text-amber-800 ring-amber-600/20"),
    "paid": ("Paid", "bg-sky-50 text-sky-800 ring-sky-600/20"),
    "shipped": ("Shipped", "bg-indigo-50 text-indigo-700 ring-indigo-600/20"),
    "delivered": ("Delivered", "bg-emerald-50 text-emerald-800 ring-emerald-600/20"),
    "refunded": ("Refunded", "bg-stone-100 text-stone-700 ring-stone-500/20"),
    "cancelled": ("Cancelled", "bg-rose-50 text-rose-700 ring-rose-600/20"),
}


@register.inclusion_tag("store/includes/status_badge.html")
def status_badge(status):
    """A colored pill showing an order's status."""
    label, colors = STATUS_BADGES[status]
    return {"label": label, "colors": colors}
