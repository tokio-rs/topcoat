import {
	DEV_RUNTIME_EVENT,
	type DevRuntimeDetail,
} from "../../../../topcoat-core/browser/dev";
import { morph } from "../../../../topcoat-core/browser/morph";
import { untrack } from "../reactivity";
import type { Runtime } from "../runtime";
import type { Scope } from "../scope";
import type { SignalId } from "../signal-registry";
import { RUNTIME_HEADER } from "./request";
import { RenderUnit } from "./unit";

/**
 * The outermost render unit. Requests use the current URL and signal values.
 * Responses contain a full document, but only the body's children are updated.
 */
export class PageUnit extends RenderUnit {
	protected readonly label = "Page";

	constructor(runtime: Runtime) {
		super(null, runtime);
	}

	protected readInputs(): void {}

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
		// Include descendant signals so the server can restore the whole page.
		const signals = untrack(() => this.contentScope.collectSignalValues());
		// The runtime header asks the server to rerun this URL as a GET
		// with the supplied signal values.
		const url = `${location.origin}${location.pathname}${location.search}`;
		return fetch(url, {
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
