import { compile } from "../expression/compile";
import type { Scope } from "../scope";
import { Event } from "../surrogate";

const EVENT_HANDLER_PREFIX = "data-topcoat-on:";

/**
 * The event the runtime dispatches on an element after hydrating it, so an
 * `@mount` handler runs whenever the server-rendered element is attached.
 */
export const MOUNT_EVENT = "mount";

type EventHandler = (event: unknown) => void;

/** An element whose mount handler waits for hydration to finish. */
export type PendingMount = { el: Element; scope: Scope };

export function setupEventHandler(el: Element, attr: Attr, scope: Scope): void {
	if (!attr.name.startsWith(EVENT_HANDLER_PREFIX)) return;

	const name = attr.name.substring(EVENT_HANDLER_PREFIX.length);
	const expression = compile<EventHandler>(attr.value, `event @${name}`);
	const handler = expression(scope.runtime.context);
	// Dispose the listener with its scope. A DOM update may reuse the element
	// and attach a new listener.
	el.addEventListener(name, (event) => handler(new Event(event)), {
		signal: scope.abortSignal,
	});
}

/** Returns true if `el` has an `@mount` handler. */
export function hasMountHandler(el: Element): boolean {
	return el.hasAttribute(`${EVENT_HANDLER_PREFIX}${MOUNT_EVENT}`);
}

/**
 * Dispatches the mount event on `mounts` in document order after the current
 * synchronous work finishes. By then the enclosing page or shard has
 * declared every signal and started watching its dependencies. Skips
 * elements whose content was replaced or removed in the meantime.
 */
export function scheduleMounts(mounts: PendingMount[]): void {
	if (mounts.length === 0) return;
	queueMicrotask(() => {
		for (const { el, scope } of mounts) {
			if (scope.isDisposed || !el.isConnected) continue;
			el.dispatchEvent(new CustomEvent(MOUNT_EVENT));
		}
	});
}
