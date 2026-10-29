# Metadata

This example shows how to set page titles, descriptions, canonical URLs, and social link previews. The root layout provides defaults for all pages and calls `topcoat::metadata::tags()` to render them in `<head>`.

Each page can replace the fields it needs to change. The about page removes the default description. Product pages load their title, description, and preview image from the selected product.

Clone the repository and open its directory:

```sh
git clone https://github.com/tokio-rs/topcoat.git
cd topcoat
```

Start the example:

```sh
cargo topcoat dev -p metadata
```

Open the page source in your browser to see the generated tags.
