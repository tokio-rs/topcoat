# A product from `products.json`. Dates are parsed at boot, like typed database columns, and the
# category is resolved to its record.
class Product < Data.define(
  :id, :slug, :name, :category, :price_cents, :compare_at_cents, :rating, :review_count, :stock,
  :color, :tagline, :description, :specs, :created_at, :featured, :related_ids, :reviews
)
  # A row of the specification table.
  Spec = Data.define(:label, :value)

  # A customer review.
  Review = Data.define(:author, :rating, :title, :body, :date)

  def self.from_json(row, category:)
    new(
      id: row["id"],
      slug: row["slug"],
      name: row["name"],
      category: category,
      price_cents: row["price_cents"],
      compare_at_cents: row["compare_at_cents"],
      rating: row["rating"],
      review_count: row["review_count"],
      stock: row["stock"],
      color: row["color"],
      tagline: row["tagline"],
      description: row["description"],
      specs: row["specs"].map { |spec| Spec.new(label: spec["label"], value: spec["value"]) }.freeze,
      created_at: Date.iso8601(row["created_at"]),
      featured: row["featured"],
      related_ids: row["related"],
      reviews: row["reviews"].map do |review|
        Review.new(
          author: review["author"],
          rating: review["rating"],
          title: review["title"],
          body: review["body"],
          date: Date.iso8601(review["date"])
        )
      end.freeze
    )
  end

  def to_param = id.to_s

  def on_sale? = !compare_at_cents.nil?

  def sold_out? = stock.zero?

  # The most a customer can add to the cart at once.
  def max_quantity = [stock, 10].min

  # The rating in tenths of a star, for example 43 for a 4.3 rating.
  def rating_tenths = (rating * 10).round

  def paragraphs = description.split("\n\n")

  # The fields of a product in list responses.
  def summary_json
    {
      id:, slug:, name:, category: category.slug, price_cents:, compare_at_cents:, rating:,
      review_count:, stock:, color:, tagline:, created_at:, featured:
    }
  end

  # Every field, as in `products.json`.
  def as_json(options = nil)
    summary_json.merge(
      description:,
      specs: specs.map(&:to_h),
      related: related_ids,
      reviews: reviews.map(&:to_h)
    ).as_json(options)
  end
end
