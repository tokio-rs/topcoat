import { compile } from "../expression/compile";
import type { Scope } from "../scope";
import { Event } from "../surrogate";

const EVENT_HANDLER_PREFIX = "data-topcoat-on:";

type EventHandler = (event: unknown) => void;

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
