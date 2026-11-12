using Harbor.Catalog;
using Microsoft.AspNetCore.Mvc;
using Microsoft.AspNetCore.Mvc.RazorPages;

namespace Harbor.Pages.Products;

/// <summary>One page of products with category sidebar, sort links and pagination.</summary>
public sealed class IndexModel(Store store) : PageModel
{
    public Store Store => store;

    public Listing Listing { get; private set; } = null!;

    public string Heading => Listing.Category?.Name ?? "All products";

    public string Description => Listing.Category?.Description
        ?? "Everything in the Harbor Market catalog, from tents to turntables.";

    // `page` is a reserved route value in Razor Pages (it names the page), so the query parameters
    // are bound from the query string explicitly.
    public IActionResult OnGet(
        [FromQuery] string? category,
        [FromQuery] string? sort,
        [FromQuery(Name = "page")] string? pageNumber)
    {
        if (!Listing.TryCreate(store, category, sort, pageNumber, out var listing, out var error))
        {
            return error is ListingError.CategoryNotFound or ListingError.PageNotFound
                ? NotFound()
                : StatusCode(StatusCodes.Status422UnprocessableEntity);
        }
        Listing = listing;
        return Page();
    }
}
