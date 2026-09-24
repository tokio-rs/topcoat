/** The media type of a framed render response: one JSON frame per line. */
export const FRAMES_MEDIA_TYPE = "application/x-ndjson";

/** A frame of rendered output sent by the server. */
export type ServerMessage =
	| { t: "run"; id: number }
	| { t: "snapshot"; html: string }
	| { t: "swap"; region: string; html: string }
	| { t: "redirect"; location: string }
	| { t: "error"; status: number };

/** Content that render frames update. */
export interface FrameTarget {
	/** Replaces the content with a render's initial HTML. */
	replaceContent(html: string): void;
	/** Updates one live region. */
	applySwap(region: string, html: string): void;
}

/**
 * Applies one frame of a render to `target`. A run announcement carries no
 * output and is ignored. `label` names the render in the error a failure
 * frame becomes.
 */
export function applyFrame(
	target: FrameTarget,
	frame: ServerMessage,
	label: string,
): void {
	switch (frame.t) {
		case "snapshot":
			target.replaceContent(frame.html);
			break;
		case "swap":
			target.applySwap(frame.region, frame.html);
			break;
		case "redirect":
			location.assign(frame.location);
			break;
		case "error":
			throw new Error(`${label} render failed: ${frame.status}`);
		case "run":
			break;
	}
}

/**
 * Reads the frames of a framed response as they arrive.
 *
 * A frame is complete once its line ends, so a partially received frame
 * waits for the rest of its bytes. Reading stops when the response ends or
 * the caller stops iterating.
 */
export async function* readFrames(
	response: Response,
): AsyncGenerator<ServerMessage> {
	const body = response.body;
	if (body === null) {
		yield* parseLines((await response.text()).split("\n"));
		return;
	}
	const reader = body.getReader();
	const decoder = new TextDecoder();
	let pending = "";
	try {
		for (;;) {
			const { done, value } = await reader.read();
			pending += decoder.decode(value, { stream: !done });
			const lines = pending.split("\n");
			pending = lines.pop() ?? "";
			yield* parseLines(lines);
			if (done) break;
		}
	} finally {
		reader.releaseLock();
	}
	yield* parseLines([pending]);
}

/** Parses each non-empty line as a frame. */
function* parseLines(lines: string[]): Generator<ServerMessage> {
	for (const line of lines) {
		if (line.trim() === "") continue;
		yield JSON.parse(line) as ServerMessage;
	}
}
