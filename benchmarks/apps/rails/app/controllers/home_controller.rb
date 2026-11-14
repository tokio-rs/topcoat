class HomeController < ApplicationController
  def index
    @categories = catalog.categories
    @featured = catalog.featured
  end
end
