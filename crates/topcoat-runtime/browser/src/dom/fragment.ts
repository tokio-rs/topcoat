/**
 * Parses `html` as content of `parent`, so elements only allowed in that
 * context survive: a `<tr>` parsed for a `<tbody>` stays a row, where a
 * parse in the document's context would drop it and keep its text.
 */
export function parseChildren(parent: Node, html: string): Node[] {
	const range = document.createRange();
	range.selectNodeContents(parent);
	return Array.from(range.createContextualFragment(html).childNodes);
}
