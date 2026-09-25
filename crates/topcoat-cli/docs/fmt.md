`topcoat fmt` formats Topcoat macro bodies in Rust source files. Run it alongside `rustfmt`, which formats the surrounding Rust code.

# Running the formatter

Run the formatter with:

```sh
topcoat fmt
```

You can also run `cargo topcoat fmt`. Use `topcoat fmt` in editors to avoid starting Cargo for each format request.

With no file arguments, the command scans Rust files under the current directory and writes changes in place.

```sh
topcoat fmt src/main.rs src/app
```

Pass file or directory paths to limit the files to format. The command searches directories recursively for Rust files.

For editor integrations and other tools, use stdin/stdout mode:

```sh
topcoat fmt --stdin < src/main.rs > /tmp/main.rs
```

In stdin mode, the formatted source is written to stdout instead of updating files on disk.

By default, the formatter handles all supported macros. Pass a comma-separated list with `--macros` to format only those macros:

```sh
topcoat fmt --macros view,class
```

# Supported syntax

The formatter changes only supported macro bodies. For example, it formats the HTML inside `view!`:

```rust
use topcoat::{router::page, view::{View, view}};

#[page("/")]
async fn page() -> topcoat::Result<impl View> {
    Ok(view! {
        <main>
            <h1>"Hello"</h1>
        </main>
    })
}
```

Macros are identified by their names at the call site. A macro imported or re-exported under a different name will not be formatted.

# Editor integration

## Neovim

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
