Rails.application.routes.draw do
  # The readiness probe of the benchmark runner.
  get "health" => "health#show"

  root "home#index"
  get "about" => "pages#about"
  resources :products, only: %i[index show], format: false

  namespace :admin do
    resources :orders, only: :index, format: false
  end

  namespace :api, format: false, defaults: { format: :json } do
    resources :products, only: %i[index show]
    resource :cart, only: :create
  end

  # Everything else, including unknown paths under /api, gets the HTML 404 page.
  match "*path", to: "errors#not_found", via: :all, format: false
end
