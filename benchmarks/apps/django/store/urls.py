from django.urls import path

from . import api, views

app_name = "store"

urlpatterns = [
    path("health", views.health, name="health"),
    path("", views.home, name="home"),
    path("about", views.about, name="about"),
    path("products", views.listing, name="listing"),
    path("products/<int:product_id>", views.detail, name="product"),
    path("admin/orders", views.orders, name="orders"),
    path("api/products", api.product_list, name="api-products"),
    # A string, so that non-numeric ids get the JSON 404 instead of the HTML one.
    path("api/products/<str:product_id>", api.product_detail, name="api-product"),
    path("api/cart", api.add_to_cart, name="api-cart"),
]
