{
  description = "Topcoat development shell";

  inputs = {
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = { fenix, nixpkgs, ... }:
    let
      systems = [
        "aarch64-darwin"
        "aarch64-linux"
        "x86_64-darwin"
        "x86_64-linux"
      ];
      forAllSystems = nixpkgs.lib.genAttrs systems;
    in
    {
      devShells = forAllSystems (system:
        let
          pkgs = import nixpkgs { inherit system; };
          rustVersion = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).workspace.package.rust-version;
          msrv =
            if builtins.length (pkgs.lib.splitString "." rustVersion) == 2
            then "${rustVersion}.0"
            else rustVersion;
          # Hash of the channel manifest, shared by host and cross targets.
          toolchainSpec = {
            channel = msrv;
            sha256 = "sha256-zm3dyIY2T414ZRR3EhLOvptzG6gta4WZUcawzMUWtqI=";
          };
          rustToolchain = (fenix.packages.${system}.toolchainOf toolchainSpec).withComponents [
            "cargo"
            "clippy"
            "rust-analyzer"
            "rust-src"
            "rustc"
            "rustfmt"
          ];
          # The MSRV toolchain plus the wasm32 standard library for hydrated
          # benchmark apps (Leptos, Dioxus).
          benchmarkRustToolchain = fenix.packages.${system}.combine [
            rustToolchain
            (fenix.packages.${system}.targets.wasm32-unknown-unknown.toolchainOf toolchainSpec).rust-std
          ];
        in
        {
          default = pkgs.mkShell {
            packages = [ rustToolchain ];
            hardeningDisable = [ "fortify" ];
            RUST_SRC_PATH = "${rustToolchain}/lib/rustlib/src/rust/library";
          };

          # Toolchains for the cross-framework benchmark suite in `benchmarks/`.
          benchmarks = pkgs.mkShell {
            packages = with pkgs; [
              benchmarkRustToolchain
              binaryen
              cargo-leptos
              dioxus-cli
              # Leptos and Dioxus require a CLI matching the `wasm-bindgen`
              # crate exactly; cargo-leptos and dx download their own when the
              # versions differ.
              wasm-bindgen-cli

              oha

              nodejs_24
              pnpm
              tailwindcss_4

              # The Bundler shipped with Ruby 3.4.9 warns against its newer
              # RubyGems, so the standalone Bundler is listed first to shadow
              # it. Rails gems build native extensions during `bundle install`.
              (bundler.override { ruby = ruby_3_4; })
              ruby_3_4
              libyaml
              libffi
              zlib

              dotnet-sdk_10

              go
              templ

              python313
              uv

              beamPackages.elixir
              beamPackages.erlang
              # Phoenix compiles its rebar3 dependencies with this binary
              # (`MIX_REBAR3`) instead of a copy installed into `MIX_HOME`.
              beamPackages.rebar3

              curl
              git
              libiconv
              openssl
              pkg-config
            ];
            hardeningDisable = [ "fortify" ];
            RUST_SRC_PATH = "${benchmarkRustToolchain}/lib/rustlib/src/rust/library";
            DOTNET_CLI_TELEMETRY_OPTOUT = "1";
            DOTNET_NOLOGO = "1";
            DOTNET_ROOT = "${pkgs.dotnet-sdk_10}/share/dotnet";
            NEXT_TELEMETRY_DISABLED = "1";
            shellHook = ''
              echo "Topcoat benchmark shell"
            '';
          };
        });
    };
}
