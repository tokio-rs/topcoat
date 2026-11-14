"""The JSON API."""

import json

from django.http import JsonResponse
from django.views.decorators.csrf import csrf_exempt
from django.views.decorators.http import require_POST

from .catalog import catalog
from .listing import Listing, ListingError


def error(status, message):
    return JsonResponse({"error": message}, status=status)


def product_list(request):
    """`GET /api/products`: the same query as the listing page."""
    try:
        listing = Listing.from_query(request.GET)
    except ListingError as e:
        return error(e.status, e.message)
    return JsonResponse(
        {
            "items": [product.summary() for product in listing.products],
            "page": listing.page,
            "page_count": listing.page_count,
            "total": listing.total,
        }
    )


def product_detail(request, product_id):
    """`GET /api/products/<id>`: one product with all its fields."""
    is_id = product_id.isascii() and product_id.isdigit() and len(product_id) <= 9
    product = catalog.product(int(product_id)) if is_id else None
    if product is None:
        return error(404, "product not found")
    return JsonResponse(product.details())


@csrf_exempt
@require_POST
def add_to_cart(request):
    """`POST /api/cart`: validates a cart line and prices it. Nothing is stored."""
    try:
        if request.content_type != "application/json":
            raise ValueError
        body = json.loads(request.body)
        product_id, quantity = body["product_id"], body["quantity"]
        if type(product_id) is not int or type(quantity) is not int:
            raise ValueError
    except (ValueError, TypeError, KeyError):
        return error(422, "invalid request body")

    product = catalog.product(product_id)
    if product is None:
        return error(422, "product not found")
    if product.is_sold_out:
        return error(422, "product is sold out")
    if not 1 <= quantity <= product.max_quantity:
        return error(422, f"quantity must be between 1 and {product.max_quantity}")
    return JsonResponse(
        {
            "product_id": product.id,
            "name": product.name,
            "quantity": quantity,
            "unit_price_cents": product.price_cents,
            "line_total_cents": product.price_cents * quantity,
        }
    )
