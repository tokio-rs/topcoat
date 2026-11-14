# One page of the product listing, shared by the HTML page and the JSON API.
#
# Parameters are validated in order (category, sort, page) and the first failure raises a
# `Listing::Error` that carries its HTTP status and API message. Empty values count as absent;
# values that are not strings (`?page[]=1`) are an invalid query.
class Listing
  PAGE_SIZE = 24
  DEFAULT_SORT = "newest"

  class Error < StandardError
    attr_reader :status

    def initialize(status, message)
      @status = status
      super(message)
    end
  end

  attr_reader :category, :sort, :page, :page_count, :total, :products

  def initialize(catalog, category: nil, sort: nil, page: nil)
    unless [category, sort, page].all? { |value| value.nil? || value.is_a?(String) }
      raise Error.new(:unprocessable_content, "invalid query")
    end

    category, sort, page = [category, sort, page].map { |value| value unless value&.empty? }

    if category
      @category = catalog.category(category) or raise Error.new(:not_found, "category not found")
    end

    @sort = sort || DEFAULT_SORT
    raise Error.new(:unprocessable_content, "invalid sort") unless Catalog::SORTS.include?(@sort)

    sorted = catalog.sorted(@category, @sort)
    @total = sorted.size
    @page_count = [(@total + PAGE_SIZE - 1) / PAGE_SIZE, 1].max
    @page = parse_page(page)
    raise Error.new(:not_found, "page not found") if @page > @page_count

    @products = sorted.slice((@page - 1) * PAGE_SIZE, PAGE_SIZE)
  end

  def empty? = @total.zero?

  # 1-based positions of the first and last product on this page.
  def first_position = (@page - 1) * PAGE_SIZE + 1
  def last_position = (@page - 1) * PAGE_SIZE + @products.size

  private

  # Digits only and not zero. Numbers too large for any page are past the last page.
  def parse_page(value)
    return 1 if value.nil?
    raise Error.new(:unprocessable_content, "invalid page") unless value.match?(/\A[0-9]+\z/)

    value.to_i.tap do |page|
      raise Error.new(:unprocessable_content, "invalid page") if page.zero?
    end
  end
end
