/**
 * The name of the window event the dev client dispatches to ask the loaded
 * runtime to take part in a refresh. The event's detail is a
 * {@link DevRuntimeDetail}.
 */
export const DEV_RUNTIME_EVENT = "topcoat:dev-runtime:v1";

/** The hooks the runtime gives the dev client for refreshing the page. */
export interface DevRuntime {
	/** Renders the current page with its browser-held signal values. */
	request(signal: AbortSignal): Promise<Response>;
	/** Releases bindings, updates the document, then hydrates it again. */
	replace(update: () => void): void;
}

/**
 * The detail of a {@link DEV_RUNTIME_EVENT} event. A loaded runtime fills in
 * `runtime` so the dev client can use it.
 */
export interface DevRuntimeDetail {
	runtime?: DevRuntime;
}
