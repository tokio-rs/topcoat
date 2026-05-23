import {
	type ExprAssignDeref,
	eval_expr_assign_deref,
} from "./expr_assign_deref";
import { type ExprCall, eval_expr_call } from "./expr_call";
import { type ExprClosure, eval_expr_closure } from "./expr_closure";
import { type ExprDeref, eval_expr_deref } from "./expr_deref";
import { type ExprField, eval_expr_field } from "./expr_field";
import { type ExprLit, eval_expr_lit } from "./expr_lit";
import { type ExprMethodCall, eval_expr_method_call } from "./expr_method_call";
import { type ExprSignalRef, eval_expr_signal_ref } from "./expr_signal_ref";
import type { Interpreter } from "./interpreter";

export { Interpreter } from "./interpreter";

export type Expr =
	| ExprAssignDeref
	| ExprCall
	| ExprClosure
	| ExprDeref
	| ExprField
	| ExprLit<unknown>
	| ExprMethodCall
	| ExprSignalRef;

export function eval_expr(expr: Expr, interpreter: Interpreter): unknown {
	switch (expr.type) {
		case "AssignDeref":
			return eval_expr_assign_deref(expr, interpreter);
		case "Call":
			return eval_expr_call(expr, interpreter);
		case "Closure":
			return eval_expr_closure(expr, interpreter);
		case "Deref":
			return eval_expr_deref(expr, interpreter);
		case "Field":
			return eval_expr_field(expr, interpreter);
		case "Lit":
			return eval_expr_lit(expr, interpreter);
		case "MethodCall":
			return eval_expr_method_call(expr, interpreter);
		case "SignalRef":
			return eval_expr_signal_ref(expr, interpreter);
		default:
			throw Error(
				`Unsupported expression for evaluation: ${(expr as Expr).type}`,
			);
	}
}
