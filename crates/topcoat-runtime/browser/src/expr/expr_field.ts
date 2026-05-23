import { type Expr, eval_expr } from "./index";
import type { Interpreter } from "./interpreter";

export type ExprField = {
	type: "Field";
	receiver: Expr;
	name: string;
};

export function eval_expr_field(
	expr: ExprField,
	interpreter: Interpreter,
): unknown {
	const receiver = eval_expr(expr.receiver, interpreter) as Record<
		string,
		unknown
	>;
	return receiver[expr.name];
}
