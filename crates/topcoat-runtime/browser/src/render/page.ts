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
import { newRender } from "./frames";
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

	protected url(): string {
		return pageUrl();
	}

	renderInputs(): { signals: Record<SignalId, DehydratedSurrogate> } {
		// Include nested signals to preserve state across the whole page.
		return { signals: untrack(() => this.contentScope.collectSignalValues()) };
	}

	/** Lets a dev refresh update the whole document with this page's state. */
	listenForDevRefresh(): void {
		window.addEventListener(
			DEV_RUNTIME_EVENT,
			(event) => {
				const { detail } = event as CustomEvent<DevRuntimeDetail>;
				detail.runtime = {
					// The dev client reads the response as a document.
					request: (signal) => this.request(signal, "text/html"),
					replace: (update) => {
						this.replace((scope, adoptable) => {
							update();
							this.runtime.hydrate(document, null, null, scope, adoptable);
						}, newRender());
					},
				};
			},
			{ signal: this.lifetime.abortSignal },
		);
	}

	protected request(signal: AbortSignal, accept: string): Promise<Response> {
		// The runtime header asks the server to rerun this URL as a GET
		// with the supplied signal values.
		return fetch(pageUrl(), {
			method: "POST",
			cache: "no-store",
			headers: {
				Accept: accept,
				"Content-Type": "application/json",
				[RUNTIME_HEADER]: "true",
			},
			body: JSON.stringify(this.renderInputs()),
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
 * Returns the page URL without its fragment. Joining these parts keeps
 * paths starting with two slashes on the current origin.
 */
function pageUrl(): string {
	return `${location.origin}${location.pathname}${location.search}`;
}
