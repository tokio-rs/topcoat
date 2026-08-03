# Deferred view example

This example sends a dashboard shell immediately, then streams its activity and recommendations as ordinary views finish.

Run it through the Topcoat development server so the external patch helper is bundled and served:

```sh
cargo topcoat dev --package deferred-view
```

Open <http://localhost:3000>.
