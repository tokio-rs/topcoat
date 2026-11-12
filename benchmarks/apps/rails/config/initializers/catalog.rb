# Load the dataset and build its indexes while the app boots, not on the first request. With
# Puma's `preload_app!` this happens once, before the workers fork.
Rails.application.config.after_initialize do
  Catalog.instance
end
