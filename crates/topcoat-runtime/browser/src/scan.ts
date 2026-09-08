import { setupBinding } from "./binding";
import { type CommentMarker, parseComment } from "./comment";
import { setupEventHandler } from "./event";
import { type Scope, ShardUnit } from "./scope";
import type { SignalId } from "./signal";
import { setupTextExpression } from "./text";

type PendingTextExpression = {
	start: Comment;
	js: string;
	scope: Scope;
};

/**
 * Walks the DOM region `(from, to)` under `root`, hydrating signals, shards,
 * dependencies, and element bindings into the provided initial scope.
 *
 * - `from`: walker starts AFTER this node. Pass `null` to start at the
 *   beginning of `root`.
 * - `to`: walker stops BEFORE this node. Pass `null` to walk to the end.
 * - `initialScope`: the content scope new bindings, signals, and
 *   dependencies attach to until a shard start marker pushes a deeper one.
 * - `adoptable`: the ids of signals that stay registered from content being
 *   replaced. A declaration of one of them keeps the existing signal and
 *   moves it into the scanning scope, removing it from the set; whatever is
 *   left in the set afterwards was not declared again.
 */
export function scan(
	root: Node,
	from: Node | null,
	to: Node | null,
	initialScope: Scope,
	adoptable: Set<SignalId> = new Set(),
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

		processMarker(marker, node as Comment, stack, textExpressions, adoptable);
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
	adoptable: Set<SignalId>,
): void {
	// Every scope on the stack is the content scope of a unit, so the top is
	// the innermost unit enclosing the marker.
	const current = stack[stack.length - 1];
	if (current === undefined) throw new Error("Stack was empty");

	switch (marker.kind) {
		case "signal": {
			// An existing signal wins over the declaration's value, and the
			// scanning scope takes it over when it comes from replaced
			// content; a signal another live scope owns stays theirs.
			const { registry } = current.runtime;
			if (
				registry.insert(marker.id, marker.value) ||
				adoptable.delete(marker.id)
			) {
				current.signalIds.add(marker.id);
			}
			break;
		}

		case "dep": {
			current.dependencies.add(marker.id);
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

		case "shard-start": {
			const shard = new ShardUnit(
				current,
				current.runtime,
				marker.id,
				marker.shard,
				marker.identity,
				marker.exprs,
				node,
			);
			stack.push(shard.contentScope);
			break;
		}

		case "shard-end": {
			const top = stack.pop();
			const shard = top?.parent;
			if (!(shard instanceof ShardUnit)) {
				throw new Error(
					`Unbalanced shard: end marker ${marker.id} has no matching start`,
				);
			}
			if (shard.scopeId !== marker.id) {
				throw new Error(
					`Mismatched shard: end ${marker.id} does not match start ${shard.scopeId}`,
				);
			}
			shard.attachEnd(node);
			shard.startWatching();
			break;
		}
	}
}
