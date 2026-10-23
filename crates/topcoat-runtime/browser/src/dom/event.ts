import { compile } from "../expression/compile";
import { untrack } from "../reactivity";
import type { Scope } from "../scope";
import { Event } from "../surrogate";

const EVENT_HANDLER_PREFIX = "data-topcoat-on:";

export type EventHandler = (event: Event) => void;

/** Handler reads never become dependencies of an effect that dispatches them. */
export function invokeEventHandler(
	handler: EventHandler,
	event: globalThis.Event,
): void {
	untrack(() => handler(new Event(event)));
}

export function setupEventHandler(el: Element, attr: Attr, scope: Scope): void {
	if (!attr.name.startsWith(EVENT_HANDLER_PREFIX)) return;

	const name = attr.name.substring(EVENT_HANDLER_PREFIX.length);
	const expression = compile<EventHandler>(attr.value, `event @${name}`);
	const handler = expression(scope.runtime.context);
	// Dispose the listener with its scope. A DOM update may reuse the element
	// and attach a new listener.
	el.addEventListener(name, (event) => invokeEventHandler(handler, event), {
		signal: scope.abortSignal,
	});
}
