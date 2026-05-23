import { type Expr, eval_expr } from "./expr";
import type { Scope } from "./scope";

const EVENT_HANDLER_PREFIX = "data-topcoat-on:";

export function setupEventHandler(el: Element, attr: Attr, scope: Scope): void {
	if (!attr.name.startsWith(EVENT_HANDLER_PREFIX)) return;

	const name = attr.name.substring(EVENT_HANDLER_PREFIX.length);
	const expr = JSON.parse(attr.value) as Expr;

	const { interpreter } = scope.runtime;
	scope.run(() => {
		el.addEventListener(name, (...params) => {
			console.log("running:", expr);
			console.log(
				"result:",
				eval_expr(
					{
						type: "Call",
						receiver: expr,
						params: params.map((param) => ({ type: "Lit", value: param })),
					},
					interpreter,
				),
			);
		});
	});
}
