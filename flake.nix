{
  description = "Topcoat CLI and development shells";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    crane.url = "github:ipetkov/crane";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    inputs@{
      flake-parts,
      crane,
      fenix,
      ...
    }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [
        "aarch64-darwin"
        "aarch64-linux"
        "x86_64-darwin"
        "x86_64-linux"
      ];

      perSystem =
        {
          pkgs,
          system,
          lib,
          ...
        }:
        let
          workspacePackage = (lib.importTOML ./Cargo.toml).workspace.package;
          rustVersion = workspacePackage.rust-version;
          msrv =
            if builtins.length (lib.splitString "." rustVersion) == 2 then "${rustVersion}.0" else rustVersion;
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
          # Fetch the same dated manifest on the evaluating host for every system.
          nightlyManifest = builtins.fetchurl {
            url = "https://static.rust-lang.org/dist/2026-11-20/channel-rust-nightly.toml";
            sha256 = "sha256-axBIOyanKDHAdB/h6wkf3l1+DCa6/WAv4+vVKvZH684=";
          };
          nightlyRustToolchain = (fenix.packages.${system}.fromManifestFile nightlyManifest).withComponents [
            "cargo"
            "rustc"
            "rustfmt"
          ];
          # Fenix supplies Cargo directly, so the dev shell dispatches +nightly itself.
          devCargo = pkgs.writeShellScriptBin "cargo" ''
            if [[ "''${1-}" == "+nightly" ]]; then
              shift
              export PATH="${nightlyRustToolchain}/bin:$PATH"
              exec ${nightlyRustToolchain}/bin/cargo "$@"
            fi
            exec ${rustToolchain}/bin/cargo "$@"
          '';

          # The MSRV toolchain plus the wasm32 standard library for hydrated
          # benchmark apps (Leptos, Dioxus).
          benchmarkRustToolchain = fenix.packages.${system}.combine [
            rustToolchain
            (fenix.packages.${system}.targets.wasm32-unknown-unknown.toolchainOf toolchainSpec).rust-std
          ];

          craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;
          # Keep the browser bundle, templates, and other files embedded by the CLI.
          src = lib.cleanSource ./.;

          commonArgs = {
            inherit src;
            pname = "topcoat-cli";
            version = workspacePackage.version;
            cargoExtraArgs = "--locked --package topcoat-cli";
            strictDeps = true;
          };

          cargoArtifacts = craneLib.buildDepsOnly commonArgs;

          topcoat-cli = craneLib.buildPackage (
            commonArgs
            // {
              inherit cargoArtifacts;
              nativeCheckInputs = [ pkgs.gitMinimal ];
              meta.mainProgram = "topcoat";
            }
          );
        in
        {
          packages = {
            inherit topcoat-cli;
            default = topcoat-cli;
          };

          devShells = {
            default = pkgs.mkShell {
              packages = with pkgs; [
                devCargo
                rustToolchain
                cargo-hack
                cargo-udeps
                # The v8 build script needs a downloader for the prebuilt archive.
                curl
                gitMinimal
                nodejs_22
                (yarn.override { nodejs = nodejs_22; })
              ];
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
          };
        };
    };
}
