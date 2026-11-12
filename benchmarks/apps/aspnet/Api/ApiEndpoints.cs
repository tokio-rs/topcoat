using System.Globalization;
using System.Text.Json;
using Harbor.Catalog;
using Microsoft.AspNetCore.Http.HttpResults;

namespace Harbor.Api;

/// <summary>A page of products.</summary>
public sealed record ProductPage(IReadOnlyList<ProductSummary> Items, int Page, int PageCount, int Total);

/// <summary>The body of an add-to-cart request. Extra fields are ignored.</summary>
public sealed class CartRequest
{
    // `required` makes System.Text.Json reject a body without them instead of defaulting to 0.
    public required long ProductId { get; init; }
    public required long Quantity { get; init; }
}

/// <summary>A priced cart line.</summary>
public sealed record CartLine(int ProductId, string Name, int Quantity, int UnitPriceCents, long LineTotalCents);

/// <summary>The body of every JSON error response.</summary>
public sealed record ApiError(string Error);

/// <summary>The JSON API, as Minimal API endpoints.</summary>
public static class ApiEndpoints
{
    public static void MapApi(this IEndpointRouteBuilder app)
    {
        var api = app.MapGroup("/api");
        api.MapGet("/products", ListProducts).WithName("ListProducts");
        api.MapGet("/products/{id}", GetProduct);
        api.MapPost("/cart", AddToCart);
    }

    /// <summary><c>GET /api/products</c>: the same query as the listing page, as JSON.</summary>
    private static Results<Ok<ProductPage>, JsonHttpResult<ApiError>> ListProducts(
        Store store,
        string? category,
        string? sort,
        string? page)
    {
        if (!Listing.TryCreate(store, category, sort, page, out var listing, out var error))
        {
            return error switch
            {
                ListingError.CategoryNotFound => NotFound("category not found"),
                ListingError.PageNotFound => NotFound("page not found"),
                ListingError.InvalidSort => Invalid("invalid sort"),
                _ => Invalid("invalid page"),
            };
        }
        var items = new ProductSummary[listing.Products.Count];
        for (var i = 0; i < items.Length; i++)
        {
            items[i] = ProductSummary.From(listing.Products[i]);
        }
        return TypedResults.Ok(new ProductPage(items, listing.Page, listing.PageCount, listing.Total));
    }

    /// <summary><c>GET /api/products/{id}</c>: one product with all its fields.</summary>
    private static Results<Ok<Product>, JsonHttpResult<ApiError>> GetProduct(Store store, string id)
    {
        if (!int.TryParse(id, CultureInfo.InvariantCulture, out var productId) || store.Product(productId) is not { } product)
        {
            return NotFound("product not found");
        }
        return TypedResults.Ok(product);
    }

    /// <summary><c>POST /api/cart</c>: validates a cart line and prices it. Nothing is stored.</summary>
    private static async Task<Results<Ok<CartLine>, JsonHttpResult<ApiError>>> AddToCart(
        Store store,
        HttpRequest request)
    {
        // The body is read here rather than bound as a parameter so that malformed bodies get the
        // 422 response from the spec instead of the framework's 400.
        CartRequest? body = null;
        if (request.HasJsonContentType())
        {
            try
            {
                body = await request.ReadFromJsonAsync<CartRequest>();
            }
            catch (JsonException)
            {
            }
        }
        if (body is null)
        {
            return Invalid("invalid request body");
        }

        if (body.ProductId is < 1 or > int.MaxValue || store.Product((int)body.ProductId) is not { } product)
        {
            return Invalid("product not found");
        }
        if (product.IsSoldOut)
        {
            return Invalid("product is sold out");
        }
        var max = product.MaxQuantity;
        if (body.Quantity < 1 || body.Quantity > max)
        {
            return Invalid($"quantity must be between 1 and {max}");
        }
        var quantity = (int)body.Quantity;
        return TypedResults.Ok(new CartLine(
            product.Id,
            product.Name,
            quantity,
            product.PriceCents,
            (long)product.PriceCents * quantity));
    }

    private static JsonHttpResult<ApiError> NotFound(string message) =>
        TypedResults.Json(new ApiError(message), statusCode: StatusCodes.Status404NotFound);

    private static JsonHttpResult<ApiError> Invalid(string message) =>
        TypedResults.Json(new ApiError(message), statusCode: StatusCodes.Status422UnprocessableEntity);
}
