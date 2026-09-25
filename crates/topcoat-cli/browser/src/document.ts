import { morph } from "../../../topcoat-core/browser/morph";

/** Script, base URL and doctype changes need a fresh document. */
export function canMorph(next: Document): boolean {
	const scripts = (doc: Document) =>
		JSON.stringify(Array.from(doc.scripts, (script) => script.outerHTML));
	const doctype = (doc: Document) => {
		const type = doc.doctype;
		return JSON.stringify(type && [type.name, type.publicId, type.systemId]);
	};
	return (
		scripts(document) === scripts(next) &&
		doctype(document) === doctype(next) &&
		document.querySelector("base")?.outerHTML ===
			next.querySelector("base")?.outerHTML
	);
}

/** Updates the full document, retaining matching elements and form state. */
export function morphDocument(next: Document): void {
	const root = document.documentElement;
	const fresh = next.documentElement;
	const { scrollX, scrollY } = window;
	for (const attr of Array.from(root.attributes)) {
		if (!fresh.hasAttribute(attr.name)) root.removeAttribute(attr.name);
	}
	for (const attr of Array.from(fresh.attributes)) {
		root.setAttribute(attr.name, attr.value);
	}
	morph(root, null, null, fresh.childNodes, { preserveFormState: true });
	window.scrollTo({ left: scrollX, top: scrollY, behavior: "instant" });
}
