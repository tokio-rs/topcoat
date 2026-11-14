using System.Collections.Frozen;
using System.Text.Json;
using System.Text.Json.Serialization.Metadata;

namespace Harbor.Catalog;

/// <summary>
/// The in-memory dataset with the indexes built at startup.
/// </summary>
/// <remarks>
/// Every lookup is a constant-time index or a slice, standing in for a database that already did the
/// filtering, sorting and aggregation. Registered as a singleton.
/// </remarks>
public sealed class Store
{
    private readonly FrozenDictionary<string, Category> _categoriesBySlug;
    private readonly Product[] _products;
    private readonly Product[][] _sortedAll;
    private readonly FrozenDictionary<Category, Product[][]> _sortedByCategory;

    private Store(List<Category> categories, Product[] products, List<Order> orders)
    {
        Categories = categories;
        _categoriesBySlug = categories.ToFrozenDictionary(category => category.Slug);
        _products = products;

        foreach (var product in products)
        {
            product.Category = _categoriesBySlug.GetValueOrDefault(product.CategorySlug)
                ?? throw new InvalidDataException($"product {product.Id} has unknown category");
            product.Category.ProductCount++;
            if (product.Related.Any(id => Product(id) is null))
            {
                throw new InvalidDataException($"product {product.Id} has unknown related ids");
            }
        }

        _sortedAll = SortAll(products);
        _sortedByCategory = categories.ToFrozenDictionary(
            category => category,
            category => SortAll(products.Where(product => product.Category == category).ToArray()));
        Featured = products.Where(product => product.Featured).ToArray();

        Orders = orders
            .OrderByDescending(order => order.PlacedAt)
            .ThenByDescending(order => order.Id)
            .ToArray();
        OrderSummary = OrderSummary.From(Orders);
    }

    /// <summary>Every category, in data order.</summary>
    public IReadOnlyList<Category> Categories { get; }

    public int ProductCount => _products.Length;

    /// <summary>The featured products, by ascending id.</summary>
    public IReadOnlyList<Product> Featured { get; }

    /// <summary>All orders, newest first.</summary>
    public IReadOnlyList<Order> Orders { get; }

    public OrderSummary OrderSummary { get; }

    /// <summary>Loads the JSON dataset from <paramref name="directory"/> and builds the indexes.</summary>
    public static Store Load(string directory)
    {
        var categories = Read(directory, "categories.json", HarborJsonContext.Default.ListCategory);
        var products = Read(directory, "products.json", HarborJsonContext.Default.ListProduct)
            .OrderBy(product => product.Id)
            .ToArray();
        for (var index = 0; index < products.Length; index++)
        {
            if (products[index].Id != index + 1)
            {
                throw new InvalidDataException($"product ids must be 1..{products.Length}");
            }
        }
        var orders = Read(directory, "orders.json", HarborJsonContext.Default.ListOrder);
        return new Store(categories, products, orders);
    }

    public Category? CategoryBySlug(string slug) => _categoriesBySlug.GetValueOrDefault(slug);

    /// <summary>Looks up a product by id.</summary>
    public Product? Product(int id) => id >= 1 && id <= _products.Length ? _products[id - 1] : null;

    /// <summary>The related products of a product, in data order.</summary>
    public IEnumerable<Product> Related(Product product) =>
        product.Related.Select(id => _products[id - 1]);

    /// <summary>The products of a category, or of the whole catalog, in the given order.</summary>
    public Product[] Sorted(Category? category, Sort sort)
    {
        var sorted = category is null ? _sortedAll : _sortedByCategory[category];
        return sorted[(int)sort];
    }

    private static Product[][] SortAll(Product[] products) =>
        Sorts.All
            .Select(sort =>
            {
                var sorted = (Product[])products.Clone();
                Array.Sort(sorted, (a, b) => sort.Compare(a, b));
                return sorted;
            })
            .ToArray();

    private static T Read<T>(string directory, string file, JsonTypeInfo<T> typeInfo)
    {
        var path = Path.Combine(directory, file);
        using var stream = File.OpenRead(path);
        return JsonSerializer.Deserialize(stream, typeInfo)
            ?? throw new InvalidDataException($"{path}: empty document");
    }
}
