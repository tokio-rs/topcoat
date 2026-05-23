import { type Expr, eval_expr } from "./index";
import type { Interpreter } from "./interpreter";

export type ExprAssignDeref = {
	type: "AssignDeref";
	place: Expr;
	value: Expr;
};

export function eval_expr_assign_deref(
	expr: ExprAssignDeref,
	interpreter: Interpreter,
): unknown {
	if (expr.place.type !== "SignalRef") {
		throw new Error(
			"AssignDeref expressions may only use signals as the place expression",
		);
	}
	interpreter.getSignal(expr.place.id).set(eval_expr(expr.value, interpreter));
	return undefined;
}
