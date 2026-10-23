import { afterEach, expect, it } from "vitest";
import { dehydrate } from "../../src/expression/dehydrate";
import { Runtime } from "../../src/runtime";
import { Event as RuntimeEvent } from "../../src/surrogate/event";

afterEach(() => {
	document.body.replaceChildren();
});

it("reports native focus destinations and synchronous current targets", () => {
	const first = document.createElement("input");
	const second = document.createElement("input");
	document.body.append(first, second);
	const events: RuntimeEvent[] = [];
	const destinations: (EventTarget | null)[] = [];
	first.addEventListener("focusout", (native) => {
		const event = new RuntimeEvent(native);
		events.push(event);
		destinations.push(
			event.related_target.is_some().dehydrate()
				? event.related_target.unwrap().inner
				: null,
		);
		expect(event.current_target.inner).toBe(first);
	});
	first.focus();
	second.focus();
	first.focus();
	first.blur();
	expect(destinations).toEqual([second, null]);
	expect(events).toHaveLength(2);
	for (const event of events) {
		expect(event.current_target.inner).toBeNull();
	}
});

it("preserves null, element, and non-element native related targets locally", () => {
	for (const native of [
		null,
		document.createElement("input"),
		document.createElementNS("http://www.w3.org/2000/svg", "svg"),
		new EventTarget(),
		document,
		window,
	]) {
		const event = new RuntimeEvent(
			new FocusEvent("focusout", { relatedTarget: native }),
		);
		if (native === null) {
			expect(event.related_target.is_none().dehydrate()).toBe(true);
		} else {
			const target = event.related_target.unwrap();
			expect(target.inner).toBe(native);
			expect(() => dehydrate(target)).toThrow();
			expect(() => dehydrate(event.related_target)).toThrow();
		}
	}
	const unrelated = new RuntimeEvent(new Event("click"));
	expect(unrelated.related_target.is_none().dehydrate()).toBe(true);
	expect(unrelated.old_state.is_none().dehydrate()).toBe(true);
	expect(unrelated.new_state.is_none().dehydrate()).toBe(true);
	const synthetic = new RuntimeEvent(new ToggleEvent("toggle"));
	expect(synthetic.old_state.unwrap().toString()).toBe("");
	expect(synthetic.new_state.unwrap().toString()).toBe("");
});

it("reports native opening and closing through hydrated toggle handlers", async () => {
	const popover = document.createElement("div");
	popover.popover = "manual";
	popover.setAttribute(
		"data-topcoat-on:toggle",
		`(e) => {
		cx.signal('old').set(e.old_state.unwrap());
		cx.signal('new').set(e.new_state.unwrap());
	}`,
	);
	document.body.append(popover);
	const runtime = new Runtime();
	runtime.registry.insert("old", runtime.context.hydrate(""));
	runtime.registry.insert("new", runtime.context.hydrate(""));
	const before: string[][] = [];
	popover.addEventListener("beforetoggle", (native) => {
		const event = new RuntimeEvent(native);
		before.push([
			event.old_state.unwrap().toString(),
			event.new_state.unwrap().toString(),
		]);
	});
	try {
		runtime.start(document);
		for (const [action, oldState, newState] of [
			["showPopover", "closed", "open"],
			["hidePopover", "open", "closed"],
		] as const) {
			const toggled = new Promise<void>((resolve) =>
				popover.addEventListener("toggle", () => resolve(), { once: true }),
			);
			popover[action]();
			await toggled;
			expect(dehydrate(runtime.registry.read("old"))).toBe(oldState);
			expect(dehydrate(runtime.registry.read("new"))).toBe(newState);
		}
		expect(before).toEqual([
			["closed", "open"],
			["open", "closed"],
		]);
	} finally {
		runtime.navigation.dispose();
		runtime.page.dispose();
	}
});

it("copies focus target data through a hydrated handler without serializing the handle", () => {
	document.body.innerHTML = `<input id="first" data-topcoat-on:focusout="(e) => {
		if (e.related_target.is_some().dehydrate()) {
			cx.signal('destination').set(e.related_target.unwrap().id);
		}
	}"><input id="second">`;
	const first = document.querySelector<HTMLInputElement>("#first");
	const second = document.querySelector<HTMLInputElement>("#second");
	if (!first || !second) throw new Error("Missing inputs");
	const runtime = new Runtime();
	runtime.registry.insert("destination", runtime.context.hydrate(""));
	try {
		runtime.start(document);
		first.focus();
		second.focus();
		expect(dehydrate(runtime.registry.read("destination"))).toBe("second");
	} finally {
		runtime.navigation.dispose();
		runtime.page.dispose();
	}
});
