import { compile } from "../expression/compile";
import { Scope } from "../scope";
import { Bool, EventTarget } from "../surrogate";
import type { Str } from "../surrogate/string";
import { type EventHandler, invokeEventHandler } from "./event";

/** The browser counterpart of Rust's EffectContext, owned by one invocation. */
export class EffectContext {
	constructor(
		readonly element: Element,
		private readonly scope: Scope,
	) {}

	get abortSignal(): AbortSignal {
		return this.scope.abortSignal;
	}

	clone(): EffectContext {
		return this;
	}

	target(): EventTarget {
		return new EventTarget(this.element);
	}

	listen(target: EventTarget, event: Str, callback: EventHandler): void {
		target.inner?.addEventListener(
			event.toString(),
			(event) => invokeEventHandler(callback, event),
			{ signal: this.abortSignal },
		);
	}

	on_cleanup(cleanup: () => void): void {
		this.scope.onCleanup(cleanup);
	}

	is_active(): Bool {
		return new Bool(!this.scope.isDisposed);
	}
}

/** Queues initialization so hydration and server subscriptions finish first. */
export function setupEffect(
	element: Element,
	source: string,
	scope: Scope,
): void {
	const expression = compile<(context: EffectContext) => void>(
		source,
		"effect",
	);
	let invocation: Scope | null = null;
	scope.effect(
		() => {
			invocation?.dispose();
			if (scope.isDisposed) return;
			invocation = new Scope(scope, scope.runtime, scope.unit);
			try {
				expression(scope.runtime.context)(
					new EffectContext(element, invocation),
				);
			} catch (error) {
				invocation.dispose();
				scope.runtime.reportError(error);
			}
		},
		{ defer: true },
	);
}
