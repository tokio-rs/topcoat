# Session

A login and logout flow with session records stored in memory.

Clone the Topcoat repository:

```sh
git clone https://github.com/tokio-rs/topcoat.git
cd topcoat
```

Then run it with:

```sh
cargo topcoat dev -p session
```

The example accepts any username without verifying credentials; it only shows the session lifecycle.
