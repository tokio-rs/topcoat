module ProductsHelper
  # Tailwind classes for each product color: the image tile and the accent used for the product
  # shape and the swatch dot.
  Swatch = Data.define(:label, :tile, :accent)
  SWATCHES = {
    "charcoal" => Swatch.new("Charcoal", "bg-stone-200", "bg-stone-700"),
    "sand" => Swatch.new("Sand", "bg-amber-100", "bg-amber-300"),
    "sage" => Swatch.new("Sage", "bg-green-100", "bg-green-700/70"),
    "navy" => Swatch.new("Navy", "bg-blue-100", "bg-blue-900"),
    "terracotta" => Swatch.new("Terracotta", "bg-orange-100", "bg-orange-700"),
    "ochre" => Swatch.new("Ochre", "bg-yellow-100", "bg-yellow-600"),
    "plum" => Swatch.new("Plum", "bg-purple-100", "bg-purple-800"),
    "sky" => Swatch.new("Sky", "bg-sky-100", "bg-sky-500")
  }.freeze

  SORT_LABELS = {
    "newest" => "Newest",
    "price-asc" => "Price: low to high",
    "price-desc" => "Price: high to low",
    "rating" => "Top rated"
  }.freeze

  STAR_CLASSES = {
    small: { true => "size-4 fill-amber-400", false => "size-4 fill-stone-300" },
    large: { true => "size-5 fill-amber-400", false => "size-5 fill-stone-300" }
  }.freeze

  def swatch(product)
    SWATCHES.fetch(product.color)
  end

  # The number of filled stars for a rating in tenths: the rating rounded half up.
  def filled_stars(rating_tenths)
    (rating_tenths + 5) / 10
  end

  # A listing URL. Parameters at their defaults are left out and the rest keep the order
  # category, sort, page (`products_path` would sort them alphabetically).
  def listing_path(category: nil, sort: Listing::DEFAULT_SORT, page: 1)
    query = []
    query << [ "category", category.slug ] if category
    query << [ "sort", sort ] unless sort == Listing::DEFAULT_SORT
    query << [ "page", page ] unless page == 1
    query.empty? ? products_path : "#{products_path}?#{URI.encode_www_form(query)}"
  end
end
