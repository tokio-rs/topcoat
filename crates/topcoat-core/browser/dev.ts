/** The dev client asks the loaded runtime to participate in a refresh. */
export const DEV_RUNTIME_EVENT = "topcoat:dev-runtime:v1";

export interface DevRuntime {
	/** Renders the current page with its browser-held signal values. */
	request(signal: AbortSignal): Promise<Response>;
	/** Releases bindings, updates the document, then hydrates it again. */
	replace(update: () => void): void;
}

export interface DevRuntimeDetail {
	runtime?: DevRuntime;
}
