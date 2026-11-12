using Harbor.Catalog;
using Harbor.Infrastructure;
using Microsoft.AspNetCore.Razor.TagHelpers;

namespace Harbor.TagHelpers;

/// <summary>
/// A row of five stars for a product's average rating or a review's rating, labelled for assistive
/// technology as a single image.
/// </summary>
/// <example>
/// <c>&lt;star-row product="product" large="true" /&gt;</c> or <c>&lt;star-row review="review" /&gt;</c>
/// </example>
[HtmlTargetElement("star-row", TagStructure = TagStructure.WithoutEndTag)]
public sealed class StarRowTagHelper : TagHelper
{
    private static readonly string SmallFilled = Star("size-4 fill-amber-400");
    private static readonly string SmallEmpty = Star("size-4 fill-stone-300");
    private static readonly string LargeFilled = Star("size-5 fill-amber-400");
    private static readonly string LargeEmpty = Star("size-5 fill-stone-300");

    /// <summary>Shows the product's rating, like <c>4.3</c>, rounded half up to whole stars.</summary>
    public Product? Product { get; set; }

    /// <summary>Shows the review's whole-star rating.</summary>
    public Review? Review { get; set; }

    /// <summary>Whether to use the large star size.</summary>
    public bool Large { get; set; }

    public override void Process(TagHelperContext context, TagHelperOutput output)
    {
        var (filled, label) = (Product, Review) switch
        {
            ({ } product, _) => ((product.RatingTenths + 5) / 10, Format.Rating(product.Rating)),
            (_, { } review) => (review.Rating, Format.Count(review.Rating)),
            _ => throw new InvalidOperationException("star-row needs a product or a review"),
        };
        output.TagName = "span";
        output.TagMode = TagMode.StartTagAndEndTag;
        output.Attributes.SetAttribute("class", "flex items-center gap-0.5");
        output.Attributes.SetAttribute("role", "img");
        output.Attributes.SetAttribute("aria-label", $"Rated {label} out of 5");
        var (filledStar, emptyStar) = Large ? (LargeFilled, LargeEmpty) : (SmallFilled, SmallEmpty);
        for (var star = 0; star < 5; star++)
        {
            output.Content.AppendHtml(star < filled ? filledStar : emptyStar);
        }
    }

    private static string Star(string @class) =>
        $"""<svg viewBox="0 0 20 20" class="{@class}"><path d="M10 1l3 6 6 1-4 4 1 6-6-3-6 3 1-6-4-4 6-1z"></path></svg>""";
}
