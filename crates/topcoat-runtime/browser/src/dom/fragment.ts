/**
 * Parses HTML using the parent element's rules. This keeps elements such
 * as table rows that would be discarded when parsed as ordinary body content.
 *
 * The parse runs on a detached element like the parent, which the fragment
 * parser takes as its context.
 */
export function parseChildren(parent: Node, html: string): Node[] {
	const context =
		parent instanceof Element
			? document.createElementNS(parent.namespaceURI, parent.localName)
			: document.createElement("body");
	context.innerHTML = html;
	return Array.from(context.childNodes);
}
