import { expect, it } from "vitest";
import { Context } from "../expression/context";
import { dehydrate } from "../expression/dehydrate";
import { SignalRegistry } from "../signal-registry";
import { Bool } from "./bool";
import { F64 } from "./f64";
import { Option } from "./option";
import { Ref } from "./ref";
import { String as RuntimeString } from "./string";
import { Tuple } from "./tuple";

const hydrate = (wire: unknown) =>
	new Context(new SignalRegistry()).hydrate(wire);

it("hydrates arrays as tuples with indexed elements", () => {
	const wire = [1.5, { t: "Option", v: [true, "x"] }, { t: "Option", v: null }];
	const tuple = hydrate(wire) as Tuple;
	expect(tuple).toBeInstanceOf(Tuple);
	expect(tuple[0]).toBeInstanceOf(F64);
	const inner = (tuple[1] as Option<Tuple>).unwrap();
	expect(inner).toBeInstanceOf(Tuple);
	expect(inner[1]).toBeInstanceOf(RuntimeString);
	expect(dehydrate(tuple)).toEqual(wire);
	expect(dehydrate(tuple.clone())).toEqual(wire);
});

it("reads elements through references and destructuring", () => {
	const tuple = new Tuple([new F64(1), new Bool(true)]);
	const reference = Ref.shared(() => tuple);
	const element = reference[1] as Ref<Bool>;
	expect(element).toBeInstanceOf(Ref);
	expect(element.deref()).toBe(tuple[1]);
	expect(reference[2]).toBeUndefined();
	const [first, second] = tuple;
	expect(first).toBe(tuple[0]);
	expect(second).toBe(tuple[1]);
});

it("renders node text by concatenating elements", () => {
	const tuple = new Tuple([
		new F64(1.5),
		Option.none(),
		new RuntimeString("a"),
		new F64(2.5),
	]);
	expect(tuple.toNodeText()).toBe("1.5a2.5");
});

it("renders attribute values from present elements", () => {
	const absent = new Tuple([new Bool(false), Option.none()]);
	expect(absent.isAttributePresent()).toBe(false);

	const present = new Tuple([
		new RuntimeString("a"),
		Option.none(),
		new RuntimeString("b"),
	]);
	expect(present.isAttributePresent()).toBe(true);
	expect(present.toAttributeValue()).toBe("ab");
});
