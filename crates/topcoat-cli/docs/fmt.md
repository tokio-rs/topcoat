# Source code formatting

Topcoat includes a source formatter for macro bodies in Rust files. It is intended to run alongside normal Rust formatting: `rustfmt` formats Rust syntax, while `topcoat fmt` formats the syntax inside Topcoat-aware macro invocations.

## CLI

Run the formatter with:

```sh
topcoat fmt
```

Prefer the direct `topcoat fmt` command, especially from editors and other frequently-run integrations. `cargo topcoat fmt` also works, but it goes through Cargo's command dispatch path and adds unnecessary startup overhead.

With no file arguments, the command scans Rust files under the current directory and writes changes in place.

```sh
topcoat fmt src/main.rs src/app
```

File arguments can point at individual files or directories. Directories are expanded recursively to Rust files.

For editor integrations and other tools, use stdin/stdout mode:

```sh
topcoat fmt --stdin < src/main.rs > /tmp/main.rs
```

In stdin mode, the formatted source is written to stdout instead of updating files on disk.

## What It Formats

The formatter parses Rust source, finds macro invocations it knows how to format, and replaces only those macro bodies. The surrounding Rust code is left as-is.

In normal Topcoat code, this primarily means `view!`:

```rust
fn page() -> topcoat::Result {
    view! {
        <main>
            <h1>"Hello"</h1>
        </main>
    }
}
```

The formatter is macro-aware rather than token-only, so the same command is also the place for other supported Topcoat macro syntax as formatting support grows. Unknown macros are ignored.

## Editor integration

### Neovim

This Neovim config uses `conform.nvim` and enables `topcoat fmt` for Rust buffers only when a `Topcoat.toml` marker exists in the project root.

```lua
require("conform").setup({
	formatters = {
		topcoat = {
			command = "topcoat",
			args = { "fmt", "--stdin" },
			require_cwd = true,
			cwd = function(self, ctx)
				return require("conform.util").root_file({ "Topcoat.toml" })(self, ctx)
			end,
		},
	},
	formatters_by_ft = {
		rust = { "topcoat", lsp_format = "first" },
	},
})
```

Create a `Topcoat.toml` marker at the root of a Topcoat project to opt in:

```sh
touch Topcoat.toml
```
