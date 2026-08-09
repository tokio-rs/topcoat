import { scan } from "./scan";
import type { Scope } from "./scope";

export interface SwapOptions {
	/** Whether to replace the target element's inner HTML or the element itself. */
	mode?: "inner" | "outer";
}

/**
 * Replaces a DOM region with an HTML fragment and re-attaches Topcoat runtime
 * bindings and event handlers.
 *
 * - `selector`: a CSS selector that resolves to the target element.
 * - `html`: the HTML fragment to insert.
 * - `opts.mode`: `inner` replaces the target's children (default); `outer`
 *   replaces the target element itself.
 *
 * New bindings and handlers attach to `scope`, normally the runtime's root
 * scope. Focus is preserved when the active element is inside the target and
 * an element with the same `id` exists in the new fragment.
 */
export function swapHtml(
	scope: Scope,
	selector: string,
	html: string,
	opts: SwapOptions = {},
): void {
	const target = document.querySelector(selector);
	if (!target) {
		throw new Error(`swapHtml: no element matches selector '${selector}'`);
	}

	const active = document.activeElement;
	const activeId = active && active instanceof HTMLElement ? active.id : null;
	const activeSelectionStart =
		active && active instanceof HTMLInputElement ? active.selectionStart : null;
	const activeSelectionEnd =
		active && active instanceof HTMLInputElement ? active.selectionEnd : null;

	const fragment = document.createRange().createContextualFragment(html);
	replaceFragment(scope, target, fragment, opts.mode ?? "inner");

	if (activeId) {
		const restored = document.getElementById(activeId);
		if (restored) {
			restored.focus();
			if (
				restored instanceof HTMLInputElement &&
				activeSelectionStart !== null &&
				activeSelectionEnd !== null
			) {
				restored.setSelectionRange(activeSelectionStart, activeSelectionEnd);
			}
		}
	}
}

/**
 * The DOM surgery behind [`swapHtml`], split out so it can be tested with a
 * mocked DOM: replaces `target` (or its children) with `fragment` and scans
 * exactly the inserted nodes into `scope`.
 */
export function replaceFragment(
	scope: Scope,
	target: Element,
	fragment: DocumentFragment,
	mode: "inner" | "outer",
): void {
	if (mode === "outer") {
		const parent = target.parentNode;
		if (!parent) {
			throw new Error(
				`swapHtml: cannot replace outer HTML of element with no parent`,
			);
		}
		// The markers bracket the inserted content so `scan` walks exactly the
		// new nodes: insert the end marker after the target, swap the target for
		// the fragment (which lands between the markers), then scan that range.
		const start = document.createComment(" topcoat-swap-start ");
		const end = document.createComment(" topcoat-swap-end ");
		parent.insertBefore(start, target);
		parent.insertBefore(end, target.nextSibling);
		parent.replaceChild(fragment, target);
		scan(parent, start, end, scope);
		parent.removeChild(start);
		parent.removeChild(end);
	} else {
		target.replaceChildren(fragment);
		scan(target, null, null, scope);
	}
}
