using System.Text.Json.Serialization;
using Harbor.Api;

namespace Harbor.Catalog;

/// <summary>
/// Source-generated System.Text.Json metadata for the dataset and the API, with the snake_case names
/// used by the JSON files and the API.
/// </summary>
[JsonSourceGenerationOptions(
    PropertyNamingPolicy = JsonKnownNamingPolicy.SnakeCaseLower,
    RespectNullableAnnotations = true)]
[JsonSerializable(typeof(List<Category>))]
[JsonSerializable(typeof(List<Product>))]
[JsonSerializable(typeof(List<Order>))]
[JsonSerializable(typeof(ProductPage))]
[JsonSerializable(typeof(CartRequest))]
[JsonSerializable(typeof(CartLine))]
[JsonSerializable(typeof(ApiError))]
public sealed partial class HarborJsonContext : JsonSerializerContext;
