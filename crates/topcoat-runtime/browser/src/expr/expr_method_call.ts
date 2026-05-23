import { type Expr, eval_expr } from "./index";
import type { Interpreter } from "./interpreter";

export type ExprMethodCall = {
	type: "MethodCall";
	receiver: Expr;
	method: string;
};

export function eval_expr_method_call(
	expr: ExprMethodCall,
	interpreter: Interpreter,
): unknown {
	const receiver = eval_expr(expr.receiver, interpreter);

	switch (typeof receiver) {
		case "string":
			switch (expr.method) {
				case "clone":
					return receiver;
			}
			break;
	}

	throw new Error(
		`Unsupported method "${expr.method}" on value of type "${typeof receiver}"`,
	);
}
