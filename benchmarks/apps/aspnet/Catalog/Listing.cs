using System.Globalization;

namespace Harbor.Catalog;

/// <summary>The order of products in a listing.</summary>
public enum Sort
{
    Newest,
    PriceAsc,
    PriceDesc,
    Rating,
}

public static class Sorts
{
    public static readonly Sort[] All = [Sort.Newest, Sort.PriceAsc, Sort.PriceDesc, Sort.Rating];

    /// <summary>The value of the <c>sort</c> query parameter.</summary>
    public static string Param(this Sort sort) => sort switch
    {
        Sort.Newest => "newest",
        Sort.PriceAsc => "price-asc",
        Sort.PriceDesc => "price-desc",
        _ => "rating",
    };

    public static string Label(this Sort sort) => sort switch
    {
        Sort.Newest => "Newest",
        Sort.PriceAsc => "Price: low to high",
        Sort.PriceDesc => "Price: high to low",
        _ => "Top rated",
    };

    public static Sort? FromParam(string value) => value switch
    {
        "newest" => Sort.Newest,
        "price-asc" => Sort.PriceAsc,
        "price-desc" => Sort.PriceDesc,
        "rating" => Sort.Rating,
        _ => null,
    };

    /// <summary>Orders two products. Ties are broken by ascending id.</summary>
    public static int Compare(this Sort sort, Product a, Product b)
    {
        var primary = sort switch
        {
            Sort.Newest => b.CreatedAt.CompareTo(a.CreatedAt),
            Sort.PriceAsc => a.PriceCents.CompareTo(b.PriceCents),
            Sort.PriceDesc => b.PriceCents.CompareTo(a.PriceCents),
            _ => b.RatingTenths != a.RatingTenths
                ? b.RatingTenths.CompareTo(a.RatingTenths)
                : b.ReviewCount.CompareTo(a.ReviewCount),
        };
        return primary != 0 ? primary : a.Id.CompareTo(b.Id);
    }
}

/// <summary>Why listing parameters could not be resolved.</summary>
public enum ListingError
{
    CategoryNotFound,
    InvalidSort,
    InvalidPage,
    PageNotFound,
}

/// <summary>One page of a product listing.</summary>
public sealed class Listing
{
    /// <summary>Number of products on one listing page.</summary>
    public const int PageSize = 24;

    public Category? Category { get; private init; }
    public Sort Sort { get; private init; }
    public int Page { get; private init; }
    public int PageCount { get; private init; }
    public int Total { get; private init; }

    /// <summary>The products on this page.</summary>
    public ArraySegment<Product> Products { get; private init; }

    public string? CategorySlug => Category?.Slug;

    /// <summary>1-based position of the first product on this page, or 0 if the listing is empty.</summary>
    public int FirstPosition => Total == 0 ? 0 : (Page - 1) * PageSize + 1;

    /// <summary>1-based position of the last product on this page.</summary>
    public int LastPosition => (Page - 1) * PageSize + Products.Count;

    /// <summary>
    /// Resolves the raw query parameters against the store. Empty values count as absent, and the
    /// parameters are validated in order: category, then sort, then page.
    /// </summary>
    public static bool TryCreate(
        Store store,
        string? category,
        string? sort,
        string? page,
        out Listing listing,
        out ListingError error)
    {
        listing = null!;
        Category? resolvedCategory = null;
        if (!string.IsNullOrEmpty(category))
        {
            resolvedCategory = store.CategoryBySlug(category);
            if (resolvedCategory is null)
            {
                error = ListingError.CategoryNotFound;
                return false;
            }
        }

        var resolvedSort = Sort.Newest;
        if (!string.IsNullOrEmpty(sort))
        {
            if (Sorts.FromParam(sort) is not { } parsed)
            {
                error = ListingError.InvalidSort;
                return false;
            }
            resolvedSort = parsed;
        }

        var sorted = store.Sorted(resolvedCategory, resolvedSort);
        var total = sorted.Length;
        var pageCount = Math.Max(1, (total + PageSize - 1) / PageSize);
        var resolvedPage = 1;
        if (!string.IsNullOrEmpty(page))
        {
            if (!page.All(char.IsAsciiDigit))
            {
                error = ListingError.InvalidPage;
                return false;
            }
            // Numbers too large to represent are past any last page.
            if (!int.TryParse(page, NumberStyles.None, CultureInfo.InvariantCulture, out resolvedPage))
            {
                error = ListingError.PageNotFound;
                return false;
            }
            if (resolvedPage == 0)
            {
                error = ListingError.InvalidPage;
                return false;
            }
        }
        if (resolvedPage > pageCount)
        {
            error = ListingError.PageNotFound;
            return false;
        }

        var start = (resolvedPage - 1) * PageSize;
        var count = Math.Min(PageSize, total - start);
        listing = new Listing
        {
            Category = resolvedCategory,
            Sort = resolvedSort,
            Page = resolvedPage,
            PageCount = pageCount,
            Total = total,
            Products = new ArraySegment<Product>(sorted, start, count),
        };
        error = default;
        return true;
    }
}
