import { Str } from "../src/surrogate/string";

const cases = [
	["", 0],
	["ASCII", 5],
	["\u{E9}", 2],
	["\u{D55C}", 3],
	["\u{1F60A}", 4],
	["\u{D55C}\u{1F60A}", 7],
	["e\u{301}", 3],
] as const;

for (const [value, expected] of cases) {
	const actual = Number(new Str(value).len().toString());
	if (actual !== expected) {
		throw new Error(
			`expected UTF-8 length ${expected} for ${JSON.stringify(value)}, got ${actual}`,
		);
	}
}
