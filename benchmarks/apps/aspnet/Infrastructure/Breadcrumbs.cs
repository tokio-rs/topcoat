namespace Harbor.Infrastructure;

/// <summary>The model of the <c>_Breadcrumbs</c> partial: links to the parent pages and the current page.</summary>
public sealed record Breadcrumbs(IReadOnlyList<(string Label, string Href)> Trail, string Current);
