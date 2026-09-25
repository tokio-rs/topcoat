import { readFrames, type ServerMessage } from "./frames";

/** Identifies a page rerun when set to "true" on a POST request. */
export const RUNTIME_HEADER = "X-Topcoat-Runtime";

/** Batches re-renders and accepts responses only while their owner is live. */
export class RenderRequest {
	private controller: AbortController | null = null;
	private pending = false;

	constructor(
		private readonly lifetime: AbortSignal,
		private readonly reportError: (error: unknown) => void,
	) {
		lifetime.addEventListener("abort", () => this.cancel(), { once: true });
	}

	cancel(): void {
		this.controller?.abort();
		this.controller = null;
	}

	schedule(task: () => Promise<void>): void {
		if (this.pending || this.lifetime.aborted) return;
		this.cancel();
		this.pending = true;
		queueMicrotask(() => {
			this.pending = false;
			if (this.lifetime.aborted) return;
			void task().catch(this.reportError);
		});
	}

	/**
	 * Sends a render request and applies each frame of its response as it
	 * arrives. A newer run, disposal, or a replacement of the content from
	 * elsewhere stops the remaining frames from applying.
	 */
	async run(
		request: (signal: AbortSignal) => Promise<Response>,
		apply: (frame: ServerMessage) => void,
		label: string,
	): Promise<void> {
		if (this.lifetime.aborted) return;
		this.cancel();
		const controller = new AbortController();
		this.controller = controller;
		const current = () =>
			!this.lifetime.aborted && this.controller === controller;

		try {
			const response = await request(controller.signal);
			if (!current()) return;
			if (response.redirected) {
				location.assign(response.url);
				return;
			}
			if (!response.ok) {
				throw new Error(
					`${label} request failed: ${response.status} ${response.statusText}`,
				);
			}
			for await (const frame of readFrames(response)) {
				if (!current()) return;
				// Applying a frame can replace the content, which cancels the
				// request in flight. Step aside for that cancel, then take the
				// run back unless something else started one meanwhile.
				this.controller = null;
				apply(frame);
				if (this.controller !== null) return;
				this.controller = controller;
			}
		} catch (error) {
			if (controller.signal.aborted || this.lifetime.aborted) return;
			throw error;
		} finally {
			if (this.controller === controller) this.controller = null;
		}
	}
}
