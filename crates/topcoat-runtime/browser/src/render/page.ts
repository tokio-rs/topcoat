import {
	DEV_RUNTIME_EVENT,
	type DevRuntimeDetail,
} from "../../../../topcoat-core/browser/dev";
import { morph } from "../../../../topcoat-core/browser/morph";
import { untrack } from "../reactivity";
import type { Runtime } from "../runtime";
import type { Scope } from "../scope";
import type { SignalId } from "../signal-registry";
import type { RerunRequest } from "./connection";
import { newRender, type RenderToken } from "./frames";
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

	/**
	 * Posts to the page URL with the runtime header, which asks the server
	 * to rerun the page as a GET with the supplied signal values.
	 */
	rerunRequest(): RerunRequest {
		return {
			url: pageUrl(),
			headers: {
				"Content-Type": "application/json",
				[RUNTIME_HEADER]: "true",
			},
			body: this.signalBody(),
		};
	}

	/**
	 * Returns the JSON body carrying the document's signal values, which
	 * renders any page with the state the browser holds.
	 */
	signalBody(): string {
		// Include nested signals to preserve state across the whole page.
		const signals = untrack(() => this.contentScope.collectSignalValues());
		return JSON.stringify({ signals });
	}

	/**
	 * Replaces the whole document with `next`, a navigation's destination,
	 * which becomes the content of `render`. Keeps the signals the
	 * destination declares again and stops the page's connected render.
	 */
	replaceDocument(next: Document, render: RenderToken): void {
		if (this.isDisposed) return;
		this.runtime.connection.stop(this);
		// The focused element would otherwise survive the morph in place.
		const active = document.activeElement;
		if (active instanceof HTMLElement) active.blur();
		this.replace((scope, adoptable) => {
			const root = document.documentElement;
			const fresh = next.documentElement;
			for (const attr of Array.from(root.attributes)) {
				if (!fresh.hasAttribute(attr.name)) root.removeAttribute(attr.name);
			}
			for (const attr of Array.from(fresh.attributes)) {
				root.setAttribute(attr.name, attr.value);
			}
			morph(root, null, null, fresh.childNodes);
			this.runtime.hydrate(document, null, null, scope, adoptable);
		}, render);
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
export function pageUrl(): string {
	return `${location.origin}${location.pathname}${location.search}`;
}
