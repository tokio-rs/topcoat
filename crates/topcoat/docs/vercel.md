Run a Topcoat application as a streaming Rust function on [Vercel](https://vercel.com/). The `topcoat-vercel` CLI builds the application using Vercel's Build Output API.

# Setup

Enable the `vercel` feature on the `topcoat` dependency:

```toml
[dependencies]
topcoat = { version = "0.5", features = ["vercel"] }
```

Keep using [`topcoat::start`](crate::start) in the application's existing binary. With the `vercel` feature enabled, it uses the normal Topcoat server locally and the Vercel runtime when deployed:

```rust,no_run
use topcoat::router::{Router, RouterBuilderDiscoverExt};

#[tokio::main]
async fn main() {
    let router = Router::builder().discover().build();
    topcoat::start(router).await.unwrap();
}
```

Install the deployment CLI and initialize the project:

```console
$ cargo install topcoat-vercel
$ topcoat-vercel init
```

If the package contains several binaries, select the application binary during setup:

```console
$ topcoat-vercel init --bin my-app
```

The command creates `vercel.json`, pins the supported Rust toolchain when the project does not already select one, and ignores generated deployment output. Existing configuration files are not replaced unless `--force` is passed.

# Deploy

Deploy the project with the Vercel CLI or a connected Git repository. Vercel installs the matching `topcoat-vercel` release and runs `topcoat-vercel build`:

```console
$ vercel deploy
```

The build writes a Build Output API v3 deployment to `.vercel/output`. The Topcoat application becomes a streaming Rust function, while bundled assets are copied into Vercel's static file output and also packaged beside the executable so [`AssetBundle::load`](crate::asset::AssetBundle::load) works at startup.

`topcoat-vercel build` currently produces a Linux x86-64 executable and must run in Vercel's Linux build environment. Native builds from other platforms fail with an explanation instead of producing an artifact that cannot run on Vercel.
