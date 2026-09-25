// @vitest-environment happy-dom
import { afterEach, expect, it, vi } from "vitest";

import {
	Connection,
	type ConnectionTarget,
	RUNTIME_PROTOCOL,
} from "./connection";

afterEach(() => {
	vi.unstubAllGlobals();
	vi.useRealTimers();
});

/** A fake WebSocket whose events are controlled by the test. */
class FakeSocket extends EventTarget {
	readyState = 0;
	readonly sent: unknown[] = [];
	closedByClient = false;

	constructor(
		readonly url: string,
		readonly protocol: string,
	) {
		super();
	}

	send(data: string): void {
		this.sent.push(JSON.parse(data));
	}

	close(): void {
		this.closedByClient = true;
		this.readyState = 3;
		this.dispatchEvent(new Event("close"));
	}

	/** Simulates a successful connection. */
	open(): void {
		this.readyState = 1;
		this.dispatchEvent(new Event("open"));
	}

	/** Delivers a message as if it came from the server. */
	receive(message: unknown): void {
		this.dispatchEvent(
			new MessageEvent("message", { data: JSON.stringify(message) }),
		);
	}

	/** Simulates a lost connection. */
	drop(): void {
		this.readyState = 3;
		this.dispatchEvent(new Event("close"));
	}
}

function fixture() {
	const sockets: FakeSocket[] = [];
	const lifetime = new AbortController();
	const target = {
		signals: { a: 1 } as Record<string, unknown>,
		renderInputs: vi.fn(() => ({ signals: target.signals })),
		replaceContent: vi.fn(),
		applySwap: vi.fn(),
		reportError: vi.fn(),
	};
	const connection = new Connection(
		"ws://app.example/room?q=1",
		target as unknown as ConnectionTarget,
		lifetime.signal,
		(url, protocol) => {
			const socket = new FakeSocket(url, protocol);
			sockets.push(socket);
			return socket as unknown as WebSocket;
		},
	);
	const socket = () => {
		const current = sockets[sockets.length - 1];
		if (!current) throw new Error("No socket opened");
		return current;
	};
	return { connection, lifetime, sockets, socket, target };
}

it("opening the connection requests a run with the current signal values", () => {
	const { connection, socket } = fixture();
	expect(socket().url).toBe("ws://app.example/room?q=1");
	expect(socket().protocol).toBe(RUNTIME_PROTOCOL);
	expect(connection.isOpen).toBe(false);
	expect(socket().sent).toEqual([]);

	socket().open();

	expect(connection.isOpen).toBe(true);
	expect(socket().sent).toEqual([{ run: 1, signals: { a: 1 } }]);
});

it("a run's snapshot and swaps reach the target", () => {
	const { socket, target } = fixture();
	socket().open();

	socket().receive({ t: "run", id: 1 });
	socket().receive({ t: "snapshot", html: "<p>one</p>" });
	socket().receive({ t: "swap", region: "r", html: "<p>two</p>" });

	expect(target.replaceContent).toHaveBeenCalledExactlyOnceWith(
		"<p>one</p>",
		expect.any(Symbol),
	);
	expect(target.applySwap).toHaveBeenCalledExactlyOnceWith(
		"r",
		"<p>two</p>",
		expect.any(Symbol),
	);
	// The swap belongs to the render the snapshot came from.
	expect(target.applySwap.mock.calls[0]?.[2]).toBe(
		target.replaceContent.mock.calls[0]?.[1],
	);
});

it("each run's output belongs to its own render", () => {
	const { connection, socket, target } = fixture();
	socket().open();

	socket().receive({ t: "run", id: 1 });
	socket().receive({ t: "snapshot", html: "<p>one</p>" });
	connection.requestRun();
	socket().receive({ t: "run", id: 2 });
	socket().receive({ t: "snapshot", html: "<p>two</p>" });

	const renders = target.replaceContent.mock.calls.map((call) => call[1]);
	expect(renders).toHaveLength(2);
	expect(renders[0]).not.toBe(renders[1]);
});

