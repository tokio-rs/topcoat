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
 * whose inputs are its URL and its dependencies. A re-run is requested from
 * the pages route and arrives as a full document, whose body is morphed
 * into the children of `<body>`; the head is left alone.
 */
export class PageUnit extends RenderUnit {
	protected readonly label = "Page";

	constructor(runtime: Runtime) {
		super(null, runtime);
	}

	protected readInputs(): void {}

	protected request(signal: AbortSignal): Promise<Response> {
		// The page owns every signal in the document, directly or through a
		// shard, so its values are the complete set the re-run resumes from.
		const signals = untrack(() => this.contentScope.collectSignalValues());
		// The root page is served at the bare prefix, since the route below
		// it needs at least one path segment.
		const path = location.pathname === "/" ? "" : location.pathname;
		return fetch(`${PAGE_ROUTE_PREFIX}${path}${location.search}`, {
			method: "POST",
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
