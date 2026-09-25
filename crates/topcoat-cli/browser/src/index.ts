import { DevConnection, type DevEvent } from "./connection";
import { PageRefresh } from "./refresh";
import { StatusIndicator } from "./status";

function start(script: HTMLScriptElement): void {
	const status = new StatusIndicator(
		script.dataset.statusIndicator !== "false",
	);
	const connection = new DevConnection(script.src, {
		event: (event) => events[event](),
		reconnected: () => {
			void refresh.refresh();
		},
		navigating: () => refresh.cancel(),
	});
	const refresh = new PageRefresh(
		() => connection.isNavigating,
		() => status.hide(),
		(error) => {
			console.error("[topcoat dev]", error);
			status.show("refresh failed", true);
		},
	);
	const events: Record<DevEvent, () => void> = {
		reload: () => {
			void refresh.refresh();
		},
		rebuilding: () => {
			refresh.cancel();
			status.show("rebuilding");
		},
		"build-failed": () => status.show("build failed", true),
		"app-exited": () => status.show("app exited", true),
		"up-to-date": () => status.hide(),
	};
	connection.start();
}

// The bundle runs as a classic script so its own URL is available here.
const script = document.currentScript;
if (script instanceof HTMLScriptElement) start(script);
