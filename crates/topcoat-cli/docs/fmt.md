`topcoat fmt` formats Topcoat macro bodies in Rust files. Run it alongside `rustfmt`, which formats the surrounding Rust code.

# Format files

Run the formatter with:

```sh
topcoat fmt
```

You can also invoke it as `cargo topcoat fmt`.

With no arguments, the command formats Rust files under the current directory in place. To choose files or directories:

```sh
topcoat fmt src/main.rs src/app
```

File arguments can point at individual files or directories. Directories are expanded recursively to Rust files.

For editor integrations and other tools, use stdin/stdout mode:

```sh
topcoat fmt --stdin < src/main.rs > /tmp/main.rs
```

In stdin mode, the formatted source is written to stdout instead of updating files on disk.

Use `--macros` to format only the named macros:

```sh
topcoat fmt --macros view,class
```

# Supported syntax

The formatter recognizes macros by name and formats their bodies. For example, it formats the HTML inside `view!`:

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

Renaming a macro at the call site prevents the formatter from recognizing it.

# Editor integration

## Neovim

This `conform.nvim` configuration runs `topcoat fmt` after the Rust language server's formatter. It enables Topcoat formatting only in projects with a `Topcoat.toml` marker file:

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
