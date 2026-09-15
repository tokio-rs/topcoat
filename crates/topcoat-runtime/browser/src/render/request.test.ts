// @vitest-environment happy-dom
import { afterEach, expect, it, vi } from "vitest";

import { RenderRequest } from "./request";

afterEach(() => vi.unstubAllGlobals());

function deferred<T>() {
	let resolve!: (value: T) => void;
	const promise = new Promise<T>((done) => {
		resolve = done;
	});
	return { promise, resolve };
}

function fixture() {
	const lifetime = new AbortController();
	const reportError = vi.fn();
	const replace = vi.fn();
	const request = new RenderRequest(lifetime.signal, reportError);
	return { lifetime, reportError, replace, request };
}

it("ignores a superseded redirect even when the request ignores cancellation", async () => {
	const { request, replace } = fixture();
	const assign = vi.fn();
	vi.stubGlobal("location", { assign });
	const response = deferred<Response>();
	const send = vi.fn((_signal: AbortSignal) => response.promise);
	const first = request.run(send, replace, "Shard");
	await request.run(async () => new Response("new"), replace, "Shard");

	response.resolve({
		redirected: true,
		url: "/login",
	} as Response);
	await first;

	expect(send.mock.calls[0]?.[0]?.aborted).toBe(true);
	expect(assign).not.toHaveBeenCalled();
	expect(replace.mock.calls).toEqual([["new"]]);
});

it("ignores an old response body that finishes after the next render", async () => {
	const { request, replace } = fixture();
	const body = deferred<string>();
	const response = new Response();
	vi.spyOn(response, "text").mockReturnValue(body.promise);
	const first = request.run(async () => response, replace, "Shard");
	await Promise.resolve();
	await request.run(async () => new Response("new"), replace, "Shard");
	body.resolve("old");
	await first;

	expect(replace.mock.calls).toEqual([["new"]]);
});

it("aborts on disposal and ignores the disposed owner's redirect", async () => {
	const { request, lifetime, replace } = fixture();
	const assign = vi.fn();
	vi.stubGlobal("location", { assign });
	const response = deferred<Response>();
	const send = vi.fn((_signal: AbortSignal) => response.promise);
	const pending = request.run(send, replace, "Shard");
	lifetime.abort();
	response.resolve({ redirected: true, url: "/login" } as Response);
	await pending;

	expect(send.mock.calls[0]?.[0].aborted).toBe(true);
	expect(assign).not.toHaveBeenCalled();
	expect(replace).not.toHaveBeenCalled();
});

it("batches scheduled requests and reports their failures once", async () => {
	const { request, reportError, replace } = fixture();
	const send = vi.fn(async () => new Response("", { status: 500 }));
	const refresh = () => request.run(send, replace, "Shard");
	request.schedule(refresh);
	request.schedule(refresh);
	await new Promise((resolve) => setTimeout(resolve, 0));

	expect(send).toHaveBeenCalledTimes(1);
	expect(reportError).toHaveBeenCalledExactlyOnceWith(
		expect.objectContaining({
			message: expect.stringContaining("Shard request failed: 500"),
		}),
	);
	expect(replace).not.toHaveBeenCalled();
});

it("does not start queued work after disposal", async () => {
	const { request, lifetime } = fixture();
	const refresh = vi.fn(async () => {});
	request.schedule(refresh);
	lifetime.abort();
	await Promise.resolve();

	expect(refresh).not.toHaveBeenCalled();
});
