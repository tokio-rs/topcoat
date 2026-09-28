import { morph } from "../../../../topcoat-core/browser/morph";
import { parseChildren } from "../dom/fragment";
import { compile, type Expression } from "../expression/compile";
import { dehydrate } from "../expression/dehydrate";
import { untrack } from "../reactivity";
import type { Runtime } from "../runtime";
import type { Scope } from "../scope";
import type { SignalId } from "../signal-registry";
import type { RerunRequest } from "./connection";
import { RenderUnit } from "./unit";

/**
 * A region bounded by shard start and end comments. When an input changes,
 * it requests new content from the endpoint with the current arguments and
 * signal values. Connected renders go to the same endpoint over the
 * document's connection.
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
		/** The request URL for shard renders, with route groups removed. */
		readonly path: string,
		readonly identity: string,
		exprs: string[],
		readonly startNode: Comment,
	) {
		super(parent, runtime);
		this.computes = exprs.map((js, index) =>
			compile(js, `shard ${path} argument ${index + 1}`),
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

	/**
	 * Posts to the endpoint with the invocation's identity, the current
	 * arguments, and the values of the signals the current content created,
	 * so the server resumes them instead of starting them over.
	 */
	rerunRequest(): RerunRequest {
		const { context } = this.runtime;
		const body = untrack(() => ({
			args: this.computes.map((compute) => dehydrate(compute(context))),
			signals: this.contentScope.collectSignalValues(),
		}));
		return {
			url: this.path,
			headers: {
				"Content-Type": "application/json",
				"X-Topcoat-Identity": this.identity,
			},
			body: JSON.stringify(body),
		};
	}

	protected override scheduleConnection(): void {
		// The enclosing content may still be scanning, so wait until its
		// connection requirements are known.
		queueMicrotask(() => super.scheduleConnection());
	}

	protected prepare(html: string): Node[] | null {
		const parent = this.startNode.parentNode;
		if (this.endNode === null || parent === null) return null;
		return parseChildren(parent, html);
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
