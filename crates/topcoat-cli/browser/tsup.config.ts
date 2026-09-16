import { defineConfig } from "tsup";

export default defineConfig({
	entry: ["src/index.ts"],
	// A classic script keeps document.currentScript available during startup.
	format: ["iife"],
	outExtension: () => ({ js: ".js" }),
	noExternal: [/(.*)/],
	platform: "browser",
	target: "es2022",
	splitting: false,
	minify: true,
	clean: true,
});
