import { morph } from "../dom/morph";
import { compile, type Expression } from "../expression/compile";
import { dehydrate } from "../expression/dehydrate";
import { untrack } from "../reactivity";
import type { Runtime } from "../runtime";
import type { Scope } from "../scope";
import type { SignalId } from "../signal-registry";
import { RenderUnit } from "./unit";

/** The route a shard re-run is requested from, with the shard's id appended. */
export const SHARD_ROUTE_PREFIX = "/_topcoat/runtime/shards";

/**
 * A shard: a region delimited by `<!-- ::topcoat::shard::start/end -->`
 * comments whose content is re-fetched from the shard's route with its
 * computed arguments whenever one of its inputs changes.
 */
export class ShardUnit extends RenderUnit {
	protected readonly label = "Shard";
	endNode: Comment | null = null;
	/**
	 * One compiled function per shard parameter, in declaration order. Each
	 * returns the parameter's current (surrogate) value; reading it inside an
	 * effect subscribes to whatever signals it touches.
	 */
	private readonly computes: Expression[];

	constructor(
		parent: Scope,
		runtime: Runtime,
		readonly shard: string,
		readonly identity: string,
		exprs: string[],
		readonly startNode: Comment,
	) {
		super(parent, runtime);
		this.computes = exprs.map((js, index) =>
			compile(js, `shard ${shard} argument ${index + 1}`),
		);
	}

	/** Must be called before `startWatching`. */
	attachEnd(end: Comment): void {
		this.endNode = end;
	}

	protected readInputs(): void {
		const { context } = this.runtime;
		for (const compute of this.computes) compute(context);
	}

	protected request(signal: AbortSignal): Promise<Response> {
		const { context } = this.runtime;
		// The signals the current content created travel with the request,
		// so the server resumes them instead of starting them over.
		const { args, signals } = untrack(() => ({
			args: this.computes.map((compute) => dehydrate(compute(context))),
			signals: this.contentScope.collectSignalValues(),
		}));
		return fetch(`${SHARD_ROUTE_PREFIX}/${this.shard}`, {
			method: "POST",
			headers: {
				"Content-Type": "application/json",
				"X-Topcoat-Identity": this.identity,
			},
			body: JSON.stringify({ args, signals }),
			signal,
		});
	}

	protected prepare(html: string): Node[] | null {
		if (this.endNode === null || this.startNode.parentNode === null) {
			return null;
		}
		const fragment = document.createRange().createContextualFragment(html);
		return Array.from(fragment.childNodes);
	}

	protected insert(
		nodes: Node[],
		scope: Scope,
		adoptable: Set<SignalId>,
	): void {
		const parent = this.startNode.parentNode;
		const end = this.endNode;
		if (parent === null || end === null) return;

		morph(parent, this.startNode, end, nodes);
		this.runtime.hydrate(parent, this.startNode, end, scope, adoptable);
	}
}
