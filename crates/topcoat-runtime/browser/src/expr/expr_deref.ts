import { type Expr, eval_expr } from "./index";
import type { Interpreter } from "./interpreter";

export type ExprDeref = {
	type: "Deref";
	inner: Expr;
};

export function eval_expr_deref(
	expr: ExprDeref,
	interpreter: Interpreter,
): unknown {
	const target = eval_expr(expr.inner, interpreter);
	return (target as () => unknown)();
}
