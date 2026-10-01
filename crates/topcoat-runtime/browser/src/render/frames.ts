import type { ServerMessage } from "../../../../topcoat-core/browser/frames";

export {
	FRAMES_MEDIA_TYPE,
	readFrames,
	type ServerMessage,
} from "../../../../topcoat-core/browser/frames";

/**
 * Identifies one render's output. Content remembers the render that
 * produced it, so a swap from an older render cannot overwrite content a
 * newer render of the same unit put in place.
 */
export type RenderToken = symbol;

/** Starts a new render's identity. */
export function newRender(): RenderToken {
	return Symbol("render");
}

/** Content that render frames update. */
export interface FrameTarget {
	/** Replaces the content with a render's initial HTML. */
	replaceContent(html: string, render: RenderToken): void;
	/** Updates one live region the same render produced. */
	applySwap(region: string, html: string, render: RenderToken): void;
}

/**
 * Applies one frame of the render identified by `render` to `target`.
 * `label` names the render in the error a failure frame becomes.
 */
export function applyFrame(
	target: FrameTarget,
	frame: ServerMessage,
	label: string,
	render: RenderToken,
): void {
	switch (frame.t) {
		case "snapshot":
			target.replaceContent(frame.html, render);
			break;
		case "swap":
			target.applySwap(frame.region, frame.html, render);
			break;
		case "redirect":
			location.assign(frame.location);
			break;
		case "error":
			throw new Error(`${label} render failed: ${frame.status}`);
	}
}
