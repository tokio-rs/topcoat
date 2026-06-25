import { setupBinding } from "./binding";
import { type CommentMarker, parseComment } from "./comment";
import { setupEventHandler } from "./event";
import { ReactiveScope, type Scope } from "./scope";
import { setupTextExpression } from "./text";

type PendingTextExpression = {
	start: Comment;
	js: string;
	scope: Scope;
};

/**
 * Walks the DOM region `(from, to)` under `root`, hydrating signals, reactive
 * scopes, and element bindings into the provided initial scope.
 *
 * - `from`: walker starts AFTER this node. Pass `null` to start at the
 *   beginning of `root`.
 * - `to`: walker stops BEFORE this node. Pass `null` to walk to the end.
 * - `initialScope`: the scope new bindings/signals attach to until a
 *   `reactive scope start` marker pushes a deeper one.
 */
export function scan(
	root: Node,
	from: Node | null,
	to: Node | null,
	initialScope: Scope,
): void {
	const walker = document.createTreeWalker(
		root,
		NodeFilter.SHOW_COMMENT | NodeFilter.SHOW_ELEMENT,
	);
	if (from) walker.currentNode = from;

	const stack: Scope[] = [initialScope];
	const textExpressions: PendingTextExpression[] = [];

	for (let node = walker.nextNode(); node; node = walker.nextNode()) {
		if (to && node === to) break;

		const current = stack[stack.length - 1];
		if (current === undefined) throw new Error("Stack was empty");

		if (node.nodeType === Node.ELEMENT_NODE) {
			processElement(node as Element, current);
			continue;
		}

		// COMMENT_NODE
		const marker = parseComment(node as Comment);
		if (!marker) continue;

		processMarker(marker, node as Comment, stack, textExpressions);
	}
}

function processElement(el: Element, scope: Scope): void {
	for (const attr of Array.from(el.attributes)) {
		setupBinding(el, attr, scope);
		setupEventHandler(el, attr, scope);
	}
}

function processMarker(
	marker: CommentMarker,
	node: Comment,
	stack: Scope[],
	textExpressions: PendingTextExpression[],
): void {
	const current = stack[stack.length - 1];
	if (current === undefined) throw new Error("Stack was empty");

	switch (marker.kind) {
		case "signal": {
			if (current.runtime.registry.insert(marker.id, marker.value)) {
				current.signalIds.add(marker.id);
			}
			break;
		}

		case "expr-start": {
			textExpressions.push({
				start: node,
				js: marker.js,
				scope: current,
			});
			break;
		}

		case "expr-end": {
			const pending = textExpressions.pop();
			if (!pending) {
				throw new Error("Unbalanced text expression: end marker has no start");
			}
			setupTextExpression(pending.start, node, pending.js, pending.scope);
			break;
		}

		case "scope-start": {
			const reactive = new ReactiveScope(
				current,
				current.runtime,
				marker.id,
				marker.path,
				marker.exprs,
				node,
			);
			stack.push(reactive.contentScope);
			break;
		}

		case "scope-end": {
			const top = stack.pop();
			const reactive = top?.parent;
			if (!(reactive instanceof ReactiveScope)) {
				throw new Error(
					`Unbalanced reactive scope: end marker ${marker.id} has no matching start`,
				);
			}
			if (reactive.scopeId !== marker.id) {
				throw new Error(
					`Mismatched reactive scope: end ${marker.id} does not match start ${reactive.scopeId}`,
				);
			}
			reactive.attachEnd(node);
			reactive.startWatching();
			break;
		}
	}
}
