import type { Context } from "./context";
import type { Scope } from "./scope";
import { Event } from "./surrogate";

const EVENT_HANDLER_PREFIX = "data-topcoat-on:";

type EventHandler = (event: unknown) => void;
type Compile = (ctx: Context) => EventHandler;

export function setupEventHandler(el: Element, attr: Attr, scope: Scope): void {
	if (!attr.name.startsWith(EVENT_HANDLER_PREFIX)) return;

	const name = attr.name.substring(EVENT_HANDLER_PREFIX.length);
	const compile = new Function("cx", `return ${attr.value};`) as Compile;

	const handler = compile(scope.runtime.context);
	el.addEventListener(name, (event) => handler(new Event(event)));
}
