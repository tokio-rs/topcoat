import { afterEach, expect, it, vi } from "vitest";

import { flushEffects, signal } from "./reactivity";
import { Runtime } from "./runtime";
import { Scope } from "./scope";

const scopes: Scope[] = [];

function scope(): Scope {
	const owner = new Scope(null, new Runtime());
	scopes.push(owner);
	return owner;
}

afterEach(() => {
	for (const owner of scopes) owner.dispose();
	scopes.length = 0;
	flushEffects();
});

it("releases parent and child effects while keeping signals for adoption", () => {
	const parent = scope();
	const child = new Scope(parent, parent.runtime);
	parent.runtime.registry.insert("kept", 0);
	child.signalIds.add("kept");
	const value = parent.runtime.registry.handle("kept");
	const callback = vi.fn(() => value());
	parent.effect(callback);
	child.effect(callback);
	value.set(1);
	expect(parent.release()).toEqual(new Set(["kept"]));
	flushEffects();
	value.set(2);
	flushEffects();
	expect(callback).toHaveBeenCalledTimes(2);
	expect(parent.runtime.registry.read("kept")).toBe(2);
	expect(child.isDisposed).toBe(true);
	expect(parent.children.size).toBe(0);
	parent.runtime.registry.delete("kept");
});

it("cannot subscribe again after releasing itself during its first run", () => {
	const owner = scope();
	const value = signal(0);
	const callback = vi.fn(() => {
		value();
		owner.release();
		value();
	});
	owner.effect(callback);
	value.set(1);
	flushEffects();
	owner.effect(callback);
	expect(callback).toHaveBeenCalledTimes(1);
});

it("disposes a failed initial effect and restores tracking for later effects", () => {
	const owner = scope();
	const value = signal(0);
	const broken = vi.fn(() => {
		value();
		throw new Error("initial run failed");
	});
	expect(() => owner.effect(broken)).toThrow("initial run failed");
	const values: number[] = [];
	owner.effect(() => values.push(value()));
	value.set(1);
	flushEffects();
	expect(broken).toHaveBeenCalledTimes(1);
	expect(values).toEqual([0, 1]);
});
