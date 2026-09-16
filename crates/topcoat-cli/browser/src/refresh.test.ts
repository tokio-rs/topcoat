// @vitest-environment happy-dom
// @vitest-environment-options {"happyDOM":{"settings":{"disableCSSFileLoading":true,"handleDisabledFileLoadingAsSuccess":true}}}
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { PageRefresh } from "./refresh";

const reload = vi.fn();
const assign = vi.fn();

beforeEach(() => {
	document.documentElement.innerHTML = "<head></head><body><p>old</p></body>";
	vi.spyOn(document, "readyState", "get").mockReturnValue("complete");
	vi.spyOn(window, "scrollTo").mockImplementation(() => {});
	vi.stubGlobal("location", {
		href: "http://localhost/search?q=x",
		pathname: "/search",
		search: "?q=x",
		reload,
		assign,
	});
});

afterEach(() => {
	vi.restoreAllMocks();
	vi.unstubAllGlobals();
	vi.clearAllMocks();
});

function response(body: string, head = ""): Response {
	const doctype = document.doctype ? "<!doctype html>" : "";
	return new Response(
		`${doctype}<html lang="en"><head>${head}</head><body>${body}</body></html>`,
		{ headers: { "Content-Type": "text/html; charset=utf-8" } },
	);
}

function fixture() {
	const beforeUpdate = vi.fn();
	const reportError = vi.fn();
	let navigating = false;
	const refresh = new PageRefresh(() => navigating, beforeUpdate, reportError);
	return {
		refresh,
		beforeUpdate,
		reportError,
		navigate: () => {
			navigating = true;
			refresh.cancel();
		},
	};
}

function deferred<T>() {
	let resolve!: (value: T) => void;
	const promise = new Promise<T>((done) => {
		resolve = done;
	});
	return { promise, resolve };
}

it("morphs a page without the runtime, preserving controls and updating its head", async () => {
	document.head.innerHTML =
		'<title>Old</title><link rel="stylesheet" href="/old.css">';
	document.body.innerHTML = '<input id="name" value="default"><p>old</p>';
	const input = document.querySelector("input") as HTMLInputElement;
	input.value = "typed";
	input.focus();
	const fetch = vi
		.fn()
		.mockResolvedValue(
			response(
				'<input id="name" value="server"><p>new</p>',
				'<title>New</title><link rel="stylesheet" href="/new.css">',
			),
		);
	vi.stubGlobal("fetch", fetch);
	const { refresh, reportError } = fixture();

	await refresh.refresh();

	expect(reportError).not.toHaveBeenCalled();
	expect(reload).not.toHaveBeenCalled();
	expect(fetch).toHaveBeenCalledWith(
		"http://localhost/search?q=x",
		expect.objectContaining({ cache: "no-store" }),
	);
	expect(document.querySelector("input")).toBe(input);
	expect(document.activeElement).toBe(input);
	expect(input.value).toBe("typed");
	expect(document.title).toBe("New");
	expect(document.querySelector("link")?.getAttribute("href")).toBe("/new.css");
	expect(document.documentElement.lang).toBe("en");
	expect(document.querySelector("p")?.textContent).toBe("new");
});

it("keeps unchanged scripts and reloads when executable resources change", async () => {
	document.head.innerHTML = '<script src="/app.js"></script>';
	const script = document.querySelector("script");
	const fetch = vi
		.fn()
		.mockResolvedValueOnce(
			response("<p>new</p>", '<script src="/app.js"></script>'),
		)
		.mockResolvedValueOnce(
			response("<p>later</p>", '<script src="/changed.js"></script>'),
		);
	vi.stubGlobal("fetch", fetch);
	const { refresh } = fixture();
	await refresh.refresh();
	expect(document.querySelector("script")).toBe(script);
	expect(reload).not.toHaveBeenCalled();
	await refresh.refresh();
	expect(reload).toHaveBeenCalledOnce();
	expect(document.querySelector("p")?.textContent).toBe("new");
});

it("ignores superseded responses even if fetch ignores cancellation", async () => {
	const pending = deferred<Response>();
	const fetch = vi
		.fn()
		.mockReturnValueOnce(pending.promise)
		.mockResolvedValueOnce(response("<p>latest</p>"));
	vi.stubGlobal("fetch", fetch);
	const { refresh } = fixture();
	const first = refresh.refresh();
	await Promise.resolve();
	await refresh.refresh();
	pending.resolve({ redirected: true, url: "/stale-login" } as Response);
	await first;
	expect(fetch.mock.calls[0]?.[1].signal.aborted).toBe(true);
	expect(assign).not.toHaveBeenCalled();
	expect(document.querySelector("p")?.textContent).toBe("latest");
});

it("does not update the page over a navigation", async () => {
	const pending = deferred<Response>();
	vi.stubGlobal("fetch", vi.fn().mockReturnValue(pending.promise));
	const { refresh, navigate } = fixture();
	const task = refresh.refresh();
	await Promise.resolve();
	navigate();
	pending.resolve(response("<p>new</p>"));
	await task;
	expect(document.querySelector("p")?.textContent).toBe("old");
	expect(reload).not.toHaveBeenCalled();
});

it("keeps the current page and reports a failed refresh", async () => {
	vi.stubGlobal(
		"fetch",
		vi.fn().mockResolvedValue(new Response("error", { status: 500 })),
	);
	const { refresh, reportError } = fixture();
	await refresh.refresh();
	expect(document.querySelector("p")?.textContent).toBe("old");
	expect(reportError).toHaveBeenCalledOnce();
	expect(reload).not.toHaveBeenCalled();
});
