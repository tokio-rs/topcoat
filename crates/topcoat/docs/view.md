This module provides Topcoat's HTML templating primitives:

- [`view!`]: the HTML-like templating macro.
- [`#[component]`][`component`]: turns an async function into a reusable component with typed props and child content.
- [`attributes!`]: builds a reusable runtime [`Attributes`] value from the same attribute syntax used inside [`view!`].
- [`class!`]: space-separated class lists from static and conditional entries.

[`view!`]: macro.view.html
[`component`]: attr.component.html
[`attributes!`]: macro.attributes.html
[`Attributes`]: struct.Attributes.html
[`class!`]: macro.class.html
