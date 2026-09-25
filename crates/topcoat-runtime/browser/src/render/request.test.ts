// @vitest-environment happy-dom
import { afterEach, expect, it, vi } from "vitest";

import type { ServerMessage } from "./frames";
import { RenderRequest } from "./request";

afterEach(() => vi.unstubAllGlobals());

function deferred<T>() {
	let resolve!: (value: T) => void;
	const promise = new Promise<T>((done) => {
		resolve = done;
	});
	return { promise, resolve };
}

/** Encodes a frame as the line the server sends it as. */
function line(frame: ServerMessage): string {
	return `${JSON.stringify(frame)}\n`;
}

const snapshot = (html: string): ServerMessage => ({ t: "snapshot", html });
const swap = (region: string, html: string): ServerMessage => ({
	t: "swap",
	region,
	html,
});

/** A response whose body the test feeds one chunk at a time. */
function streamed() {
	let controller!: ReadableStreamDefaultController<Uint8Array>;
	const body = new ReadableStream<Uint8Array>({
		start(c) {
			controller = c;
		},
	});
	const encoder = new TextEncoder();
	return {
		response: new Response(body),
		push: (text: string) => controller.enqueue(encoder.encode(text)),
		close: () => controller.close(),
	};
}

/** Lets the frames pushed so far reach the consumer. */
async function drain(): Promise<void> {
	await new Promise((resolve) => setTimeout(resolve, 0));
}

function fixture() {
	const lifetime = new AbortController();
	const reportError = vi.fn();
	const apply = vi.fn();
	const request = new RenderRequest(lifetime.signal, reportError);
	return { lifetime, reportError, apply, request };
}

it("applies each frame of a streamed response as it arrives", async () => {
	const { request, apply } = fixture();
	const { response, push, close } = streamed();
	const run = request.run(async () => response, apply, "Shard");

	push(line(snapshot("first")));
	await drain();
	expect(apply.mock.calls).toEqual([[snapshot("first")]]);

	// A frame split across chunks applies once its line is complete.
	const update = line(swap("aa", "second"));
	push(update.slice(0, 10));
	await drain();
	expect(apply).toHaveBeenCalledTimes(1);
	push(update.slice(10));
	await drain();
	expect(apply.mock.calls).toEqual([
		[snapshot("first")],
		[swap("aa", "second")],
	]);

	close();
	await run;
});

it("keeps applying frames after one of them cancels the request while replacing content", async () => {
	const { request, apply } = fixture();
	const { response, push, close } = streamed();
	// Replacing content cancels the request in flight, as a render unit
	// does, which must not stop the run's own remaining frames.
	apply.mockImplementation(() => request.cancel());
	const run = request.run(async () => response, apply, "Shard");

	push(line(snapshot("first")));
	await drain();
	push(line(swap("aa", "second")));
	close();
	await run;

	expect(apply.mock.calls).toEqual([
		[snapshot("first")],
		[swap("aa", "second")],
	]);
});

it("stops applying a run's frames once a newer run starts", async () => {
	const { request, apply } = fixture();
	const { response, push, close } = streamed();
	const first = request.run(async () => response, apply, "Shard");
	push(line(snapshot("old")));
	await drain();

	await request.run(
		async () => new Response(line(snapshot("new"))),
		apply,
		"Shard",
	);
	push(line(swap("aa", "stale")));
	close();
	await first;

	expect(apply.mock.calls).toEqual([[snapshot("old")], [snapshot("new")]]);
});

it("ignores a superseded redirect even when the request ignores cancellation", async () => {
	const { request, apply } = fixture();
	const assign = vi.fn();
	vi.stubGlobal("location", { assign });
	const response = deferred<Response>();
	const send = vi.fn((_signal: AbortSignal) => response.promise);
	const first = request.run(send, apply, "Shard");
	await request.run(
		async () => new Response(line(snapshot("new"))),
		apply,
		"Shard",
	);

	response.resolve({
		redirected: true,
		url: "/login",
	} as Response);
	await first;

	expect(send.mock.calls[0]?.[0]?.aborted).toBe(true);
	expect(assign).not.toHaveBeenCalled();
	expect(apply.mock.calls).toEqual([[snapshot("new")]]);
});

it("ignores an old response body that finishes after the next render", async () => {
	const { request, apply } = fixture();
	const body = deferred<string>();
	const response = new Response();
	vi.spyOn(response, "text").mockReturnValue(body.promise);
	const first = request.run(async () => response, apply, "Shard");
	await Promise.resolve();
	await request.run(
		async () => new Response(line(snapshot("new"))),
		apply,
		"Shard",
	);
	body.resolve(line(snapshot("old")));
	await first;

	expect(apply.mock.calls).toEqual([[snapshot("new")]]);
});

it("aborts on disposal and ignores the disposed owner's redirect", async () => {
	const { request, lifetime, apply } = fixture();
	const assign = vi.fn();
	vi.stubGlobal("location", { assign });
	const response = deferred<Response>();
	const send = vi.fn((_signal: AbortSignal) => response.promise);
	const pending = request.run(send, apply, "Shard");
	lifetime.abort();
	response.resolve({ redirected: true, url: "/login" } as Response);
	await pending;

	expect(send.mock.calls[0]?.[0].aborted).toBe(true);
	expect(assign).not.toHaveBeenCalled();
	expect(apply).not.toHaveBeenCalled();
});

it("batches scheduled requests and reports their failures once", async () => {
	const { request, reportError, apply } = fixture();
	const send = vi.fn(async () => new Response("", { status: 500 }));
	const refresh = () => request.run(send, apply, "Shard");
	request.schedule(refresh);
	request.schedule(refresh);
	await new Promise((resolve) => setTimeout(resolve, 0));

	expect(send).toHaveBeenCalledTimes(1);
	expect(reportError).toHaveBeenCalledExactlyOnceWith(
		expect.objectContaining({
			message: expect.stringContaining("Shard request failed: 500"),
		}),
	);
	expect(apply).not.toHaveBeenCalled();
});

it("does not start queued work after disposal", async () => {
	const { request, lifetime } = fixture();
	const refresh = vi.fn(async () => {});
	request.schedule(refresh);
	lifetime.abort();
	await Promise.resolve();

	expect(refresh).not.toHaveBeenCalled();
});
