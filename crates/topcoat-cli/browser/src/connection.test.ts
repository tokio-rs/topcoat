// @vitest-environment happy-dom
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { DevConnection } from "./connection";

class Socket {
	static all: Socket[] = [];
	static at(index: number): Socket {
		const socket = Socket.all[index];
		if (!socket) throw new Error(`Missing socket ${index}`);
		return socket;
	}
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
	vi.stubGlobal("navigation", new EventTarget());
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
	const socket = Socket.at(0);
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
	Socket.at(0).close();
	vi.advanceTimersByTime(500);
	const socket = Socket.at(1);
	socket.onopen?.();
	socket.onmessage?.({ data: "reload" });
	vi.advanceTimersByTime(5000);
	expect(Socket.all).toHaveLength(2);
	expect(handlers.reconnected).toHaveBeenCalledOnce();
	expect(handlers.event).toHaveBeenCalledWith("reload");
});

it("defers reconnecting during navigation and resumes after a page is restored", () => {
	vi.stubGlobal("navigation", undefined);
	const handlers = start();
	window.dispatchEvent(new Event("pagehide"));
	Socket.at(0).close();
	vi.advanceTimersByTime(1000);
	expect(Socket.all).toHaveLength(1);
	expect(connection.isNavigating).toBe(true);
	expect(handlers.navigating).toHaveBeenCalledOnce();
	window.dispatchEvent(new Event("pageshow"));
	vi.advanceTimersByTime(500);
	Socket.at(1).onopen?.();
	expect(handlers.reconnected).toHaveBeenCalledOnce();
});

it("keeps reloads and reconnects active after same-document navigation", () => {
	const handlers = start();
	window.navigation.dispatchEvent(
		Object.assign(new Event("navigate"), {
			destination: { sameDocument: true },
		}),
	);

	expect(connection.isNavigating).toBe(false);
	expect(handlers.navigating).not.toHaveBeenCalled();
	Socket.at(0).onmessage?.({ data: "reload" });
	expect(handlers.event).toHaveBeenCalledWith("reload");
	Socket.at(0).close();
	vi.advanceTimersByTime(500);
	Socket.at(1).onopen?.();
	expect(handlers.reconnected).toHaveBeenCalledOnce();
});

it("defers reconnecting for a new document and resumes if navigation fails", () => {
	const handlers = start();
	window.navigation.dispatchEvent(
		Object.assign(new Event("navigate"), {
			destination: { sameDocument: false },
		}),
	);
	Socket.at(0).close();
	vi.advanceTimersByTime(1000);

	expect(connection.isNavigating).toBe(true);
	expect(handlers.navigating).toHaveBeenCalledOnce();
	expect(Socket.all).toHaveLength(1);
	window.navigation.dispatchEvent(new Event("navigateerror"));
	vi.advanceTimersByTime(500);
	Socket.at(1).onopen?.();
	expect(connection.isNavigating).toBe(false);
	expect(handlers.reconnected).toHaveBeenCalledOnce();
});

it("cancels a pending reconnect when stopped", () => {
	start();
	Socket.at(0).close();
	connection.stop();
	vi.advanceTimersByTime(1000);
	expect(Socket.all).toHaveLength(1);
});
