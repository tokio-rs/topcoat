// @vitest-environment happy-dom

import { expect, it } from "vitest";
import { dehydrate } from "../expression/dehydrate";
import { Event as RuntimeEvent } from "./event";

it("distinguishes absent related targets and toggle states from present values", () => {
	for (const native of [
		new Event("click"),
		new Event("toggle"),
		new FocusEvent("blur"),
	]) {
		const event = new RuntimeEvent(native);
		expect(event.related_target.is_none().dehydrate()).toBe(true);
		expect(event.old_state.is_none().dehydrate()).toBe(true);
		expect(event.new_state.is_none().dehydrate()).toBe(true);
	}
});

it("keeps related target identity and live element properties", () => {
	const input = document.createElement("input");
	input.id = "destination";
	const event = new RuntimeEvent(
		new FocusEvent("focusout", { relatedTarget: input }),
	);
	const target = event.related_target.unwrap();
	expect(target.inner).toBe(input);
	expect(target.id.toString()).toBe("destination");
	input.value = "changed";
	expect(target.value.toString()).toBe("changed");
	expect(() => dehydrate(target)).toThrow();
	expect(() => dehydrate(event.related_target)).toThrow();
});

it("preserves non-element targets without assuming HTMLElement", () => {
	for (const native of [new EventTarget(), document, window]) {
		const event = new RuntimeEvent(
			new FocusEvent("focusin", { relatedTarget: native }),
		);
		const target = event.related_target.unwrap();
		expect(target.inner).toBe(native);
		expect(target.id.toString()).toBe("");
		expect(target.value.toString()).toBe("");
		expect(target.checked.dehydrate()).toBe(false);
		expect(() => dehydrate(target)).toThrow();
	}
});

it("preserves supplied toggle strings including empty synthetic defaults", () => {
	for (const [oldState, newState] of [
		["closed", "open"],
		["open", "closed"],
		["", ""],
		["custom", "state"],
	] as const) {
		// Happy DOM has no ToggleEvent constructor. Native events are covered by
		// the browser suite; these properties exercise the same facade reads.
		const event = new RuntimeEvent(
			Object.assign(new Event("toggle"), { oldState, newState }),
		);
		expect(event.old_state.unwrap().toString()).toBe(oldState);
		expect(event.new_state.unwrap().toString()).toBe(newState);
		expect(event.related_target.is_none().dehydrate()).toBe(true);
	}
});

it("preserves keyboard and input data and existing absent-field defaults", () => {
	const keyboard = new RuntimeEvent(
		new KeyboardEvent("keydown", {
			key: "Escape",
			code: "Escape",
			isComposing: true,
			repeat: true,
			altKey: true,
			ctrlKey: true,
			metaKey: true,
			shiftKey: true,
		}),
	);
	expect(keyboard.key.toString()).toBe("Escape");
	expect(keyboard.code.toString()).toBe("Escape");
	for (const value of [
		keyboard.is_composing,
		keyboard.repeat,
		keyboard.alt_key,
		keyboard.ctrl_key,
		keyboard.meta_key,
		keyboard.shift_key,
	]) {
		expect(value.dehydrate()).toBe(true);
	}
	const input = new RuntimeEvent(
		new InputEvent("input", {
			data: "x",
			inputType: "insertText",
			isComposing: true,
		}),
	);
	expect(input.data.toString()).toBe("x");
	expect(input.input_type.toString()).toBe("insertText");
	expect(input.is_composing.dehydrate()).toBe(true);
	const plain = new RuntimeEvent(new Event("click"));
	expect(plain.key.toString()).toBe("");
	expect(plain.is_composing.dehydrate()).toBe(false);
	expect(plain.client_x.dehydrate()).toBe(0);
});

it("preserves dispatch targets, cancellation, and propagation methods", () => {
	for (const stop of [
		"stop_propagation",
		"stop_immediate_propagation",
	] as const) {
		const parent = document.createElement("div");
		const input = document.createElement("input");
		input.name = "choice";
		input.checked = true;
		parent.append(input);
		const calls: string[] = [];
		input.addEventListener("click", (native) => {
			const event = new RuntimeEvent(native);
			expect(event.target.inner).toBe(input);
			expect(event.current_target.inner).toBe(input);
			expect(event.target.name.toString()).toBe("choice");
			expect(event.target.checked.dehydrate()).toBe(true);
			expect(event.event_type.toString()).toBe("click");
			expect(event.time_stamp.dehydrate()).toBe(native.timeStamp);
			expect(event.bubbles.dehydrate()).toBe(true);
			expect(event.cancelable.dehydrate()).toBe(true);
			event.prevent_default();
			expect(event.default_prevented.dehydrate()).toBe(true);
			event[stop]();
		});
		input.addEventListener("click", () => calls.push("sibling"));
		parent.addEventListener("click", () => calls.push("parent"));
		expect(
			input.dispatchEvent(
				new Event("click", { bubbles: true, cancelable: true }),
			),
		).toBe(false);
		expect(calls).toEqual(stop === "stop_propagation" ? ["sibling"] : []);
	}
});
