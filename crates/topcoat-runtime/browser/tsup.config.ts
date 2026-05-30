import { defineConfig } from "tsup";

export default defineConfig({
	entry: ["src/index.ts"],
	format: ["esm"],

	noExternal: [/(.*)/],
	platform: "browser",
	target: "es2022",

	splitting: false,
	minify: true,
	sourcemap: true,
	clean: true,
});
