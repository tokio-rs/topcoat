/** View Transition attribute names, and the typed transition navigation uses. */

/**
 * Declares view transition types (space separated) for a link's runtime
 * navigation, e.g. `data-topcoat-vt-types="expand"`. The page swap runs
 * inside a transition of those types, which the app's stylesheet can match
 * with `:active-view-transition-type(...)` and `::view-transition-*`.
 */
export const VT_TYPES_ATTRIBUTE = "data-topcoat-vt-types";

/**
 * Marks the element that morphs into its counterpart on the destination,
 * e.g. `data-topcoat-vt-shared="card-42"`. The value becomes the element's
 * `view-transition-name` when the link is followed, pairing it with the
 * destination element that declares the same name in its style.
 *
 * Clicking the link applies the name and `VT_SHARED_CLASS` before the
 * browser captures the current page. After the swap, the destination's
 * markup determines the styles the element keeps.
 *
 * Only meaningful inside a link that declares `VT_TYPES_ATTRIBUTE`.
 */
export const VT_SHARED_ATTRIBUTE = "data-topcoat-vt-shared";

/**
 * The `view-transition-class` every marked element carries, letting a
 * stylesheet target just that group with `::view-transition-group(.vt-shared)`
 * instead of `(*)`, which would also match the root snapshot. Engines that
 * predate `view-transition-class` ignore it.
 */
export const VT_SHARED_CLASS = "vt-shared";

/**
 * The view transition types `element` declares, or null when it declares
 * none. An empty attribute counts as none.
 */
export function vtTypesOf(element: Element): string[] | null {
	const raw = element.getAttribute(VT_TYPES_ATTRIBUTE);
	if (raw === null) return null;
	const types = raw.split(/\s+/).filter(Boolean);
	return types.length > 0 ? types : null;
}

/**
 * Applies a DOM change inside a typed same-document view transition,
 * resolving once the change ran and the new state is captured.
 *
 * Applies the change directly where the API is missing or the visitor
 * prefers reduced motion, and retries with the untyped form where types
 * are unsupported. A duplicate `view-transition-name` also degrades a typed
 * transition: `ViewTransition.ready` rejects and the transition is skipped
 * while the change still happens.
 */
export async function startTypedTransition(
	types: string[],
	update: () => void,
): Promise<void> {
	if (
		!("startViewTransition" in document) ||
		matchMedia("(prefers-reduced-motion: reduce)").matches
	) {
		update();
		return;
	}
	const transition = (() => {
		try {
			return document.startViewTransition({ types, update });
		} catch {
			// Engines that predate the options form throw before any DOM
			// change; the untyped form still swaps inside a transition.
			return document.startViewTransition(update);
		}
	})();
	// `updateCallbackDone` only reports that the update callback returned;
	// the browser captures the new page afterwards, before `ready` settles.
	// Waiting for `ready` keeps later frames from landing in the captured
	// snapshot. A skipped transition (a duplicate `view-transition-name`,
	// for example) rejects `ready` even though the change applied, so its
	// rejection must not surface as an unhandled one.
	const captured = transition.ready.catch(() => {});
	// A skipped transition rejects `finished` as well. Nothing here waits for
	// the animation to end, but marking the rejection handled keeps a skip
	// from surfacing as an unhandled one.
	transition.finished.catch(() => {});
	await Promise.all([transition.updateCallbackDone, captured]);
}
