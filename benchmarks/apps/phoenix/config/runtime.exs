import Config

# config/runtime.exs is executed for all environments, including
# during releases. It is executed after compilation and before the
# system starts, so it is typically used to load production configuration
# and secrets from environment variables or elsewhere. Do not define
# any compile-time configuration in here, as it won't be applied.
# The block below contains prod specific runtime configuration.

# ## Using releases
#
# If you use `mix release`, you need to explicitly enable the server
# by passing the PHX_SERVER=true when you start it:
#
#     PHX_SERVER=true bin/harbor start
#
# Alternatively, you can use `mix phx.gen.release` to generate a `bin/server`
# script that automatically sets the env var above.
if System.get_env("PHX_SERVER") do
  config :harbor, HarborWeb.Endpoint, server: true
end

port = String.to_integer(System.get_env("PORT", "4000"))

# Bandit compresses responses for clients that accept it unless told otherwise, so the
# benchmark switch maps to its `compress` option.
compress =
  case System.get_env("BENCH_COMPRESSION", "off") do
    "on" -> true
    "off" -> false
    other -> raise "invalid BENCH_COMPRESSION #{inspect(other)}, expected on or off"
  end

config :harbor, HarborWeb.Endpoint,
  http: [ip: {127, 0, 0, 1}, port: port, http_options: [compress: compress]]

# The JSON dataset loaded at startup; defaults to `benchmarks/data` next to this app.
config :harbor,
       :data_dir,
       System.get_env("BENCH_DATA_DIR") || Path.expand("../../../data", __DIR__)

if config_env() == :dev do
  # Reload browser tabs when matching files change.
  config :harbor, HarborWeb.Endpoint,
    live_reload: [
      web_console_logger: true,
      patterns: [
        # Static assets, except user uploads
        ~r"priv/static/(?!uploads/).*\.(js|css|png|jpeg|jpg|gif|svg)$",
        # Router, Controllers, LiveViews and LiveComponents
        ~r"lib/harbor_web/router\.ex$",
        ~r"lib/harbor_web/(controllers|live|components)/.*\.(ex|heex)$"
      ]
    ]
end

if config_env() == :prod do
  # The secret key base is used to sign/encrypt cookies and other secrets.
  # A default value is used in config/dev.exs and config/test.exs but you
  # want to use a different value for prod and you most likely don't want
  # to check this value into version control, so we use an environment
  # variable instead.
  secret_key_base =
    System.get_env("SECRET_KEY_BASE") ||
      raise """
      environment variable SECRET_KEY_BASE is missing.
      You can generate one by calling: mix phx.gen.secret
      """

  host = System.get_env("PHX_HOST") || "localhost"

  config :harbor, :dns_cluster_query, System.get_env("DNS_CLUSTER_QUERY")

  config :harbor, HarborWeb.Endpoint,
    url: [host: host, port: port, scheme: "http"],
    secret_key_base: secret_key_base
end
