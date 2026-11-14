class Admin::OrdersController < ApplicationController
  def index
    @orders = catalog.orders
    @summary = catalog.order_summary
  end
end
