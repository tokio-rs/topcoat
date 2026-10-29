# Metadata

This example sets page metadata such as titles, descriptions, and canonical URLs. The root layout sets defaults for every page and renders them in `<head>` with `metadata::tags()`. Pages override single fields, the about page removes the description, and product pages load their title and description from the product they show.

Clone the Topcoat repository:

```sh
git clone https://github.com/tokio-rs/topcoat.git
cd topcoat
```

Then run it with:

```sh
cargo topcoat dev -p metadata
```

View the page source to see the rendered tags.
