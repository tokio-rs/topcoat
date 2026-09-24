/**
 * Parses HTML using the parent element's rules. This keeps elements such
 * as table rows that would be discarded when parsed as ordinary body content.
 */
export function parseChildren(parent: Node, html: string): Node[] {
	const range = document.createRange();
	range.selectNodeContents(parent);
	return Array.from(range.createContextualFragment(html).childNodes);
}
