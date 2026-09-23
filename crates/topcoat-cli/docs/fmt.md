`topcoat fmt` formats the bodies of Topcoat macros, like the HTML inside a `view!` invocation. `rustfmt` leaves macro bodies alone, so run both: `rustfmt` formats the Rust code, and `topcoat fmt` formats the code inside Topcoat macros.

# The CLI command

Run the formatter with:

```sh
topcoat fmt
```

`cargo topcoat fmt` works too, but Cargo's command dispatch makes it start more slowly. Prefer `topcoat fmt`, especially in editors and other tools that run it often.

Without file arguments, the command formats every Rust file under the current directory and writes the changes in place. You can also pass files and directories. Directories are searched recursively for Rust files.

```sh
topcoat fmt src/main.rs src/app
```

For editors and other tools, use `--stdin`. The command then reads source code from standard input and writes the formatted code to standard output instead of changing files on disk.

```sh
topcoat fmt --stdin < src/main.rs > /tmp/main.rs
```

By default the formatter handles every macro it supports. Pass `--macros` with a comma-separated list of macro names to format only those macros and leave the rest as they are. An unknown name is an error, and the error message lists the supported names.

```sh
topcoat fmt --macros view,class
```

# What it formats

The formatter parses the Rust source, finds the invocations of macros it supports, and rewrites only their bodies. The Rust code around them is left as it is.

In most Topcoat code, this means the HTML inside `view!` invocations gets formatted:

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

Many other Topcoat macros, such as `class!` and `font!`, are formatted as well.

The formatter recognizes a macro by the name used at the call site. If you import or re-export a macro under a different name, its invocations are no longer formatted.

# Editor integration

## Neovim

This Neovim config uses [`conform.nvim`](https://github.com/stevearc/conform.nvim). It runs `topcoat fmt` on Rust buffers, but only when a `Topcoat.toml` file exists at the project root.

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

To opt a project in, create an empty `Topcoat.toml` file at its root:

```sh
touch Topcoat.toml
```
