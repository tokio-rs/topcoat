import { type Expr, eval_expr } from "./index";
import type { Interpreter } from "./interpreter";

export type ExprClosure = {
	type: "Closure";
	params: string[];
	body: Expr;
};

export function eval_expr_closure(
	expr: ExprClosure,
	interpreter: Interpreter,
): unknown {
	return (...params: unknown[]) => {
		interpreter.pushEnvironment();
		for (const [index, paramName] of expr.params.entries()) {
			interpreter.getEnvironment().define(paramName, params[index]);
		}
		eval_expr(expr.body, interpreter);
		interpreter.popEnvironment();
	};
}
