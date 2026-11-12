class ProductsController < ApplicationController
  def index
    @listing = Listing.new(catalog, category: params[:category], sort: params[:sort], page: params[:page])
  end

  def show
    @product = params[:id].match?(/\A[0-9]+\z/) && catalog.product(params[:id].to_i)
    render_error :not_found unless @product
  end
end
