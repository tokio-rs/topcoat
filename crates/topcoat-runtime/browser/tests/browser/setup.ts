import { execFileSync } from "node:child_process";
import { mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import type { TestProject } from "vitest/node";

export default function setup(project: TestProject) {
	const root = resolve(import.meta.dirname, "../../../../..");
	const directory = mkdtempSync(join(tmpdir(), "topcoat-write-revisions-"));
	const fixture = join(directory, "fixture.html");
	try {
		execFileSync(
			"cargo",
			[
				"test",
				"-p",
				"topcoat-runtime-macro",
				"--test",
				"signal_write_revisions",
				"--locked",
			],
			{
				cwd: root,
				env: {
					...process.env,
					CARGO_BUILD_JOBS: "2",
					CARGO_TARGET_DIR: join(root, "target"),
					TOPCOAT_REVISION_FIXTURE: fixture,
				},
				stdio: "inherit",
			},
		);
		project.provide("revisionFixture", readFileSync(fixture, "utf8"));
	} finally {
		rmSync(directory, { recursive: true, force: true });
	}
}

declare module "vitest" {
	export interface ProvidedContext {
		revisionFixture: string;
	}
}
