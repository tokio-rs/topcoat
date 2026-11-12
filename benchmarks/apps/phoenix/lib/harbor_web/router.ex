defmodule HarborWeb.Router do
  use HarborWeb, :router

  pipeline :browser do
    plug :accepts, ["html"]
    plug :fetch_session
    plug :fetch_live_flash
    plug :put_root_layout, html: {HarborWeb.Layouts, :root}
    plug :protect_from_forgery
    plug :put_secure_browser_headers
    plug HarborWeb.CartCount
  end

  pipeline :api do
    plug :accepts, ["json"]
  end

  # The readiness probe, outside any pipeline.
  get "/health", HarborWeb.HealthController, :show

  scope "/", HarborWeb do
    pipe_through :browser

    get "/", PageController, :home
    get "/about", PageController, :about
    get "/products", ProductController, :index
    get "/products/:id", ProductController, :show
    get "/admin/orders", OrderController, :index
  end

  scope "/api", HarborWeb.API do
    pipe_through :api

    get "/products", ProductController, :index
    get "/products/:id", ProductController, :show
    post "/cart", CartController, :create
  end
end
