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
	readonly sent: Record<string, unknown>[] = [];
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

	/** Delivers a frame of `run` as if it came from the server. */
	receive(run: number | undefined, frame: unknown): void {
		this.dispatchEvent(
			new MessageEvent("message", { data: JSON.stringify({ run, frame }) }),
		);
	}

	/** Simulates a lost connection. */
	drop(): void {
		this.readyState = 3;
		this.dispatchEvent(new Event("close"));
	}
}

/** A target posting to `url` with `body`, which runs whenever the connection opens. */
function target(url: string, body = "{}") {
	const self = {
		body,
		rerunRequest: vi.fn(() => ({
			url,
			headers: { "Content-Type": "application/json" },
			body: self.body,
		})),
		replaceContent: vi.fn(),
		applySwap: vi.fn(),
		reportError: vi.fn(),
		connectionOpened: vi.fn(() => connection?.run(self)),
	};
	let connection: Connection | undefined;
	return {
		self,
		bind(to: Connection) {
			connection = to;
			return self as unknown as ConnectionTarget;
		},
	};
}

function fixture() {
	const sockets: FakeSocket[] = [];
	const reportError = vi.fn();
	const connection = new Connection(
		() => "https://app.example/room?q=1",
		reportError,
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
	const page = target("https://app.example/room?q=1", '{"signals":{}}');
	const shard = target("/feed", '{"args":[]}');
	return {
		connection,
		sockets,
		socket,
		reportError,
		page: page.self,
		pageTarget: page.bind(connection),
		shard: shard.self,
		shardTarget: shard.bind(connection),
	};
}

it("the first member opens the socket at the page URL, and each member runs once it opens", () => {
	const { connection, sockets, socket, pageTarget, shardTarget } = fixture();
	expect(sockets).toHaveLength(0);

	connection.join(pageTarget);
	connection.join(shardTarget);

	expect(sockets).toHaveLength(1);
	expect(socket().url).toBe("wss://app.example/room?q=1");
	expect(socket().protocol).toBe(RUNTIME_PROTOCOL);
	expect(socket().sent).toEqual([]);

	socket().open();

	expect(socket().sent).toEqual([
		{
			run: 1,
			method: "POST",
			path: "/room?q=1",
			headers: { "Content-Type": "application/json" },
			body: '{"signals":{}}',
		},
		{
			run: 2,
			method: "POST",
			path: "/feed",
			headers: { "Content-Type": "application/json" },
			body: '{"args":[]}',
		},
	]);
});

it("a member joining an open connection runs right away", () => {
	const { connection, socket, pageTarget, shardTarget } = fixture();
	connection.join(pageTarget);
	socket().open();

	connection.join(shardTarget);

	expect(socket().sent.map((message) => message.path)).toEqual([
		"/room?q=1",
		"/feed",
	]);
});

it("frames of interleaved runs reach their own targets", () => {
	const { connection, socket, page, pageTarget, shard, shardTarget } =
		fixture();
	connection.join(pageTarget);
	connection.join(shardTarget);
	socket().open();

	socket().receive(1, { t: "snapshot", html: "<p>page</p>" });
	socket().receive(2, { t: "snapshot", html: "<p>shard</p>" });
	socket().receive(1, { t: "swap", region: "r", html: "<p>page swap</p>" });

	expect(page.replaceContent).toHaveBeenCalledExactlyOnceWith(
		"<p>page</p>",
		expect.any(Symbol),
	);
	expect(shard.replaceContent).toHaveBeenCalledExactlyOnceWith(
		"<p>shard</p>",
		expect.any(Symbol),
	);
	expect(page.applySwap).toHaveBeenCalledExactlyOnceWith(
		"r",
		"<p>page swap</p>",
		expect.any(Symbol),
	);
	expect(shard.applySwap).not.toHaveBeenCalled();
	// A swap belongs to the render its run's snapshot came from.
	expect(page.applySwap.mock.calls[0]?.[2]).toBe(
		page.replaceContent.mock.calls[0]?.[1],
	);
	expect(page.replaceContent.mock.calls[0]?.[1]).not.toBe(
		shard.replaceContent.mock.calls[0]?.[1],
	);
});

it("a new run of a target stops its previous run, whose later output is dropped", () => {
	const { connection, socket, page, pageTarget, shardTarget } = fixture();
	connection.join(pageTarget);
	connection.join(shardTarget);
	socket().open();
	page.body = '{"signals":{"a":2}}';

	expect(connection.run(pageTarget)).toBe(true);

	expect(socket().sent.slice(2)).toEqual([
		{ stop: 1 },
		expect.objectContaining({ run: 3, body: '{"signals":{"a":2}}' }),
	]);
	socket().receive(1, { t: "snapshot", html: "<p>stale</p>" });
	socket().receive(3, { t: "snapshot", html: "<p>fresh</p>" });
	expect(page.replaceContent).toHaveBeenCalledExactlyOnceWith(
		"<p>fresh</p>",
		expect.any(Symbol),
	);
});

it("stopping a target's run drops its output and leaves other runs alone", () => {
	const { connection, socket, page, pageTarget, shard, shardTarget } =
		fixture();
	connection.join(pageTarget);
	connection.join(shardTarget);
	socket().open();

	connection.stop(shardTarget);
	connection.stop(shardTarget);

	expect(socket().sent.slice(2)).toEqual([{ stop: 2 }]);
	socket().receive(2, { t: "snapshot", html: "<p>shard</p>" });
	socket().receive(1, { t: "snapshot", html: "<p>page</p>" });
	expect(shard.replaceContent).not.toHaveBeenCalled();
	expect(page.replaceContent).toHaveBeenCalledOnce();
});

it("a run cannot start while the socket is not open", () => {
	const { connection, socket, pageTarget } = fixture();
	connection.join(pageTarget);

	expect(connection.run(pageTarget)).toBe(false);
	expect(socket().sent).toEqual([]);
});

it("a redirect navigates, a run's error goes to its target, and a rejected message is reported", () => {
	const { connection, socket, page, pageTarget, reportError } = fixture();
	const assign = vi.fn();
	vi.stubGlobal("location", { assign, href: "https://app.example/room" });
	connection.join(pageTarget);
	socket().open();

	socket().receive(1, { t: "error", status: 403 });
	socket().receive(1, { t: "redirect", location: "/login" });
	socket().receive(undefined, { t: "error", status: 400 });

	expect(page.reportError).toHaveBeenCalledExactlyOnceWith(
		expect.objectContaining({ message: expect.stringContaining("403") }),
	);
	expect(assign).toHaveBeenCalledExactlyOnceWith("/login");
	expect(reportError).toHaveBeenCalledExactlyOnceWith(
		expect.objectContaining({ message: expect.stringContaining("400") }),
	);
});

it("a failure applying a frame is reported and later frames still apply", () => {
	const { connection, socket, page, pageTarget } = fixture();
	connection.join(pageTarget);
	socket().open();
	page.replaceContent.mockImplementationOnce(() => {
		throw new Error("morph failed");
	});

	socket().receive(1, { t: "snapshot", html: "<p>bad</p>" });
	socket().receive(1, { t: "swap", region: "r", html: "<p>two</p>" });

	expect(page.reportError).toHaveBeenCalledExactlyOnceWith(
		expect.objectContaining({ message: "morph failed" }),
	);
	expect(page.applySwap).toHaveBeenCalledOnce();
});

it("a dropped connection is reopened with a growing delay and its members run again", () => {
	vi.useFakeTimers();
	const { connection, sockets, socket, page, pageTarget } = fixture();
	connection.join(pageTarget);
	socket().open();

	socket().drop();
	expect(connection.isOpen).toBe(false);
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
	expect(page.connectionOpened).toHaveBeenCalledTimes(2);
	expect(socket().sent).toEqual([expect.objectContaining({ run: 2 })]);
	// The server dropped the old run with its connection.
	socket().receive(1, { t: "snapshot", html: "<p>stale</p>" });
	expect(page.replaceContent).not.toHaveBeenCalled();

	// After a successful connection, retries start at the shortest delay again.
	socket().drop();
	vi.advanceTimersByTime(1000);
	expect(sockets).toHaveLength(4);
});

it("the socket closes once the last member leaves, unless another joins in the same task", async () => {
	vi.useFakeTimers();
	const { connection, sockets, socket, pageTarget, shardTarget } = fixture();
	connection.join(pageTarget);
	socket().open();

	connection.leave(pageTarget);
	connection.join(shardTarget);
	await Promise.resolve();
	expect(socket().closedByClient).toBe(false);

	connection.remove(shardTarget);
	await Promise.resolve();
	expect(socket().closedByClient).toBe(true);
	vi.advanceTimersByTime(60_000);
	expect(sockets).toHaveLength(1);
});

it("the last member leaving while reconnecting cancels the reopening", async () => {
	vi.useFakeTimers();
	const { connection, sockets, socket, pageTarget } = fixture();
	connection.join(pageTarget);
	socket().open();
	socket().drop();

	connection.leave(pageTarget);
	await Promise.resolve();

	vi.advanceTimersByTime(60_000);
	expect(sockets).toHaveLength(1);
});
