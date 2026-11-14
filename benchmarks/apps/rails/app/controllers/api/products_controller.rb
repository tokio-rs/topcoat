class Api::ProductsController < Api::BaseController
  def index
    listing = Listing.new(catalog, category: params[:category], sort: params[:sort], page: params[:page])
    render json: {
      items: listing.products.map(&:summary_json),
      page: listing.page,
      page_count: listing.page_count,
      total: listing.total
    }
  end

  def show
    product = params[:id].match?(/\A[0-9]+\z/) && catalog.product(params[:id].to_i)
    if product
      render json: product
    else
      render_error "product not found", status: :not_found
    end
  end
end
