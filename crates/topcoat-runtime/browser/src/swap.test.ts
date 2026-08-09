import { expect, it, vi } from "vitest";
import { Runtime } from "./runtime";
import { replaceFragment } from "./swap";

// `replaceFragment` delegates re-attachment to `scan`, which needs a real DOM
// tree walker. Mock it and record the (root, from, to) range it is asked to
// scan so the tests can assert exactly which nodes were covered.
const scans: { root: unknown; from: unknown; to: unknown }[] = [];
vi.mock("./scan", () => ({
	scan: (root: unknown, from: unknown, to: unknown) => {
		scans.push({ root, from, to });
	},
}));

/**
 * A minimal fake DOM node supporting the operations `replaceFragment` uses.
 * Children are tracked in an array; `insertBefore`, `replaceChild`,
 * `replaceChildren`, and `removeChild` mutate it. Passing a `FakeFragment`
 * anywhere a node is expected splices the fragment's children in and empties
 * the fragment, mirroring real `DocumentFragment` behavior.
 */
class FakeNode {
	children: FakeNode[] = [];
	parentNode: FakeNode | null = null;

	get nextSibling(): FakeNode | null {
		if (!this.parentNode) return null;
		const siblings = this.parentNode.children;
		const index = siblings.indexOf(this);
		return index === -1 ? null : (siblings[index + 1] ?? null);
	}

	insertBefore(node: FakeNode, ref: FakeNode | null): FakeNode {
		if (node instanceof FakeFragment) {
			for (const child of [...node.children]) this.insertBefore(child, ref);
			return node;
		}
		node.parentNode?.removeChild(node);
		node.parentNode = this;
		const index = ref === null ? -1 : this.children.indexOf(ref);
		if (index === -1) this.children.push(node);
		else this.children.splice(index, 0, node);
		return node;
	}

	replaceChild(node: FakeNode, old: FakeNode): FakeNode {
		const index = this.children.indexOf(old);
		if (index === -1) throw new Error("replaceChild: node not found");
		if (node instanceof FakeFragment) {
			const ref = this.children[index + 1] ?? null;
			this.removeChild(old);
			this.insertBefore(node, ref);
			return old;
		}
		node.parentNode?.removeChild(node);
		node.parentNode = this;
		old.parentNode = null;
		this.children.splice(index, 1, node);
		return old;
	}

	replaceChildren(...nodes: FakeNode[]): void {
		for (const child of [...this.children]) this.removeChild(child);
		for (const node of nodes) this.insertBefore(node, null);
	}

	removeChild(node: FakeNode): FakeNode {
		const index = this.children.indexOf(node);
		if (index === -1) throw new Error("removeChild: node not found");
		this.children.splice(index, 1);
		node.parentNode = null;
		return node;
	}
}

class FakeFragment extends FakeNode {}

function setup() {
	scans.length = 0;
	const runtime = new Runtime();
	const parent = new FakeNode();
	const target = new FakeNode();
	parent.insertBefore(target, null);
	return { scope: runtime.rootScope, parent, target };
}

// `document.createComment` is only used for the outer-mode markers. Stub it so
// the markers are fake nodes too.
Object.defineProperty(globalThis, "document", {
	value: { createComment: () => new FakeNode() },
	configurable: true,
	writable: true,
});

it("inner mode replaces the target's children and scans the target", () => {
	const { scope, target } = setup();
	const oldChild = new FakeNode();
	target.insertBefore(oldChild, null);

	const fragment = new FakeFragment();
	const newChild = new FakeNode();
	fragment.insertBefore(newChild, null);

	replaceFragment(scope, target as never, fragment as never, "inner");

	expect(target.children).toEqual([newChild]);
	expect(oldChild.parentNode).toBeNull();
	expect(fragment.children).toEqual([]);
	expect(scans).toEqual([{ root: target, from: null, to: null }]);
});

it("outer mode swaps the target for the fragment and scans between markers", () => {
	const { scope, parent, target } = setup();
	const before = new FakeNode();
	const after = new FakeNode();
	parent.insertBefore(before, target);
	parent.insertBefore(after, null);

	const fragment = new FakeFragment();
	const newChild = new FakeNode();
	fragment.insertBefore(newChild, null);

	replaceFragment(scope, target as never, fragment as never, "outer");

	// The target is gone, the new child sits where it was, siblings untouched.
	expect(parent.children).toEqual([before, newChild, after]);
	expect(target.parentNode).toBeNull();

	// scan is bounded by the two markers, which are removed afterwards.
	expect(scans).toHaveLength(1);
	const { root, from, to } = scans[0] as {
		root: FakeNode;
		from: FakeNode;
		to: FakeNode;
	};
	expect(root).toBe(parent);
	expect(parent.children).not.toContain(from);
	expect(parent.children).not.toContain(to);
});

it("outer mode without a parent throws", () => {
	const { scope } = setup();
	const orphan = new FakeNode();
	const fragment = new FakeFragment();

	expect(() =>
		replaceFragment(scope, orphan as never, fragment as never, "outer"),
	).toThrow(/no parent/);
	expect(scans).toHaveLength(0);
});
