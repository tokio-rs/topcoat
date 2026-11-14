"""The in-memory dataset and the indexes built from it at startup.

The benchmark has no database. The JSON files are loaded once into plain objects, and every
lookup a request makes is a dictionary or list index or a slice, standing in for a database that
already did the filtering, sorting and aggregation.
"""

import json
from dataclasses import dataclass, field
from datetime import date, datetime
from enum import StrEnum
from pathlib import Path

from django.conf import settings

from .palette import PALETTES, Palette


class Sort(StrEnum):
    """The order of products in a listing; the value is the `sort` query parameter."""

    NEWEST = "newest"
    PRICE_ASC = "price-asc"
    PRICE_DESC = "price-desc"
    RATING = "rating"

    @property
    def label(self):
        return SORT_LABELS[self]


SORT_LABELS = {
    Sort.NEWEST: "Newest",
    Sort.PRICE_ASC: "Price: low to high",
    Sort.PRICE_DESC: "Price: high to low",
    Sort.RATING: "Top rated",
}

# Sort keys, all ending in the id so ties are broken by ascending id.
SORT_KEYS = {
    Sort.NEWEST: lambda p: (-p.created_at.toordinal(), p.id),
    Sort.PRICE_ASC: lambda p: (p.price_cents, p.id),
    Sort.PRICE_DESC: lambda p: (-p.price_cents, p.id),
    Sort.RATING: lambda p: (-p.rating_tenths, -p.review_count, p.id),
}


@dataclass(slots=True)
class Category:
    slug: str
    name: str
    description: str
    # Counted at startup.
    product_count: int = 0


@dataclass(slots=True)
class Spec:
    label: str
    value: str


@dataclass(slots=True)
class Review:
    author: str
    rating: int
    title: str
    body: str
    date: date

    @property
    def rating_tenths(self):
        return self.rating * 10


@dataclass(slots=True)
class Product:
    id: int
    slug: str
    name: str
    category: Category
    price_cents: int
    compare_at_cents: int | None
    rating: float
    review_count: int
    stock: int
    color: str
    tagline: str
    description: str
    specs: list[Spec]
    created_at: date
    featured: bool
    related_ids: list[int]
    reviews: list[Review]
    # The rating in tenths of a star, for example 43 for 4.3.
    rating_tenths: int = 0
    # The related products, resolved at startup like a prefetched relation.
    related: list["Product"] = field(default_factory=list)

    @property
    def palette(self) -> Palette:
        return PALETTES[self.color]

    @property
    def on_sale(self):
        return self.compare_at_cents is not None

    @property
    def is_sold_out(self):
        return self.stock == 0

    @property
    def is_low_stock(self):
        return 1 <= self.stock <= 9

    @property
    def max_quantity(self):
        """The most a customer can add to the cart at once."""
        return min(self.stock, 10)

    @property
    def savings_cents(self):
        return self.compare_at_cents - self.price_cents

    @property
    def savings_percent(self):
        return self.savings_cents * 100 // self.compare_at_cents

    @property
    def paragraphs(self):
        return self.description.split("\n\n")

    def summary(self):
        """The fields included in list responses."""
        return {
            "id": self.id,
            "slug": self.slug,
            "name": self.name,
            "category": self.category.slug,
            "price_cents": self.price_cents,
            "compare_at_cents": self.compare_at_cents,
            "rating": self.rating,
            "review_count": self.review_count,
            "stock": self.stock,
            "color": self.color,
            "tagline": self.tagline,
            "created_at": self.created_at.isoformat(),
            "featured": self.featured,
        }

    def details(self):
        """Every field, as in `products.json`."""
        return {
            **self.summary(),
            "description": self.description,
            "specs": [{"label": spec.label, "value": spec.value} for spec in self.specs],
            "related": self.related_ids,
            "reviews": [
                {
                    "author": review.author,
                    "rating": review.rating,
                    "title": review.title,
                    "body": review.body,
                    "date": review.date.isoformat(),
                }
                for review in self.reviews
            ],
        }


REVENUE_STATUSES = frozenset({"paid", "shipped", "delivered"})


@dataclass(slots=True)
class Order:
    id: int
    number: str
    customer: str
    email: str
    placed_at: datetime
    status: str
    items: int
    total_cents: int
    country: str


@dataclass(slots=True)
class OrderSummary:
    """Aggregates over all orders, computed at startup like a database query would."""

    count: int
    revenue_cents: int
    average_cents: int
    refunded_cents: int

    @classmethod
    def of(cls, orders):
        revenue = [order.total_cents for order in orders if order.status in REVENUE_STATUSES]
        revenue_cents = sum(revenue)
        # Rounded half up to a cent.
        average_cents = (revenue_cents + len(revenue) // 2) // len(revenue) if revenue else 0
        refunded_cents = sum(o.total_cents for o in orders if o.status == "refunded")
        return cls(len(orders), revenue_cents, average_cents, refunded_cents)


class Catalog:
    def __init__(self, data_dir: Path):
        self.categories = [Category(**c) for c in read_json(data_dir / "categories.json")]
        self.category_by_slug = {category.slug: category for category in self.categories}

        self.products = sorted(
            (self._product(raw) for raw in read_json(data_dir / "products.json")),
            key=lambda product: product.id,
        )
        self.product_by_id = {product.id: product for product in self.products}
        for product in self.products:
            product.category.product_count += 1
            product.related = [self.product_by_id[id] for id in product.related_ids]
        self.featured = [product for product in self.products if product.featured]

        self._sorted = {}
        for category in [None, *self.categories]:
            products = [p for p in self.products if category in (None, p.category)]
            for sort, key in SORT_KEYS.items():
                self._sorted[category and category.slug, sort] = sorted(products, key=key)

        self.orders = sorted(
            (self._order(raw) for raw in read_json(data_dir / "orders.json")),
            key=lambda order: (order.placed_at, order.id),
            reverse=True,
        )
        self.order_summary = OrderSummary.of(self.orders)

    def product(self, id):
        return self.product_by_id.get(id)

    def sorted_products(self, category, sort):
        """The products of a category, or all products, in the given order."""
        return self._sorted[category and category.slug, sort]

    def _product(self, raw):
        related_ids = raw.pop("related")
        return Product(
            **{
                **raw,
                "category": self.category_by_slug[raw["category"]],
                "specs": [Spec(**spec) for spec in raw["specs"]],
                "created_at": date.fromisoformat(raw["created_at"]),
                "related_ids": related_ids,
                "reviews": [
                    Review(**{**review, "date": date.fromisoformat(review["date"])})
                    for review in raw["reviews"]
                ],
                # The data has one decimal, so this only removes float noise like 42.99999.
                "rating_tenths": round(raw["rating"] * 10),
            }
        )

    def _order(self, raw):
        return Order(**{**raw, "placed_at": datetime.fromisoformat(raw["placed_at"])})


def read_json(path):
    with open(path, encoding="utf-8") as file:
        return json.load(file)


catalog = Catalog(settings.BENCH_DATA_DIR)
