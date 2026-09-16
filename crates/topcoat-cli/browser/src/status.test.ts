// @vitest-environment happy-dom
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { StatusIndicator } from "./status";

beforeEach(() => {
	document.body.innerHTML = "";
	vi.stubGlobal(
		"FontFace",
		class {
			load() {
				return Promise.resolve(this);
			}
		},
	);
	vi.spyOn(document.fonts, "add").mockReturnValue(document.fonts);
});

afterEach(() => {
	vi.restoreAllMocks();
	vi.unstubAllGlobals();
});

it("updates the pill, allows dismissal and shows the next build event", () => {
	const status = new StatusIndicator(true);
	status.show("rebuilding");
	const host = document.querySelector("topcoat-dev-status")!;
	const shadow = host.shadowRoot!;
	expect(shadow.querySelector("b")?.textContent).toBe("rebuilding");
	status.show("build failed", true);
	expect(shadow.querySelector("b")?.className).toBe("error");
	expect((shadow.querySelector(".spinner") as HTMLElement).style.display).toBe(
		"none",
	);
	shadow.querySelector("button")!.click();
	expect(host.isConnected).toBe(false);
	status.show("rebuilding");
	expect(document.querySelector("topcoat-dev-status")).toBe(host);
	status.hide();
	expect(host.isConnected).toBe(false);
});

it("does not show an outdated status when the body becomes available", () => {
	const body = document.body;
	body.remove();
	try {
		const status = new StatusIndicator(true);
		status.show("rebuilding");
		status.hide();
		document.documentElement.append(body);
		document.dispatchEvent(new Event("DOMContentLoaded"));
		expect(document.querySelector("topcoat-dev-status")).toBeNull();
	} finally {
		if (!body.isConnected) document.documentElement.append(body);
	}
});

it("does nothing when the indicator is disabled", () => {
	const status = new StatusIndicator(false);
	status.show("build failed", true);
	expect(document.querySelector("topcoat-dev-status")).toBeNull();
	expect(document.fonts.add).not.toHaveBeenCalled();
});
