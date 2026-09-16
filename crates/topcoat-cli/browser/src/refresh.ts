import {
	DEV_RUNTIME_EVENT,
	type DevRuntimeDetail,
} from "../../../topcoat-core/browser/dev";
import { canMorph, morphDocument } from "./document";

/** Refreshes the page after a build, accepting only the latest response. */
export class PageRefresh {
	private controller: AbortController | null = null;

	constructor(
		private readonly navigating: () => boolean,
		private readonly beforeUpdate: () => void,
		private readonly reportError: (error: unknown) => void,
	) {}

	cancel(): void {
		this.controller?.abort();
		this.controller = null;
	}

	async refresh(): Promise<void> {
		this.cancel();
		if (this.navigating()) return;
		const controller = new AbortController();
		this.controller = controller;
		const url = location.href;
		const current = () =>
			this.controller === controller && !this.navigating() && location.href === url;
		try {
			// Deferred module scripts (including the runtime) start before this.
			await ready(controller.signal);
			if (!current()) return;
			const detail: DevRuntimeDetail = {};
			window.dispatchEvent(new CustomEvent(DEV_RUNTIME_EVENT, { detail }));
			const response = await (detail.runtime?.request(controller.signal) ??
				fetch(url, {
					cache: "no-store",
					headers: { Accept: "text/html" },
					signal: controller.signal,
				}));
			if (!current()) return;
			if (response.redirected) {
				location.assign(response.url);
				return;
			}
			if (!response.ok) throw new Error(`Refresh failed: ${response.status} ${response.statusText}`);
			if (response.headers.get("Content-Type")?.split(";")[0]?.trim() !== "text/html") {
				location.reload();
				return;
			}
			const html = await response.text();
			if (!current()) return;
			const next = new DOMParser().parseFromString(html, "text/html");
			if (!canMorph(next)) {
				location.reload();
				return;
			}
			this.beforeUpdate();
			const update = () => morphDocument(next);
			if (detail.runtime) detail.runtime.replace(update);
			else update();
		} catch (error) {
			if (current()) this.reportError(error);
		} finally {
			if (this.controller === controller) this.controller = null;
		}
	}
}

/** Waits for initial markup and deferred scripts, or cancellation. */
function ready(signal: AbortSignal): Promise<void> {
	if (document.readyState !== "loading" || signal.aborted) return Promise.resolve();
	return new Promise((resolve) => {
		const finish = () => {
			document.removeEventListener("DOMContentLoaded", finish);
			signal.removeEventListener("abort", finish);
			resolve();
		};
		document.addEventListener("DOMContentLoaded", finish, { once: true });
		signal.addEventListener("abort", finish, { once: true });
	});
}
