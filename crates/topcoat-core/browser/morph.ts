/**
 * Morphs a range of the document into new content, keeping every node it
 * can match so that focus, scroll position, and other state the browser
 * holds on an element survive a replacement.
 *
 * The algorithm is a port of idiomorph's, on a sibling range instead of an
 * element: old and new siblings are paired in order by node type and tag
 * name, an `id` is a hard identity that is never paired with a different
 * one, and an element containing an id that survives the morph is matched
 * to the element that contains the same id in the new content, so a
 * container is found again even when the siblings around it shifted. An
 * element with a surviving id that ended up somewhere else is moved into
 * place rather than recreated.
 *
 * Attributes and text are synced in place. The value of the focused element
 * is left alone, so a morph never disturbs what the user is typing.
 */

/** The ids present in both the old and the new content. */
type Ctx = {
	persistent: Set<string>;
	/**
	 * The persistent ids in each element's subtree, itself included, for the
	 * elements of both the old range and the new content. An element absent
	 * from the map contains no persistent id.
	 */
	idSets: Map<Node, Set<string>>;
	/** The old element carrying each persistent id. */
	oldById: Map<string, Element>;
	/** The focused element, which is never removed or moved. */
	active: Element | null;
};

/**
 * Morphs the children of `parent` strictly between `start` and `end` into
 * `content`. A `null` bound means the start or end of the parent, so
 * `morph(el, null, null, nodes)` morphs all of an element's children.
 *
 * The nodes in `content` are moved into the document where they are used,
 * so the caller should not rely on them afterwards.
 */
export function morph(
	parent: ParentNode,
	start: ChildNode | null,
	end: ChildNode | null,
	content: Iterable<Node>,
): void {
	const newNodes = Array.from(content);
	const oldNodes = rangeNodes(parent, start, end);
	const ctx = createContext(parent, oldNodes, newNodes);
	morphChildren(
		ctx,
		parent,
		start ? start.nextSibling : parent.firstChild,
		end,
		newNodes,
	);
}

function rangeNodes(
	parent: ParentNode,
	start: ChildNode | null,
	end: ChildNode | null,
): ChildNode[] {
	const nodes: ChildNode[] = [];
	let n = start ? start.nextSibling : parent.firstChild;
	while (n && n !== end) {
		nodes.push(n);
		n = n.nextSibling;
	}
	return nodes;
}

function createContext(
	parent: ParentNode,
	oldNodes: Node[],
	newNodes: Node[],
): Ctx {
	const oldIds = new Map<string, Element>();
	for (const node of oldNodes) {
		for (const el of elementsWithId(node)) oldIds.set(el.id, el);
	}
	const persistent = new Set<string>();
	for (const node of newNodes) {
		for (const el of elementsWithId(node)) {
			if (oldIds.has(el.id)) persistent.add(el.id);
		}
	}

	const idSets = new Map<Node, Set<string>>();
	const collect = (roots: Node[]) => {
		for (const root of roots) {
			for (const el of elementsWithId(root)) {
				if (!persistent.has(el.id)) continue;
				// The id belongs to the element and every ancestor up to the
				// root of the range, so a match at any level finds it.
				let current: Node | null = el;
				while (current) {
					let set = idSets.get(current);
					if (!set) {
						set = new Set();
						idSets.set(current, set);
					}
					set.add(el.id);
					if (current === root) break;
					current = current.parentNode;
				}
			}
		}
	};
	collect(oldNodes);
	collect(newNodes);

	const oldById = new Map<string, Element>();
	for (const id of persistent) {
		const el = oldIds.get(id);
		if (el) oldById.set(id, el);
	}

	return {
		persistent,
		idSets,
		oldById,
		active: parent.ownerDocument?.activeElement ?? null,
	};
}

/** The elements in `root`'s subtree, itself included, that have an id. */
function elementsWithId(root: Node): Element[] {
	if (!(root instanceof Element)) return [];
	const found = Array.from(root.querySelectorAll("[id]"));
	if (root.id) found.unshift(root);
	return found;
}

/**
 * Morphs the old siblings from `insertionPoint` up to `end` into
 * `newNodes`, both exclusive of `end`.
 */
