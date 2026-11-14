"""The HTML pages."""

from django.http import Http404, HttpResponse
from django.shortcuts import render
from django.urls import reverse

from .catalog import Sort, catalog
from .listing import Listing, ListingError, listing_url


def health(request):
    return HttpResponse("ok", content_type="text/plain")


def home(request):
    return render(
        request,
        "store/home.html",
        {"categories": catalog.categories, "featured": catalog.featured},
    )


def about(request):
    return render(request, "store/about.html")


def listing(request):
    """One page of products with the category sidebar, sort links and pagination."""
    try:
        listing = Listing.from_query(request.GET)
    except ListingError as e:
        if e.status == 404:
            raise Http404(e.message) from None
        return invalid_request(request)

    category = listing.category
    home = reverse("store:home")
    if category:
        trail = [("Home", home), ("Products", reverse("store:listing"))]
    else:
        trail = [("Home", home)]
    # Category links keep the sort, sort links keep the category, and both reset the page.
    sidebar = [
        {
            "name": "All products",
            "count": len(catalog.products),
            "url": listing_url(None, listing.sort),
            "current": category is None,
        },
        *(
            {
                "name": c.name,
                "count": c.product_count,
                "url": listing_url(c, listing.sort),
                "current": c is category,
            }
            for c in catalog.categories
        ),
    ]
    sorts = [
        {"label": sort.label, "url": listing_url(category, sort), "current": sort is listing.sort}
        for sort in Sort
    ]
    return render(
        request,
        "store/listing.html",
        {
            "listing": listing,
            "title": category.name if category else "All products",
            "description": (
                category.description
                if category
                else "Everything in the Harbor Market catalog, from tents to turntables."
            ),
            "trail": trail,
            "current": category.name if category else "Products",
            "sidebar": sidebar,
            "sorts": sorts,
        },
    )


def detail(request, product_id):
    """A product with its specifications, reviews and related products."""
    product = catalog.product(product_id)
    if product is None:
        raise Http404("product not found")
    category_url = listing_url(product.category)
    trail = [
        ("Home", reverse("store:home")),
        ("Products", reverse("store:listing")),
        (product.category.name, category_url),
    ]
    return render(
        request,
        "store/detail.html",
        {"product": product, "category_url": category_url, "trail": trail},
    )


def orders(request):
    """Summary cards and a table of every order, newest first."""
    return render(
        request,
        "store/orders.html",
        {"orders": catalog.orders, "summary": catalog.order_summary},
    )


def invalid_request(request):
    return render(request, "422.html", status=422)
