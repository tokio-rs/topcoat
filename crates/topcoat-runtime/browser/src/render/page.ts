import {
	DEV_RUNTIME_EVENT,
	type DevRuntimeDetail,
} from "../../../../topcoat-core/browser/dev";
import { morph } from "../../../../topcoat-core/browser/morph";
import { untrack } from "../reactivity";
import type { Runtime } from "../runtime";
import type { Scope } from "../scope";
import type { SignalId } from "../signal-registry";
import { RenderUnit } from "./unit";

/** The route a page re-run is requested from, with the page's path appended. */
export const PAGE_ROUTE_PREFIX = "/_topcoat/runtime/pages";

/**
 * The page: the outermost unit, whose content is the whole document and
 * whose inputs are its dependencies. A re-run requests the current URL from
 * the pages route and receives a full document. Its body is morphed into the
 * children of `<body>`, and the head is left alone.
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
		// The page owns every signal in the document, directly or through a
		// shard, so its values are the complete set the re-run resumes from.
		const signals = untrack(() => this.contentScope.collectSignalValues());
		// The root page is served at the bare prefix, since the route below
		// it needs at least one path segment.
		const path = location.pathname === "/" ? "" : location.pathname;
		return fetch(`${PAGE_ROUTE_PREFIX}${path}${location.search}`, {
			method: "POST",
			cache: "no-store",
			headers: { "Content-Type": "application/json" },
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
