# Creating an app

`topcoat new` creates a Topcoat application with the features you choose. For a walkthrough from installation to your first edit, see [Getting started](https://github.com/tokio-rs/topcoat/blob/main/docs/getting_started.md).

## Use the wizard

Run the command with a destination directory:

```sh
topcoat new my-app
```

The wizard asks which features to include, then generates the app and prints the commands to run next. Without a directory argument, it also asks where to create the app.

The destination must not already exist, even if it is empty. The Cargo package name defaults to the directory name. Use `--name` to choose a different package name:

```sh
topcoat new apps/web --name my-app
```

The command initializes a Git repository unless the destination is inside an existing repository. Pass `--no-git` to skip Git initialization.

## Use a preset

Presets answer all feature questions without prompting. Supply the destination on the command line:

```sh
topcoat new my-app --recommended
```

`--recommended` includes module routing, Toasty with SQLite, Topcoat's browser runtime, Tailwind CSS, Topcoat UI, Iconify icons, a Fontsource font, and a todo example.

For a smaller starting point:

```sh
topcoat new my-app --minimal
```

`--minimal` includes module routing and a plain CSS stylesheet, without a database, browser interaction library, icons, fonts, UI components, or the todo example.

## Customize features

Individual flags override a preset. Without a preset, the wizard asks about choices you have not supplied.

For example, keep the recommended setup but use in-memory storage for the todo example:

```sh
topcoat new my-app --recommended --database none
```

Or create a minimal app with Topcoat's browser runtime:

```sh
topcoat new my-app --minimal --interaction topcoat
```

The main feature options are:

| Option | Choices |
| --- | --- |
| `--routing` | `module`, `discover`, `manual` |
| `--database` | `toasty`, `none` |
| `--interaction` | `topcoat`, `htmx`, `datastar`, `alpine-ajax`, `none` |
| `--tailwind` / `--no-tailwind` | Tailwind CSS or a plain CSS stylesheet |
| `--icons` | `iconify`, `custom`, `none` |
| `--font` | `fontsource`, `none` |
| `--ui` / `--no-ui` | Include or omit Topcoat UI components |
| `--example` / `--no-example` | Include or omit the todo example |

Use `--icon-set` to choose an Iconify set and `--font-family` to choose a Fontsource family ID. These settings require their corresponding integrations. Toasty uses SQLite, which is also selectable with `--database-backend sqlite`.

Topcoat UI requires Tailwind and Iconify. When overriding a preset, update related choices together. For example, disabling Tailwind in the recommended preset also requires disabling UI components:

```sh
topcoat new my-app --recommended --no-tailwind --no-ui
```

Run `topcoat new --help` for all options and defaults.

## Use in scripts

Pass `--no-interactive` to ensure the command never asks questions. It fails if required choices are missing. A preset supplies all choices, and individual flags can override it:

```sh
topcoat new my-app --minimal --no-interactive --no-git
```

When no interactive terminal is available, the command also requires a destination and all feature choices. It does not silently choose a preset.

## Run the app

Change into the generated directory and follow the printed next steps. If the app includes both Toasty and the todo example, first create its database tables:

```sh
cd my-app
cargo run -- toasty migration generate
cargo run -- toasty migration apply
topcoat dev
```

For an app without database models, run `topcoat dev` directly from its directory. Open <http://127.0.0.1:3000> to view the app. The generated README explains its setup and project structure.
