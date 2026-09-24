import {
	DEV_RUNTIME_EVENT,
	type DevRuntimeDetail,
} from "../../../../topcoat-core/browser/dev";
import { morph } from "../../../../topcoat-core/browser/morph";
import type { DehydratedSurrogate } from "../expression/serialized";
import { untrack } from "../reactivity";
import type { Runtime } from "../runtime";
import type { Scope } from "../scope";
import type { SignalId } from "../signal-registry";
import { Connection, type ConnectionTarget } from "./connection";
import { RUNTIME_HEADER } from "./request";
import { RenderUnit } from "./unit";

/**
 * The outermost render unit. Requests use the current URL and signal values.
 * Responses contain a full document, but only the body's children are updated.
 *
 * Once the content asks for a connection, the page opens one at its own URL
 * and re-renders over it instead of posting. The connection stays open for
 * the rest of the page's life, whether or not later content still asks for
 * it.
 */
export class PageUnit extends RenderUnit implements ConnectionTarget {
	protected readonly label = "Page";
	private connection: Connection | null = null;

	constructor(runtime: Runtime) {
		super(null, runtime);
	}

	protected readInputs(): void {}

	override startWatching(): void {
		super.startWatching();
		this.connectIfRequired();
	}

	/**
	 * Opens the connection the content asks for. Waits for the document to
	 * finish loading first, so a swap still streaming in over HTTP cannot
	 * land after the connected snapshot.
	 */
	private connectIfRequired(loaded = document.readyState === "complete"): void {
		if (this.isDisposed || this.connection !== null) return;
		if (!this.requiresConnection) return;
		if (!loaded) {
			window.addEventListener("load", () => this.connectIfRequired(true), {
				once: true,
				signal: this.lifetime.abortSignal,
			});
			return;
		}
		this.connection = new Connection(
			connectionUrl(),
			this,
			this.lifetime.abortSignal,
		);
	}

	override refresh(): Promise<void> {
		// Over an open connection, a re-render is a new run on it.
		if (this.connection?.isOpen) {
			this.connection.requestRun();
			return Promise.resolve();
		}
		return super.refresh();
	}

	collectSignals(): Record<SignalId, DehydratedSurrogate> {
		// Include descendant signals so the server can restore the whole page.
		return untrack(() => this.contentScope.collectSignalValues());
	}

	reportError(error: unknown): void {
		this.runtime.reportError(error);
	}

	/** Lets a dev refresh update the whole document with this page's state. */
	listenForDevRefresh(): void {
		window.addEventListener(
			DEV_RUNTIME_EVENT,
			(event) => {
				const { detail } = event as CustomEvent<DevRuntimeDetail>;
				detail.runtime = {
					request: (signal) => this.request(signal),
					replace: (update) => {
						this.replace((scope, adoptable) => {
							update();
							this.runtime.hydrate(document, null, null, scope, adoptable);
						});
					},
				};
			},
			{ signal: this.lifetime.abortSignal },
		);
	}

	protected request(signal: AbortSignal): Promise<Response> {
		const signals = this.collectSignals();
		// The runtime header asks the server to rerun this URL as a GET
		// with the supplied signal values.
		return fetch(pageUrl(), {
			method: "POST",
			cache: "no-store",
			headers: {
				"Content-Type": "application/json",
				[RUNTIME_HEADER]: "true",
			},
			body: JSON.stringify({ signals }),
			signal,
		});
	}

	protected prepare(html: string): Node[] | null {
		const doc = new DOMParser().parseFromString(html, "text/html");
		return Array.from(doc.body.childNodes);
	}

	protected insert(
		nodes: Node[],
		scope: Scope,
		adoptable: Set<SignalId>,
	): void {
		morph(document.body, null, null, nodes);
		// The whole document, so declarations outside the body are adopted
		// again.
		this.runtime.hydrate(document, null, null, scope, adoptable);
	}
}

/**
 * The page's own URL without its fragment. Built from the parts rather
 * than resolved, so a path starting with two slashes stays on this origin.
 */
function pageUrl(): string {
	return `${location.origin}${location.pathname}${location.search}`;
}

/** The page's own URL as a WebSocket URL. */
function connectionUrl(): string {
	const url = new URL(pageUrl());
	url.protocol = url.protocol === "https:" ? "wss:" : "ws:";
	return url.href;
}