function morphChildren(
	ctx: Ctx,
	parent: ParentNode,
	insertionPoint: ChildNode | null,
	end: ChildNode | null,
	newNodes: Node[],
): void {
	let cursor = insertionPoint;
	for (const newNode of newNodes) {
		if (cursor && cursor !== end) {
			const match = findBestMatch(ctx, newNode, cursor, end);
			if (match) {
				removeNodesBetween(cursor, match);
				cursor = morphNode(ctx, match, newNode).nextSibling;
				continue;
			}
		}

		// Nothing at the cursor matches. An element with a persistent id
		// exists somewhere in the old content, so bring it here instead of
		// creating a copy.
		if (newNode instanceof Element && ctx.persistent.has(newNode.id)) {
			const old = ctx.oldById.get(newNode.id);
			if (old) {
				moveBefore(parent, old, cursor);
				cursor = morphNode(ctx, old, newNode).nextSibling;
				continue;
			}
		}

		cursor = insertNode(ctx, parent, newNode, cursor).nextSibling;
	}

	while (cursor && cursor !== end) {
		const next: ChildNode | null = cursor.nextSibling;
		cursor.remove();
		cursor = next;
	}
}

/**
 * Finds the old sibling from `start` up to `end` that `newNode` should
 * morph into, or `null` to insert `newNode` instead.
 *
 * An id-set match wins outright. Otherwise the first soft match without
 * any persistent id is taken, unless the search would remove the focused
 * element or more persistent ids than `newNode` carries, or `newNode`'s
 * following siblings match the old nodes better, which means `newNode` was
 * inserted in front of them.
 */
function findBestMatch(
	ctx: Ctx,
	newNode: Node,
	start: ChildNode,
	end: ChildNode | null,
): ChildNode | null {
	let softMatch: ChildNode | null | undefined = null;
	let nextSibling = newNode.nextSibling;
	let siblingSoftMatches = 0;
	let displacedIds = 0;
	const nodeIds = ctx.idSets.get(newNode)?.size ?? 0;

	let cursor: ChildNode | null = start;
	while (cursor && cursor !== end) {
		if (isSoftMatch(ctx, cursor, newNode)) {
			if (isIdSetMatch(ctx, cursor, newNode)) return cursor;
			if (softMatch === null && !ctx.idSets.has(cursor)) {
				softMatch = cursor;
			}
		}

		if (
			softMatch === null &&
			nextSibling &&
			isSoftMatch(ctx, cursor, nextSibling)
		) {
			siblingSoftMatches++;
			nextSibling = nextSibling.nextSibling;
			if (siblingSoftMatches >= 2) softMatch = undefined;
		}

		if (ctx.active && cursor.contains(ctx.active)) break;
		displacedIds += ctx.idSets.get(cursor)?.size ?? 0;
		if (displacedIds > nodeIds) break;

		cursor = cursor.nextSibling;
	}

	return softMatch ?? null;
}

/**
 * Whether `oldNode` can morph into `newNode`: same node type and tag, and
 * not an element whose persistent id differs.
 */
function isSoftMatch(ctx: Ctx, oldNode: Node, newNode: Node): boolean {
	if (oldNode.nodeType !== newNode.nodeType) return false;
	if (oldNode instanceof Element && newNode instanceof Element) {
		if (oldNode.tagName !== newNode.tagName) return false;
		if (
			oldNode.id &&
			oldNode.id !== newNode.id &&
			ctx.persistent.has(oldNode.id)
		) {
			return false;
		}
	}
	return true;
}

/** Whether the two elements share a persistent id somewhere below them. */
function isIdSetMatch(ctx: Ctx, oldNode: Node, newNode: Node): boolean {
	const oldSet = ctx.idSets.get(oldNode);
	const newSet = ctx.idSets.get(newNode);
	if (!oldSet || !newSet) return false;
	for (const id of oldSet) {
		if (newSet.has(id)) return true;
	}
	return false;
}

/** Removes the siblings from `from` up to, not including, `to`. */
function removeNodesBetween(from: ChildNode, to: ChildNode): void {
	let cursor: ChildNode | null = from;
	while (cursor && cursor !== to) {
		const next: ChildNode | null = cursor.nextSibling;
		cursor.remove();
		cursor = next;
	}
}

/**
 * Morphs `oldNode` into `newNode` in place, or replaces it when the two
 * cannot be reconciled, returning whichever node ends up in the document.
 */
