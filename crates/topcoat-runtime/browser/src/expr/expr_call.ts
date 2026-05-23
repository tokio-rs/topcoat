import { type Expr, eval_expr } from "./index";
import type { Interpreter } from "./interpreter";

export type ExprCall = {
	type: "Call";
	receiver: Expr;
	params: Expr[];
};

export function eval_expr_call(
	expr: ExprCall,
	interpreter: Interpreter,
): unknown {
	const receiver = eval_expr(expr.receiver, interpreter) as (
		...params: unknown[]
	) => unknown;
	console.log(receiver);
	const params = expr.params.map((param) => eval_expr(param, interpreter));
	return receiver(...params);
}
