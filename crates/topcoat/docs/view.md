HTML templating: the macros and types for rendering views and building components.

- [`view!`]: the HTML-like templating macro.
- [`#[component]`](macro@component): turns an async function into a reusable component with typed props and child content.
- [`attributes!`]: builds an [`Attributes`] value at runtime, using the same attribute syntax as [`view!`].
- [`class!`]: builds a space-separated class list from static and conditional entries.
- [`live!`] and [`emit!`]: live regions that stream replacement content into a page after it reached the browser.
- [`suspense`] and [`error_boundary`]: components that show a fallback in place of child content that is still loading or that failed to render.

[`view!`]: macro@view
[`attributes!`]: macro@attributes
[`class!`]: macro@class
[`live!`]: macro@live
[`emit!`]: macro@emit
