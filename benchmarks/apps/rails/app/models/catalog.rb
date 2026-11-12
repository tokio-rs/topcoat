# The in-memory Harbor Market dataset with the indexes built at boot.
#
# It stands in for a database that already did its job: every lookup a request makes is a hash
# or array access, or a slice of a presorted list. `config/initializers/catalog.rb` loads it while
# the app boots, so with `preload_app!` Puma workers share it copy-on-write.
class Catalog
  SORTS = %w[newest price-asc price-desc rating].freeze

  attr_reader :categories, :products, :featured, :orders, :order_summary

  class << self
    def instance
      @instance ||= new(Rails.configuration.x.data_dir)
    end
  end

  def initialize(data_dir)
    @categories = read_json(data_dir, "categories.json").map { |row| Category.from_json(row) }
    @categories.freeze
    @category_by_slug = @categories.index_by(&:slug).freeze

    @products = read_json(data_dir, "products.json")
      .map { |row| Product.from_json(row, category: @category_by_slug.fetch(row["category"])) }
      .sort_by(&:id)
    @products.each.with_index(1) do |product, expected_id|
      raise ArgumentError, "product ids must be 1..#{@products.size}" unless product.id == expected_id
    end
    @products.freeze

    @product_count_by_category = @products.map { |product| product.category.slug }.tally.freeze
    @sorted = SORTS.index_with { |sort| sort_products(@products, sort) }.freeze
    @sorted_by_category = @categories.to_h do |category|
      in_category = @products.select { |product| product.category.equal?(category) }
      [category.slug, SORTS.index_with { |sort| sort_products(in_category, sort) }.freeze]
    end.freeze
    @featured = @products.select(&:featured).freeze

    @orders = read_json(data_dir, "orders.json")
      .map { |row| Order.from_json(row) }
      .sort_by { |order| [-order.placed_at.to_i, -order.id] }
      .freeze
    @order_summary = OrderSummary.new(@orders)

    freeze
  end

  def category(slug)
    @category_by_slug[slug]
  end

  def product_count(category = nil)
    category ? @product_count_by_category.fetch(category.slug, 0) : @products.size
  end

  # Looks up a product by its id, or returns nil.
  def product(id)
    @products[id - 1] if id.between?(1, @products.size)
  end

  # The related products, in data order.
  def related(of)
    of.related_ids.filter_map { |id| product(id) }
  end

  # All products, or those of one category, in a sort order from `SORTS`.
  def sorted(category, sort)
    (category ? @sorted_by_category.fetch(category.slug) : @sorted).fetch(sort)
  end

  private

  def read_json(data_dir, name)
    JSON.parse(File.read(File.join(data_dir, name)), freeze: true)
  end

  # Ties are broken by ascending id.
  def sort_products(products, sort)
    products.sort_by do |product|
      case sort
      when "newest" then [-product.created_at.jd, product.id]
      when "price-asc" then [product.price_cents, product.id]
      when "price-desc" then [-product.price_cents, product.id]
      when "rating" then [-product.rating_tenths, -product.review_count, product.id]
      end
    end.freeze
  end
end
