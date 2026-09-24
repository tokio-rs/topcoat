import { morph } from "../../../../topcoat-core/browser/morph";
import { parseChildren } from "../dom/fragment";
import { compile, type Expression } from "../expression/compile";
import { dehydrate } from "../expression/dehydrate";
import type { DehydratedSurrogate } from "../expression/serialized";
import { untrack } from "../reactivity";
import type { Runtime } from "../runtime";
import type { Scope } from "../scope";
import type { SignalId } from "../signal-registry";
import { RenderUnit } from "./unit";

/**
 * A region bounded by shard start and end comments. When an input changes,
 * it requests new content from the endpoint with the current arguments and
 * signal values. A connection opens at the endpoint's URL.
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

	protected url(): string {
		return this.path;
	}

	/**
	 * Collects the current arguments and the values of the signals the
	 * current content created, so the server resumes them instead of
	 * starting them over.
	 */
	private endpointBody(): {
		args: DehydratedSurrogate[];
		signals: Record<SignalId, DehydratedSurrogate>;
	} {
		const { context } = this.runtime;
		return untrack(() => ({
			args: this.computes.map((compute) => dehydrate(compute(context))),
			signals: this.contentScope.collectSignalValues(),
		}));
	}

	/** Names the invocation alongside the endpoint's body. */
	renderInputs(): object {
		return { ...this.endpointBody(), shard: this.identity };
	}

	protected override scheduleConnection(): void {
		// The enclosing content may still be scanning, so wait until its
		// connection requirements are known.
		queueMicrotask(() => super.scheduleConnection());
	}

	protected request(signal: AbortSignal, accept: string): Promise<Response> {
		return fetch(this.path, {
			method: "POST",
			headers: {
				Accept: accept,
				"Content-Type": "application/json",
				"X-Topcoat-Identity": this.identity,
			},
			body: JSON.stringify(this.endpointBody()),
			signal,
		});
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
