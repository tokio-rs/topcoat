defmodule Harbor.Application do
  # See https://elixir.hexdocs.pm/Application.html
  # for more information on OTP Applications
  @moduledoc false

  use Application

  @impl true
  def start(_type, _args) do
    # The read-only dataset is loaded into `:persistent_term` before the endpoint starts
    # accepting requests.
    data_dir = Application.fetch_env!(:harbor, :data_dir)
    :ok = Harbor.Catalog.load!(data_dir)
    :ok = Harbor.Sales.load!(data_dir)

    children = [
      HarborWeb.Telemetry,
      {DNSCluster, query: Application.get_env(:harbor, :dns_cluster_query) || :ignore},
      {Phoenix.PubSub, name: Harbor.PubSub},
      # Start to serve requests, typically the last entry
      HarborWeb.Endpoint
    ]

    # See https://elixir.hexdocs.pm/Supervisor.html
    # for other strategies and supported options
    opts = [strategy: :one_for_one, name: Harbor.Supervisor]
    Supervisor.start_link(children, opts)
  end

  # Tell Phoenix to update the endpoint configuration
  # whenever the application is updated.
  @impl true
  def config_change(changed, _new, removed) do
    HarborWeb.Endpoint.config_change(changed, removed)
    :ok
  end
end
