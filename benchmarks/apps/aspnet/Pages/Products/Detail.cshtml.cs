using Harbor.Catalog;
using Microsoft.AspNetCore.Mvc;
using Microsoft.AspNetCore.Mvc.RazorPages;

namespace Harbor.Pages.Products;

/// <summary>Product details, specifications, reviews and related products.</summary>
public sealed class DetailModel(Store store) : PageModel
{
    public Product Product { get; private set; } = null!;

    public IEnumerable<Product> Related => store.Related(Product);

    public IActionResult OnGet(int id)
    {
        if (store.Product(id) is not { } product)
        {
            return NotFound();
        }
        Product = product;
        return Page();
    }
}
