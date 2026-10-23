import { configDefaults, defineConfig } from "vitest/config";

export default defineConfig({
	test: {
		includeSource: ["src/reactivity.ts"],
		exclude: [...configDefaults.exclude, "**/*.browser.test.ts"],
	},
});
