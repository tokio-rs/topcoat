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

	async run(
		request: (signal: AbortSignal) => Promise<Response>,
		replace: (html: string) => void,
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
			const html = await response.text();
			if (!current()) return;
			this.controller = null;
			replace(html);
		} catch (error) {
			if (controller.signal.aborted || this.lifetime.aborted) return;
			throw error;
		} finally {
			if (this.controller === controller) this.controller = null;
		}
	}
}
