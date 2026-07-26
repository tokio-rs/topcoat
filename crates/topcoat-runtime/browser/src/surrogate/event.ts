import { Bool } from "./bool";
import { F64 } from "./f64";
import { String as RuntimeString } from "./string";

type EventTargetLike = globalThis.EventTarget | null;

export class Event {
	constructor(private readonly inner: globalThis.Event) {}

	get alt_key(): Bool {
		return boolProp(this.inner, "altKey");
	}

	get bubbles(): Bool {
		return new Bool(this.inner.bubbles);
	}

	get button(): F64 {
		return f64Prop(this.inner, "button");
	}

	get buttons(): F64 {
		return f64Prop(this.inner, "buttons");
	}

	get cancelable(): Bool {
		return new Bool(this.inner.cancelable);
	}

	get client_x(): F64 {
		return f64Prop(this.inner, "clientX");
	}

	get client_y(): F64 {
		return f64Prop(this.inner, "clientY");
	}

	get code(): RuntimeString {
		return stringProp(this.inner, "code");
	}

	get ctrl_key(): Bool {
		return boolProp(this.inner, "ctrlKey");
	}

	get current_target(): EventTarget {
		return new EventTarget(this.inner.currentTarget);
	}

	get data(): RuntimeString {
		return stringProp(this.inner, "data");
	}

	get default_prevented(): Bool {
		return new Bool(this.inner.defaultPrevented);
	}

	get delta_x(): F64 {
		return f64Prop(this.inner, "deltaX");
	}

	get delta_y(): F64 {
		return f64Prop(this.inner, "deltaY");
	}

	get delta_z(): F64 {
		return f64Prop(this.inner, "deltaZ");
	}

	get event_type(): RuntimeString {
		return new RuntimeString(this.inner.type);
	}

	get input_type(): RuntimeString {
		return stringProp(this.inner, "inputType");
	}

	get is_composing(): Bool {
		return boolProp(this.inner, "isComposing");
	}

	get key(): RuntimeString {
		return stringProp(this.inner, "key");
	}

	get meta_key(): Bool {
		return boolProp(this.inner, "metaKey");
	}

	get movement_x(): F64 {
		return f64Prop(this.inner, "movementX");
	}

	get movement_y(): F64 {
		return f64Prop(this.inner, "movementY");
	}

	get offset_x(): F64 {
		return f64Prop(this.inner, "offsetX");
	}

	get offset_y(): F64 {
		return f64Prop(this.inner, "offsetY");
	}

	get page_x(): F64 {
		return f64Prop(this.inner, "pageX");
	}

	get page_y(): F64 {
		return f64Prop(this.inner, "pageY");
	}

	get pointer_id(): F64 {
		return f64Prop(this.inner, "pointerId");
	}

	get pointer_type(): RuntimeString {
		return stringProp(this.inner, "pointerType");
	}

	get repeat(): Bool {
		return boolProp(this.inner, "repeat");
	}

	get screen_x(): F64 {
		return f64Prop(this.inner, "screenX");
	}

	get screen_y(): F64 {
		return f64Prop(this.inner, "screenY");
	}

	get shift_key(): Bool {
		return boolProp(this.inner, "shiftKey");
	}

	get target(): EventTarget {
		return new EventTarget(this.inner.target);
	}

	get time_stamp(): F64 {
		return new F64(this.inner.timeStamp);
	}

	prevent_default(): void {
		this.inner.preventDefault();
	}

	stop_immediate_propagation(): void {
		this.inner.stopImmediatePropagation();
	}

	stop_propagation(): void {
		this.inner.stopPropagation();
	}
}

export class EventTarget {
	constructor(private readonly inner: EventTargetLike) {}

	get checked(): Bool {
		return boolProp(this.inner, "checked");
	}

	get id(): RuntimeString {
		return stringProp(this.inner, "id");
	}

	get name(): RuntimeString {
		return stringProp(this.inner, "name");
	}

	get text_content(): RuntimeString {
		return new RuntimeString(
			this.inner instanceof Node ? (this.inner.textContent ?? "") : "",
		);
	}

	get value(): RuntimeString {
		return stringProp(this.inner, "value");
	}
}

function boolProp(source: unknown, name: string): Bool {
	return new Bool(
		typeof source === "object" && source !== null && name in source
			? Boolean((source as Record<string, unknown>)[name])
			: false,
	);
}

function f64Prop(source: unknown, name: string): F64 {
	const value =
		typeof source === "object" && source !== null && name in source
			? (source as Record<string, unknown>)[name]
			: 0;
	return new F64(
		typeof value === "number" && Number.isFinite(value) ? value : 0,
	);
}

function stringProp(source: unknown, name: string): RuntimeString {
	const value =
		typeof source === "object" && source !== null && name in source
			? (source as Record<string, unknown>)[name]
			: "";
	return new RuntimeString(value == null ? "" : globalThis.String(value));
}
