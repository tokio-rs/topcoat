import { defineConfig } from "tsup";

export default defineConfig({
	define: { "import.meta.vitest": "undefined" },
	entry: { coherence: "tests/coherence/entry.ts" },
	format: ["iife"],
	globalName: "TopcoatCoherence",
	outExtension: () => ({ js: ".js" }),
	noExternal: [/(.*)/],
	platform: "browser",
	target: "es2022",
	splitting: false,
	minify: true,
	clean: false,
});
