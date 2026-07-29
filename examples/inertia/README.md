# Inertia

This example pairs Topcoat with React through the Inertia.js v3 protocol. Rust owns routes, prop resolution, validation, redirects, flash data, and asset versioning. React owns the page components and client visits.

It demonstrates:

- the required v3 JSON script bootstrap and empty mount element;
- configured `auth` sharing and the automatic `errors` prop;
- ordinary and partial page visits;
- deferred, rescued, merged, once, and infinite-scroll props;
- redirect-based validation with an error bag;
- page flash after a successful mutation;
- session origin verification for mutation requests;
- asset versions derived from content-hashed JavaScript and CSS.

## Build the frontend

The frontend uses Yarn and pins exact Inertia, React, and Vite versions. From the repository root:

```sh
cd examples/inertia/frontend
yarn install --frozen-lockfile
yarn build
cd ../../..
```

Vite eagerly imports every page and emits only `assets/app.js` and `assets/app.css`. Those built files are committed so the Rust asset declarations always have source inputs. Rebuild and commit them whenever frontend source or dependencies change.

During frontend development, keep Vite's watch build running in a separate terminal. It rebuilds the committed asset inputs when JSX or CSS changes, and `topcoat dev` then bundles them and reloads the application:

```sh
cd examples/inertia/frontend
yarn build --watch
```

## Run the application

Start the example from the repository root:

```sh
cargo topcoat dev --package inertia
```

Open `http://127.0.0.1:3000`.

The example uses a stable development cookie key when `TOPCOAT_COOKIE_KEY` is absent. Set a secret of at least 32 bytes in production and keep it persistent across restarts and application instances:

```sh
TOPCOAT_COOKIE_KEY='replace-this-with-a-long-random-production-secret' \
    cargo topcoat dev --package inertia
```

The flash cookie has `Secure` disabled because this example runs over local plain HTTP. Production applications should keep [`CookieFlashStore`](https://docs.rs/topcoat/latest/topcoat/inertia/struct.CookieFlashStore.html)'s secure default.

## Try the protocol features

Open the Users page and inspect the browser network panel. The initial document contains `<script data-page="inertia-app" type="application/json">` and `<div id="inertia-app"></div>` with no legacy `data-page` attribute on the mount element.

Use "Reload only stats" and watch the stats resolution number change without reloading the other props. Deferred stats and activity load after the first render. Navigation is a once prop with a ten-minute expiry; "Refresh once navigation" explicitly requests it and changes its separate resolution number without changing the URL or user list. The user counter starts with one page of eight users. Scroll inside the fixed-height list to load more pages without changing the visible URL.

Open Optimistic updates and submit its name-and-age form. The page has its own list, separate from the paginated Users data. A temporary saving row appears immediately, and the server response replaces it with the persisted row. The example endpoint waits briefly so the optimistic state is easy to see. Submit a one-character name or an invalid age to see the temporary row roll back and the validation errors appear.

Submit a one-character name on the Create user page. The server stores validation errors in the private flash cookie, redirects with 303, and returns them under the `createUser` error bag. A valid name is added to the example's in-memory user store, redirects to Users, appears at the start of the list, and displays one-time page flash until you leave the Users page. Background partial reloads do not make the notice disappear. Restarting the example resets the list.
