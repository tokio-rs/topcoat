import type { Context } from "./context";
import type { Scope } from "./scope";

const EVENT_HANDLER_PREFIX = "data-topcoat-on:";

type Compile = (ctx: Context) => EventListener;

export function setupEventHandler(el: Element, attr: Attr, scope: Scope): void {
	if (!attr.name.startsWith(EVENT_HANDLER_PREFIX)) return;

	const name = attr.name.substring(EVENT_HANDLER_PREFIX.length);
	const compile = new Function("__context", `return ${attr.value};`) as Compile;

	const handler = compile(scope.runtime.context);
	el.addEventListener(name, handler);
}
