// Reduces an HTML document (stdin) to its normalized visible text (stdout) so
// documents rendered by different frameworks can be diffed. Scripts, styles,
// comments, and tags are removed; basic entities are decoded; whitespace is
// collapsed. Comments are removed without inserting whitespace because React
// separates adjacent text nodes with comment markers.

import { readFileSync } from "node:fs";

let html = readFileSync(0, "utf8");

html = html.replace(/<script[\s\S]*?<\/script>/gi, " ");
html = html.replace(/<style[\s\S]*?<\/style>/gi, " ");
html = html.replace(/<!--[\s\S]*?-->/g, "");
html = html.replace(/<[^>]*>/g, " ");
html = html
  .replaceAll("&lt;", "<")
  .replaceAll("&gt;", ">")
  .replaceAll("&quot;", '"')
  .replaceAll("&#x27;", "'")
  .replaceAll("&#39;", "'")
  .replaceAll("&nbsp;", " ")
  .replaceAll("&amp;", "&");
html = html.replace(/\s+/g, " ").trim();

process.stdout.write(`${html}\n`);
