"""Resolving and linking product listings, shared by the listing page and the JSON API."""

from dataclasses import dataclass
from urllib.parse import urlencode

from django.urls import reverse

from .catalog import Category, Product, Sort, catalog

PAGE_SIZE = 24


class ListingError(Exception):
    """Listing parameters that do not resolve to a page, with the status and API message."""

    def __init__(self, status, message):
        super().__init__(message)
        self.status = status
        self.message = message


@dataclass(slots=True)
class Listing:
    category: Category | None
    sort: Sort
    page: int
    page_count: int
    total: int
    products: list[Product]

    @classmethod
    def from_query(cls, query):
        """Resolves the `category`, `sort` and `page` parameters, validated in that order.

        Empty values count as absent.
        """
        category = None
        if slug := query.get("category"):
            category = catalog.category_by_slug.get(slug)
            if category is None:
                raise ListingError(404, "category not found")

        sort = Sort.NEWEST
        if value := query.get("sort"):
            try:
                sort = Sort(value)
            except ValueError:
                raise ListingError(422, "invalid sort") from None

        products = catalog.sorted_products(category, sort)
        total = len(products)
        page_count = max(1, -(-total // PAGE_SIZE))

        page = 1
        if value := query.get("page"):
            digits = value.lstrip("0")
            if not (value.isascii() and value.isdigit()) or not digits:
                raise ListingError(422, "invalid page")
            # Very long numbers are past the last page too, without parsing them.
            if len(digits) > 9 or int(digits) > page_count:
                raise ListingError(404, "page not found")
            page = int(digits)

        start = (page - 1) * PAGE_SIZE
        return cls(category, sort, page, page_count, total, products[start : start + PAGE_SIZE])

    @property
    def first_position(self):
        return (self.page - 1) * PAGE_SIZE + 1

    @property
    def last_position(self):
        return (self.page - 1) * PAGE_SIZE + len(self.products)

    def url(self, page):
        """A link to another page of this listing."""
        return listing_url(self.category, self.sort, page)


def listing_url(category=None, sort=Sort.NEWEST, page=1):
    """The URL of a listing page. Parameters at their default value are left out."""
    params = {}
    if category is not None:
        params["category"] = category.slug
    if sort != Sort.NEWEST:
        params["sort"] = sort.value
    if page != 1:
        params["page"] = page
    url = reverse("store:listing")
    return f"{url}?{urlencode(params)}" if params else url
