import type { WriteSignal } from "@maverick-js/signals";
import type { SignalId } from "../signal";
import type { Interpreter } from "./interpreter";

export type ExprSignalRef = {
	type: "SignalRef";
	id: SignalId;
};

export function eval_expr_signal_ref(
	expr: ExprSignalRef,
	interpreter: Interpreter,
): WriteSignal<unknown> {
	return interpreter.getSignal(expr.id);
}