function morphNode(ctx: Ctx, oldNode: ChildNode, newNode: Node): ChildNode {
	if (oldNode instanceof Element && newNode instanceof Element) {
		if (oldNode.tagName === newNode.tagName) {
			const isActive = oldNode === ctx.active;
			syncAttributes(oldNode, newNode, isActive);
			morphChildren(
				ctx,
				oldNode,
				oldNode.firstChild,
				null,
				Array.from(newNode.childNodes),
			);
			// After the children, so a select sees its morphed options.
			syncProperties(oldNode, newNode, isActive);
			return oldNode;
		}
	} else if (oldNode.nodeType === newNode.nodeType) {
		// A text or comment node: only its data can differ.
		if (oldNode.nodeValue !== newNode.nodeValue) {
			oldNode.nodeValue = newNode.nodeValue;
		}
		return oldNode;
	}

	oldNode.replaceWith(newNode);
	return newNode as ChildNode;
}

/**
 * Inserts `newNode` before `before`. A node whose subtree carries a
 * persistent id is not inserted as is, because the id's old element must
 * move into it: an empty element of the same kind is inserted and morphed
 * into the new node instead.
 */
function insertNode(
	ctx: Ctx,
	parent: ParentNode,
	newNode: Node,
	before: ChildNode | null,
): ChildNode {
	if (newNode instanceof Element && ctx.idSets.has(newNode)) {
		const doc = parent.ownerDocument ?? document;
		const empty = doc.createElementNS(newNode.namespaceURI, newNode.localName);
		parent.insertBefore(empty, before);
		return morphNode(ctx, empty, newNode);
	}
	parent.insertBefore(newNode, before);
	return newNode as ChildNode;
}

/**
 * Moves `node` before `before` in `parent`, keeping its state where the
 * browser supports a state-preserving move.
 */
function moveBefore(
	parent: ParentNode,
	node: Node,
	before: ChildNode | null,
): void {
	const mover = parent as ParentNode & {
		moveBefore?: (node: Node, before: ChildNode | null) => void;
	};
	if (mover.moveBefore && node.isConnected) {
		try {
			mover.moveBefore(node, before);
			return;
		} catch {
			// The node cannot be moved in place, so fall through to a plain
			// insertion.
		}
	}
	parent.insertBefore(node, before);
}

/**
 * Syncs the attributes of `oldEl` to those of `newEl`. The value attribute
 * of the focused element is left alone, like its value property.
 */
function syncAttributes(
	oldEl: Element,
	newEl: Element,
	isActive: boolean,
): void {
	// An attribute without a namespace goes through the plain API: the
	// namespaced one rejects a name with a colon in it, like the runtime's
	// own `data-topcoat-on:click`, when no namespace is given.
	for (const attr of Array.from(newEl.attributes)) {
		if (isActive && attr.name === "value") continue;
		if (attr.namespaceURI === null) {
			if (oldEl.getAttribute(attr.name) !== attr.value) {
				oldEl.setAttribute(attr.name, attr.value);
			}
		} else if (
			oldEl.getAttributeNS(attr.namespaceURI, attr.localName) !== attr.value
		) {
			oldEl.setAttributeNS(attr.namespaceURI, attr.name, attr.value);
		}
	}
	for (const attr of Array.from(oldEl.attributes)) {
		if (isActive && attr.name === "value") continue;
		if (attr.namespaceURI === null) {
			if (!newEl.hasAttribute(attr.name)) oldEl.removeAttribute(attr.name);
		} else if (!newEl.hasAttributeNS(attr.namespaceURI, attr.localName)) {
			oldEl.removeAttributeNS(attr.namespaceURI, attr.localName);
		}
	}
}

/**
 * Brings the live form state of `oldEl` in line with `newEl`, because the
 * attribute is only the initial state once the user has touched a control.
 * The value of the focused element stays what the user typed.
 */
function syncProperties(
	oldEl: Element,
	newEl: Element,
	isActive: boolean,
): void {
	if (oldEl instanceof HTMLInputElement && newEl instanceof HTMLInputElement) {
		if (oldEl.checked !== newEl.checked) oldEl.checked = newEl.checked;
		if (!isActive && oldEl.value !== newEl.value) oldEl.value = newEl.value;
	} else if (
		oldEl instanceof HTMLTextAreaElement &&
		newEl instanceof HTMLTextAreaElement
	) {
		if (!isActive && oldEl.value !== newEl.value) oldEl.value = newEl.value;
	} else if (
		oldEl instanceof HTMLOptionElement &&
		newEl instanceof HTMLOptionElement
	) {
		if (oldEl.selected !== newEl.selected) oldEl.selected = newEl.selected;
	} else if (
		oldEl instanceof HTMLSelectElement &&
		newEl instanceof HTMLSelectElement
	) {
		if (!isActive && oldEl.value !== newEl.value) oldEl.value = newEl.value;
	}
}