it("the output of a superseded run is dropped until the latest run's output starts", () => {
	const { connection, socket, target } = fixture();
	socket().open();
	target.signals = { a: 2 };
	connection.requestRun();
	expect(socket().sent[1]).toEqual({ run: 2, signals: { a: 2 } });

	// Old output can still arrive before the server announces the new run.
	socket().receive({ t: "run", id: 1 });
	socket().receive({ t: "snapshot", html: "<p>stale</p>" });
	socket().receive({ t: "swap", region: "r", html: "<p>stale</p>" });
	socket().receive({ t: "run", id: 2 });
	socket().receive({ t: "snapshot", html: "<p>fresh</p>" });

	expect(target.replaceContent).toHaveBeenCalledExactlyOnceWith(
		"<p>fresh</p>",
		expect.any(Symbol),
	);
	expect(target.applySwap).not.toHaveBeenCalled();
});

it("a redirect navigates and an error is reported", () => {
	const { socket, target } = fixture();
	const assign = vi.fn();
	vi.stubGlobal("location", { assign });
	socket().open();

	socket().receive({ t: "run", id: 1 });
	socket().receive({ t: "error", status: 400 });
	socket().receive({ t: "redirect", location: "/login" });

	expect(target.reportError).toHaveBeenCalledExactlyOnceWith(
		expect.objectContaining({ message: "Connected render failed: 400" }),
	);
	expect(assign).toHaveBeenCalledExactlyOnceWith("/login");
});

it("a failure applying a message is reported and later messages still apply", () => {
	const { socket, target } = fixture();
	socket().open();
	target.replaceContent.mockImplementationOnce(() => {
		throw new Error("morph failed");
	});

	socket().receive({ t: "run", id: 1 });
	socket().receive({ t: "snapshot", html: "<p>bad</p>" });
	socket().receive({ t: "swap", region: "r", html: "<p>two</p>" });

	expect(target.reportError).toHaveBeenCalledExactlyOnceWith(
		expect.objectContaining({ message: "morph failed" }),
	);
	expect(target.applySwap).toHaveBeenCalledExactlyOnceWith(
		"r",
		"<p>two</p>",
		expect.any(Symbol),
	);
});

it("a dropped connection is reopened with a growing delay and requests a fresh run", () => {
	vi.useFakeTimers();
	const { connection, sockets, socket } = fixture();
	socket().open();

	socket().drop();
	expect(connection.isOpen).toBe(false);
	// Requests made while disconnected wait for the automatic render on reconnect.
	connection.requestRun();
	vi.advanceTimersByTime(999);
	expect(sockets).toHaveLength(1);
	vi.advanceTimersByTime(1);
	expect(sockets).toHaveLength(2);
	socket().drop();
	vi.advanceTimersByTime(1999);
	expect(sockets).toHaveLength(2);
	vi.advanceTimersByTime(1);
	expect(sockets).toHaveLength(3);

	socket().open();
	expect(socket().sent).toEqual([{ run: 2, signals: { a: 1 } }]);

	// After a successful connection, retries start at the shortest delay again.
	socket().drop();
	vi.advanceTimersByTime(1000);
	expect(sockets).toHaveLength(4);
});

it("aborting the lifetime closes the connection and stops reconnecting", () => {
	vi.useFakeTimers();
	const { connection, lifetime, sockets, socket } = fixture();
	socket().open();

	lifetime.abort();

	expect(socket().closedByClient).toBe(true);
	expect(connection.isOpen).toBe(false);
	vi.advanceTimersByTime(60_000);
	expect(sockets).toHaveLength(1);
});

it("aborting the lifetime while reconnecting cancels the reopening", () => {
	vi.useFakeTimers();
	const { lifetime, sockets, socket } = fixture();
	socket().open();
	socket().drop();

	lifetime.abort();

	vi.advanceTimersByTime(60_000);
	expect(sockets).toHaveLength(1);
});
