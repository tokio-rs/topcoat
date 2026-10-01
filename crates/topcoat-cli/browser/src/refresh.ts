import {
	DEV_RUNTIME_EVENT,
	type DevRuntimeDetail,
} from "../../../topcoat-core/browser/dev";
import {
	FRAMES_MEDIA_TYPE,
	readFrames,
} from "../../../topcoat-core/browser/frames";
import { canMorph, morphDocument, morphRegion } from "./document";

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
		// An initial HTTP stream can keep deferred scripts from ever starting.
		if (document.readyState === "loading") {
			location.reload();
			return;
		}
		const controller = new AbortController();
		this.controller = controller;
		let finish!: () => void;
		const finished = new Promise<void>((resolve) => {
			finish = resolve;
		});
		controller.signal.addEventListener("abort", finish, { once: true });
		const url = location.href;
		const current = () =>
			this.controller === controller &&
			!this.navigating() &&
			location.href === url;
		try {
			const detail: DevRuntimeDetail = {};
			window.dispatchEvent(new CustomEvent(DEV_RUNTIME_EVENT, { detail }));
			const response = await (detail.runtime?.request(
				controller.signal,
				finished,
			) ??
				fetch(url, {
					cache: "no-store",
					headers: { Accept: FRAMES_MEDIA_TYPE },
					signal: controller.signal,
				}));
			if (!current()) return;
			if (response.redirected) {
				location.assign(response.url);
				return;
			}
			if (!response.ok)
				throw new Error(
					`Refresh failed: ${response.status} ${response.statusText}`,
				);
			if (
				response.headers
					.get("Content-Type")
					?.split(";")[0]
					?.trim()
					.toLowerCase() !== FRAMES_MEDIA_TYPE
			) {
				location.reload();
				return;
			}
			let committed = false;
			for await (const frame of readFrames(response)) {
				if (!current()) return;
				switch (frame.t) {
					case "snapshot": {
						const next = new DOMParser().parseFromString(
							frame.html,
							"text/html",
						);
						if (!canMorph(next)) {
							location.reload();
							return;
						}
						this.beforeUpdate();
						const update = () => morphDocument(next);
						if (detail.runtime) detail.runtime.replace(update);
						else update();
						committed = true;
						break;
					}
					case "swap":
						if (!committed)
							throw new Error("Refresh update arrived before its page");
						if (detail.runtime) detail.runtime.swap(frame.region, frame.html);
						else morphRegion(frame.region, frame.html);
						break;
					case "redirect":
						location.assign(frame.location);
						return;
					case "error":
						throw new Error(`Refresh render failed: ${frame.status}`);
				}
			}
			if (current() && !committed)
				throw new Error("Refresh response contained no page");
		} catch (error) {
			if (current()) this.reportError(error);
		} finally {
			// Also stop the body on redirects, incompatible scripts and stale frames.
			controller.abort();
			finish();
			if (this.controller === controller) this.controller = null;
		}
	}
}
