using System.Text.Json.Serialization;

namespace Harbor.Catalog;

/// <summary>A product category from <c>categories.json</c>.</summary>
public sealed class Category
{
    public required string Slug { get; init; }
    public required string Name { get; init; }
    public required string Description { get; init; }

    /// <summary>Number of products in the category, counted at startup.</summary>
    [JsonIgnore]
    public int ProductCount { get; set; }
}

/// <summary>A product from <c>products.json</c>. Serialized as is by the product API.</summary>
public sealed class Product
{
    public required int Id { get; init; }
    public required string Slug { get; init; }
    public required string Name { get; init; }
    [JsonPropertyName("category")]
    public required string CategorySlug { get; init; }
    public required int PriceCents { get; init; }
    public int? CompareAtCents { get; init; }
    public required double Rating { get; init; }
    public required int ReviewCount { get; init; }
    public required int Stock { get; init; }
    public required string Color { get; init; }
    public required string Tagline { get; init; }
    public required string Description { get; init; }
    public required List<Spec> Specs { get; init; }
    public required DateOnly CreatedAt { get; init; }
    public required bool Featured { get; init; }
    public required List<int> Related { get; init; }
    public required List<Review> Reviews { get; init; }

    /// <summary>The product's category, resolved at startup.</summary>
    [JsonIgnore]
    public Category Category { get; set; } = null!;

    [JsonIgnore]
    public bool IsSoldOut => Stock == 0;

    [JsonIgnore]
    public bool IsOnSale => CompareAtCents is not null;

    /// <summary>The most a customer can add to the cart at once.</summary>
    [JsonIgnore]
    public int MaxQuantity => Math.Min(Stock, 10);

    /// <summary>The rating in tenths of a star, for example 43 for a 4.3 rating.</summary>
    [JsonIgnore]
    public int RatingTenths => (int)Math.Round(Rating * 10);

    /// <summary>The description split into its paragraphs.</summary>
    public string[] Paragraphs() => Description.Split("\n\n");
}

/// <summary>The product fields included in list responses.</summary>
public sealed record ProductSummary(
    int Id,
    string Slug,
    string Name,
    string Category,
    int PriceCents,
    int? CompareAtCents,
    double Rating,
    int ReviewCount,
    int Stock,
    string Color,
    string Tagline,
    DateOnly CreatedAt,
    bool Featured)
{
    public static ProductSummary From(Product product) =>
        new(
            product.Id,
            product.Slug,
            product.Name,
            product.CategorySlug,
            product.PriceCents,
            product.CompareAtCents,
            product.Rating,
            product.ReviewCount,
            product.Stock,
            product.Color,
            product.Tagline,
            product.CreatedAt,
            product.Featured);
}

/// <summary>One row of a product's specification table.</summary>
public sealed record Spec(string Label, string Value);

/// <summary>A customer review of a product.</summary>
public sealed record Review(string Author, int Rating, string Title, string Body, DateOnly Date);

/// <summary>An order from <c>orders.json</c>.</summary>
public sealed record Order(
    int Id,
    string Number,
    string Customer,
    string Email,
    DateTimeOffset PlacedAt,
    OrderStatus Status,
    int Items,
    long TotalCents,
    string Country);

/// <summary>The fulfilment state of an order.</summary>
[JsonConverter(typeof(JsonStringEnumConverter<OrderStatus>))]
public enum OrderStatus
{
    [JsonStringEnumMemberName("pending")] Pending,
    [JsonStringEnumMemberName("paid")] Paid,
    [JsonStringEnumMemberName("shipped")] Shipped,
    [JsonStringEnumMemberName("delivered")] Delivered,
    [JsonStringEnumMemberName("refunded")] Refunded,
    [JsonStringEnumMemberName("cancelled")] Cancelled,
}

/// <summary>Aggregates over all orders, computed once at startup like a database query would.</summary>
public sealed record OrderSummary(int Count, long RevenueCents, long AverageCents, long RefundedCents)
{
    public static OrderSummary From(IReadOnlyCollection<Order> orders)
    {
        long revenue = 0, refunded = 0;
        var revenueOrders = 0;
        foreach (var order in orders)
        {
            switch (order.Status)
            {
                case OrderStatus.Paid or OrderStatus.Shipped or OrderStatus.Delivered:
                    revenue += order.TotalCents;
                    revenueOrders++;
                    break;
                case OrderStatus.Refunded:
                    refunded += order.TotalCents;
                    break;
            }
        }
        var average = revenueOrders == 0 ? 0 : (revenue + revenueOrders / 2) / revenueOrders;
        return new OrderSummary(orders.Count, revenue, average, refunded);
    }
}
