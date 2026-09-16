// @vitest-environment happy-dom
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { DevConnection } from "./connection";

class Socket {
	static all: Socket[] = [];
	onopen: (() => void) | null = null;
	onclose: (() => void) | null = null;
	onmessage: ((event: { data: unknown }) => void) | null = null;
	constructor(readonly url: string) {
		Socket.all.push(this);
	}
	close(): void {
		this.onclose?.();
	}
}

let connection: DevConnection;

beforeEach(() => {
	vi.useFakeTimers();
	vi.stubGlobal("WebSocket", Socket);
	Socket.all = [];
});

afterEach(() => {
	connection.stop();
	vi.useRealTimers();
	vi.unstubAllGlobals();
});

function start() {
	const handlers = {
		event: vi.fn(),
		reconnected: vi.fn(),
		navigating: vi.fn(),
	};
	connection = new DevConnection(
		"https://localhost:59039/dev.js?x=1#hash",
		handlers,
	);
	connection.start();
	return handlers;
}

it("uses the script's origin and only delivers known build events", () => {
	const handlers = start();
	const socket = Socket.all[0]!;
	expect(socket.url).toBe("wss://localhost:59039/ws");
	socket.onopen?.();
	socket.onmessage?.({ data: "rebuilding" });
	socket.onmessage?.({ data: "reload" });
	socket.onmessage?.({ data: "unexpected" });
	expect(handlers.event.mock.calls).toEqual([["rebuilding"], ["reload"]]);
	expect(handlers.reconnected).not.toHaveBeenCalled();
});

it("keeps the reconnected socket for subsequent builds", () => {
	const handlers = start();
	Socket.all[0]!.close();
	vi.advanceTimersByTime(500);
	const socket = Socket.all[1]!;
	socket.onopen?.();
	socket.onmessage?.({ data: "reload" });
	vi.advanceTimersByTime(5000);
	expect(Socket.all).toHaveLength(2);
	expect(handlers.reconnected).toHaveBeenCalledOnce();
	expect(handlers.event).toHaveBeenCalledWith("reload");
});

it("defers reconnecting during navigation and resumes after a page is restored", () => {
	const handlers = start();
	window.dispatchEvent(new Event("pagehide"));
	Socket.all[0]!.close();
	vi.advanceTimersByTime(1000);
	expect(Socket.all).toHaveLength(1);
	expect(connection.isNavigating).toBe(true);
	expect(handlers.navigating).toHaveBeenCalledOnce();
	window.dispatchEvent(new Event("pageshow"));
	vi.advanceTimersByTime(500);
	Socket.all[1]!.onopen?.();
	expect(handlers.reconnected).toHaveBeenCalledOnce();
});

it("cancels a pending reconnect when stopped", () => {
	start();
	Socket.all[0]!.close();
	connection.stop();
	vi.advanceTimersByTime(1000);
	expect(Socket.all).toHaveLength(1);
});
