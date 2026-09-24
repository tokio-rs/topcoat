import { ShardUnit } from "../render/shard";
import { type Region, Scope } from "../scope";
import type { SignalId } from "../signal-registry";
import { setupBinding } from "./binding";
import { setupEventHandler } from "./event";
import { type CommentMarker, parseComment } from "./markers";
import { setupTextExpression } from "./text";

type PendingTextExpression = {
	start: Comment;
	js: string;
	scope: Scope;
};

/**
 * The scope the markup being walked belongs to, with the shard or region
 * whose end marker closes it.
 */
type Frame = {
	scope: Scope;
	shard: ShardUnit | null;
	region: Region | null;
};

/**
 * Attaches runtime behavior to the DOM range `(from, to)` under `root`.
 *
 * - `from`: starts after this node, or at the beginning of `root` if `null`.
 * - `to`: stops before this node, or at the end of `root` if `null`.
 * - `initialScope`: owns resources until a nested shard starts its own scope.
 * - `adoptable`: signal IDs retained from replaced content. A matching
 *   declaration keeps the existing value, assigns ownership to the current
 *   scope, and removes the ID from this set. Remaining IDs were not reused.
 */
export function hydrate(
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

	const stack: Frame[] = [{ scope: initialScope, shard: null, region: null }];
	const textExpressions: PendingTextExpression[] = [];

	for (let node = walker.nextNode(); node; node = walker.nextNode()) {
		if (to && node === to) break;

		const current = stack[stack.length - 1]?.scope;
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
	stack: Frame[],
	textExpressions: PendingTextExpression[],
	adoptable: Set<SignalId>,
): void {
	// The top of the stack is the innermost unit or region enclosing the
	// marker; its owner is the enclosing unit's content scope.
	const current = stack[stack.length - 1]?.scope;
	if (current === undefined) throw new Error("Stack was empty");

	switch (marker.kind) {
		case "signal": {
			// An existing signal wins over the declaration's value, and the
			// scanning scope takes it over when it comes from replaced
			// content; a signal another live scope owns stays theirs.
			const { context, registry } = current.runtime;
			if (
				registry.insert(marker.id, context.hydrate(marker.value)) ||
				adoptable.delete(marker.id)
			) {
				current.signalIds.add(marker.id);
			}
			break;
		}

		case "dep": {
			current.owner.dependencies.add(marker.id);
			break;
		}

		case "connect": {
			current.owner.requiresConnection = true;
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

		case "region-start": {
			const scope = new Scope(current, current.runtime, current.owner);
			const region: Region = { id: marker.id, start: node, end: null, scope };
			current.regions.set(marker.id, region);
			stack.push({ scope, shard: null, region });
			break;
		}

		case "region-end": {
			const region = stack[stack.length - 1]?.region;
			if (!region) {
				throw new Error(
					`Unbalanced region: end marker ${marker.id} has no matching start`,
				);
			}
			if (region.id !== marker.id) {
				throw new Error(
					`Mismatched region: end ${marker.id} does not match start ${region.id}`,
				);
			}
			region.end = node;
			stack.pop();
			break;
		}

		case "shard-start": {
			const shard = new ShardUnit(
				current,
				current.runtime,
				marker.path,
				marker.identity,
				marker.exprs,
				node,
			);
			stack.push({ scope: shard.contentScope, shard, region: null });
			break;
		}

		case "shard-end": {
			const shard = stack[stack.length - 1]?.shard;
			if (!shard) {
				throw new Error(
					`Unbalanced shard: end marker ${marker.identity} has no matching start`,
				);
			}
			if (shard.identity !== marker.identity) {
				throw new Error(
					`Mismatched shard: end ${marker.identity} does not match start ${shard.identity}`,
				);
			}
			shard.attachEnd(node);
			stack.pop();
			shard.startWatching();
			break;
		}
	}
}
