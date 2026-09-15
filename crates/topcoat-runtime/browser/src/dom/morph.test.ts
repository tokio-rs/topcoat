// @vitest-environment happy-dom
import { beforeEach, expect, it } from "vitest";

import { morph } from "./morph";

beforeEach(() => {
	document.body.innerHTML = "";
});

/** Parses `html` into nodes the way a unit does with a server response. */
function nodes(html: string): Node[] {
	const fragment = document.createRange().createContextualFragment(html);
	return Array.from(fragment.childNodes);
}

/** Morphs the whole body into `html`. */
function morphBody(html: string): void {
	morph(document.body, null, null, nodes(html));
}

it("updates text and attributes in place, keeping the element", () => {
	document.body.innerHTML = `<p class="a" title="t">old</p>`;
	const p = document.body.firstElementChild;

	morphBody(`<p class="b" lang="en">new</p>`);

	expect(document.body.firstElementChild).toBe(p);
	expect(document.body.innerHTML).toBe(`<p class="b" lang="en">new</p>`);
});

it("keeps a focused input and what the user typed while its siblings change", () => {
	document.body.innerHTML = `<input value="sho"><ul><li>shoe</li><li>shorts</li></ul>`;
	const input = document.querySelector("input") as HTMLInputElement;
	input.focus();
	input.value = "shoes";

	morphBody(`<input value="shoes"><ul><li>shoes</li></ul><p>1 result</p>`);

	expect(document.querySelector("input")).toBe(input);
	expect(document.activeElement).toBe(input);
	expect(input.value).toBe("shoes");
	expect(document.body.innerHTML).toBe(
		`<input value="sho"><ul><li>shoes</li></ul><p>1 result</p>`,
	);
});

it("syncs the value of an input that is not focused", () => {
	document.body.innerHTML = `<input value="a"><input type="checkbox">`;
	const [text, box] = Array.from(
		document.querySelectorAll("input"),
	) as HTMLInputElement[];
	(text as HTMLInputElement).value = "typed";

	morphBody(`<input value="b"><input type="checkbox" checked>`);

	expect((text as HTMLInputElement).value).toBe("b");
	expect((box as HTMLInputElement).checked).toBe(true);
});

it("matches elements by id across a reorder", () => {
	document.body.innerHTML = `<ul><li id="a">a</li><li id="b">b</li><li id="c">c</li></ul>`;
	const [a, , c] = Array.from(document.querySelectorAll("li"));

	morphBody(`<ul><li id="c">c</li><li id="a">a!</li></ul>`);

	const after = Array.from(document.querySelectorAll("li"));
	expect(after).toEqual([c, a]);
	expect(document.body.innerHTML).toBe(
		`<ul><li id="c">c</li><li id="a">a!</li></ul>`,
	);
});

it("inserts in front of a list without disturbing the identified items", () => {
	document.body.innerHTML = `<ul><li id="a">a</li><li id="b">b</li></ul>`;
	const [a, b] = Array.from(document.querySelectorAll("li"));

	morphBody(`<ul><li id="x">x</li><li id="a">a</li><li id="b">b</li></ul>`);

	const after = Array.from(document.querySelectorAll("li"));
	expect(after[1]).toBe(a);
	expect(after[2]).toBe(b);
	expect(document.body.innerHTML).toBe(
		`<ul><li id="x">x</li><li id="a">a</li><li id="b">b</li></ul>`,
	);
});

it("finds a container again through the ids inside it when a sibling appears before it", () => {
	document.body.innerHTML = `<div><p id="x">x</p></div>`;
	const x = document.getElementById("x");
	const div = x?.parentElement;

	morphBody(`<h1>title</h1><div><p id="x">x</p></div>`);

	expect(document.getElementById("x")).toBe(x);
	expect(x?.parentElement).toBe(div);
	expect(document.body.innerHTML).toBe(
		`<h1>title</h1><div><p id="x">x</p></div>`,
	);
});

it("moves an identified element into freshly inserted content", () => {
	document.body.innerHTML = `<div id="a">a</div>`;
	const a = document.getElementById("a");

	morphBody(`<section><div id="a">a</div></section>`);

	expect(document.getElementById("a")).toBe(a);
	expect(document.body.innerHTML).toBe(
		`<section><div id="a">a</div></section>`,
	);
});

it("never pairs an identified element with a different id", () => {
	document.body.innerHTML = `<div id="a">a</div><div id="b">b</div>`;
	const b = document.getElementById("b");

	morphBody(`<div id="b">b</div>`);

	expect(document.body.firstElementChild).toBe(b);
	expect(document.body.innerHTML).toBe(`<div id="b">b</div>`);
});

it("replaces an element whose tag changed and removes trailing nodes", () => {
	document.body.innerHTML = `<p>a</p><p>b</p><p>c</p>`;

	morphBody(`<h2>a</h2><p>b</p>`);

	expect(document.body.innerHTML).toBe(`<h2>a</h2><p>b</p>`);
});

it("updates comment data in place", () => {
	document.body.innerHTML = `<!--one--><p>p</p>`;
	const comment = document.body.firstChild;

	morphBody(`<!--two--><p>p</p>`);

	expect(document.body.firstChild).toBe(comment);
	expect(comment?.nodeValue).toBe("two");
});

it("morphs only the range between two markers", () => {
	document.body.innerHTML = `<p>before</p><!--start--><span>old</span><!--end--><p>after</p>`;
	const [before, start, , end, after] = Array.from(document.body.childNodes);

	morph(
		document.body,
		start as ChildNode,
		end as ChildNode,
		nodes(`<span>new</span><em>more</em>`),
	);

	expect(Array.from(document.body.childNodes).slice(0, 2)).toEqual([
		before,
		start,
	]);
	expect(Array.from(document.body.childNodes).slice(-2)).toEqual([end, after]);
	expect(document.body.innerHTML).toBe(
		`<p>before</p><!--start--><span>new</span><em>more</em><!--end--><p>after</p>`,
	);
});

it("empties a range and fills an empty one", () => {
	document.body.innerHTML = `<!--start--><span>old</span><!--end-->`;
	const [start, , end] = Array.from(document.body.childNodes) as [
		ChildNode,
		ChildNode,
		ChildNode,
	];

	morph(document.body, start, end, []);
	expect(document.body.innerHTML).toBe(`<!--start--><!--end-->`);

	morph(document.body, start, end, nodes(`<b>new</b>`));
	expect(document.body.innerHTML).toBe(`<!--start--><b>new</b><!--end-->`);
});

it("leaves an unchanged document untouched", () => {
	document.body.innerHTML = `<div><input value="a"><!--c--><p>t</p></div>`;
	const before = Array.from(document.body.querySelectorAll("*"));

	morphBody(document.body.innerHTML);

	expect(Array.from(document.body.querySelectorAll("*"))).toEqual(before);
});
