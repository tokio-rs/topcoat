# Deployment

A Topcoat app is a Rust executable with an asset bundle beside it. Build both together, copy both to the server, and run the executable. Rust and the Topcoat CLI are needed to build the app, not to run it.

The examples use an application binary named `my-app` and Cargo's default output directory.

## Build the application

From the project root, run:

```sh
cargo topcoat asset bundle --release --bin my-app
```

This builds the release executable and writes the bundle beside it under `target/release/`. Deploy the executable and the entire `assets/` directory from this same build. Do not pair a binary with a bundle produced by another build.

Use a Topcoat CLI compatible with the version in `Cargo.lock`. If the application uses Topcoat from Git, build the CLI from the same Git revision. See [Getting started](getting_started.md) for CLI installation and [Assets](asset.md) for custom output paths.

The executable must match the server's operating system and CPU architecture. Build in a compatible environment, especially for Linux. A Rust executable may use shared libraries, so install the libraries required by the application on the destination.

## Run the application

Applications that use bundled assets must load the bundle when building the router:

```rust,no_run
use topcoat::{
    asset::{AssetBundle, RouterBuilderAssetExt},
    router::{Router, RouterBuilderDiscoverExt},
};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let router = Router::builder()
        .discover()
        .assets(AssetBundle::load()?)
        .build();

    topcoat::start(router).await
}
```

`AssetBundle::load()` looks for `assets/` next to the running executable, not in the working directory. A missing or unreadable manifest returns an error. Test a page and its assets before sending traffic to a new release.

Run the release executable directly:

```sh
./my-app
```

With `topcoat::start`, the default listener is `127.0.0.1:3000`. Use `HOST=0.0.0.0` when a container or another machine must connect, and use the port assigned by the hosting environment when one is provided:

```sh
HOST=0.0.0.0 PORT=3000 ./my-app
```

Do not use `topcoat dev` as the production start command. `topcoat::start` handles Ctrl+C and, on Unix, SIGTERM. It stops accepting new connections and lets in-flight requests finish for up to 30 seconds by default. Configure the process manager to wait longer than 30 seconds before killing the process. If you change `RouterService::shutdown_timeout`, change the process manager timeout too.

## Linux VM with systemd

Copy the executable and bundle as one release, then unpack them into a new directory such as `/opt/my-app/releases/001`. Keep the release read-only to the service and make sure its user can execute the binary and read `assets/`.

Create `/etc/systemd/system/my-app.service`:

```ini
[Unit]
Description=My Topcoat app
After=network-online.target

[Service]
Type=exec
User=my-app
WorkingDirectory=/opt/my-app/current
ExecStart=/opt/my-app/current/my-app
Environment=HOST=127.0.0.1
Environment=PORT=3000
Restart=on-failure
TimeoutStopSec=45s

[Install]
WantedBy=multi-user.target
```

Point `/opt/my-app/current` at the release directory and start the service:

```sh
sudo systemctl daemon-reload
sudo systemctl enable --now my-app.service
sudo journalctl -u my-app.service -f
```

## Multi-stage Docker build

Build the executable and bundle inside the image build. Pass build arguments that match the Rust toolchain and Topcoat CLI used by the application rather than hardcoding project versions in this guide.

```dockerfile
ARG RUST_IMAGE=rust:<matching-rust-version>-bookworm
FROM ${RUST_IMAGE} AS build
ARG TOPCOAT_CLI_VERSION=<matching-topcoat-cli-version>

WORKDIR /src
COPY . .
RUN cargo install topcoat-cli --version "=${TOPCOAT_CLI_VERSION}" --locked
RUN cargo topcoat asset bundle --release --bin my-app

FROM debian:bookworm-slim
WORKDIR /app
COPY --from=build /src/target/release/my-app ./my-app
COPY --from=build /src/target/release/assets ./assets
ENV HOST=0.0.0.0 PORT=3000
EXPOSE 3000
STOPSIGNAL SIGTERM
ENTRYPOINT ["/app/my-app"]
```

Choose a runtime image compatible with the executable and install its required shared libraries. The image does not make the executable architecture-independent. Run the container with a stop timeout longer than 30 seconds:

```sh
docker build --build-arg RUST_IMAGE=rust:<matching-rust-version>-bookworm --build-arg TOPCOAT_CLI_VERSION=<matching-topcoat-cli-version> -t my-app:release .
docker run --rm --stop-timeout 45 --publish 127.0.0.1:3000:3000 my-app:release
```

## Reverse proxy and TLS

Keep the application on a private listener and terminate TLS at a reverse proxy or hosting platform. Forward paths unchanged, including `/_topcoat/`, and preserve the public `Host`, `Origin`, and `Sec-Fetch-Site` headers. For streamed responses or WebSockets, configure and test proxy buffering, upgrades, and timeouts.

## Health checks

Add a simple route and register it through `.discover()` or explicitly with `.route(health)`:

```rust
use topcoat::{Result, router::route};

#[route(GET "/healthz")]
async fn health() -> Result<&'static str> {
    Ok("ok")
}
```

Check it with `curl --fail http://127.0.0.1:3000/healthz`. This checks HTTP serving only. Test a rendered page and its assets separately, and add a readiness check if traffic also depends on a database or another service.

## Updating and rollback

Install each release in a new directory or publish it as a new image tag or digest. Keep the executable and its matching `assets/` directory together. For a VM, switch the `current` symlink and restart the service; keep the previous directory until the new release passes its checks.

```sh
sudo ln -sfnT releases/002 /opt/my-app/current.next
sudo mv -Tf /opt/my-app/current.next /opt/my-app/current
sudo systemctl restart my-app.service
```

Rollback by switching the symlink back and restarting, or by replacing the container with the previous image reference. This replaces one process with another and does not roll back database changes.
