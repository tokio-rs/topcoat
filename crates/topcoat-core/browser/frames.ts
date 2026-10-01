/** The media type of a framed render response: one JSON frame per line. */
export const FRAMES_MEDIA_TYPE = "application/x-ndjson";

/** A frame of rendered output sent by the server. */
export type ServerMessage =
	| { t: "snapshot"; html: string }
	| { t: "swap"; region: string; html: string }
	| { t: "redirect"; location: string }
	| { t: "error"; status: number };

/** Reads complete frames as they arrive, retaining only unfinished lines. */
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
