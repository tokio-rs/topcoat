import { signal, effect } from "https://esm.sh/@maverick-js/signals@6.0.0";

const signals = new Map();
const scopes = new Map();
const bindingsByScope = new Map();

const SIGNAL_RE = /^\s*signal:\s*(\{.*\})\s*$/;
const SCOPE_START_RE = /^\s*reactive scope start:\s*("[^"]+")\s+(\[[^\]]*\])\s+("[^"]*")\s*$/;
const SCOPE_END_RE = /^\s*reactive scope end:\s*("[^"]+")\s*$/;

let pendingScopes = new Set();
let flushScheduled = false;

function parseComment(node) {
	let m;
	if ((m = node.data.match(SIGNAL_RE))) {
		const data = JSON.parse(m[1]);
		return { kind: "signal", id: data.id, value: data.value };
	}
	if ((m = node.data.match(SCOPE_START_RE))) {
		return {
			kind: "scope-start",
			id: JSON.parse(m[1]),
			track: JSON.parse(m[2]),
			path: JSON.parse(m[3]),
		};
	}
	if ((m = node.data.match(SCOPE_END_RE))) {
		return { kind: "scope-end", id: JSON.parse(m[1]) };
	}
	return null;
}

function scan(rootNode, fromNode, toNode, initialScopeId) {
	const walker = document.createTreeWalker(
		rootNode,
		NodeFilter.SHOW_COMMENT | NodeFilter.SHOW_ELEMENT,
	);
	if (fromNode) walker.currentNode = fromNode;

	const scopeStack = [initialScopeId];
	const newScopes = [];
	const newBindings = [];

	for (let node = walker.nextNode(); node; node = walker.nextNode()) {
		if (toNode && node === toNode) break;

		const currentScopeId = scopeStack[scopeStack.length - 1];

		if (node.nodeType === Node.ELEMENT_NODE) {
			const sigId = node.dataset && node.dataset.topcoatBind;
			if (sigId) newBindings.push([node, sigId, currentScopeId]);
			continue;
		}

		const parsed = parseComment(node);
		if (!parsed) continue;

		if (parsed.kind === "signal") {
			const existing = signals.get(parsed.id);
			if (existing) {
				existing.sig.set(parsed.value);
			} else {
				signals.set(parsed.id, {
					sig: signal(parsed.value),
					scopeId: currentScopeId,
				});
			}
		} else if (parsed.kind === "scope-start") {
			scopes.set(parsed.id, {
				startNode: node,
				endNode: null,
				track: parsed.track,
				path: parsed.path,
				parentId: currentScopeId,
				abortController: null,
				disposeEffect: null,
			});
			scopeStack.push(parsed.id);
			newScopes.push(parsed.id);
		} else if (parsed.kind === "scope-end") {
			const scope = scopes.get(parsed.id);
			if (scope) scope.endNode = node;
			scopeStack.pop();
		}
	}

	for (const id of newScopes) setupEffect(id);
	for (const [el, sigId, scopeId] of newBindings) setupBinding(el, sigId, scopeId);
}

function setupBinding(el, sigId, scopeId) {
	const entry = signals.get(sigId);
	if (!entry) return;

	const isCheckbox = el.type === "checkbox" || el.type === "radio";
	const prop = isCheckbox ? "checked" : "value";
	const evt = isCheckbox ? "change" : "input";

	let writing = false;

	const onInput = () => {
		writing = true;
		try {
			entry.sig.set(el[prop]);
		} finally {
			writing = false;
		}
	};
	el.addEventListener(evt, onInput);

	const disposeEffect = effect(() => {
		const v = entry.sig();
		if (writing) return;
		if (el[prop] !== v) el[prop] = v;
	});

	const dispose = () => {
		el.removeEventListener(evt, onInput);
		disposeEffect();
	};

	const list = bindingsByScope.get(scopeId);
	if (list) list.push(dispose);
	else bindingsByScope.set(scopeId, [dispose]);
}

function setupEffect(scopeId) {
	const scope = scopes.get(scopeId);
	if (!scope || scope.disposeEffect) return;

	let first = true;
	scope.disposeEffect = effect(() => {
		for (const sigId of scope.track) {
			const s = signals.get(sigId);
			if (s) s.sig();
		}
		if (first) {
			first = false;
			return;
		}
		pendingScopes.add(scopeId);
		scheduleFlush();
	});
}

function scheduleFlush() {
	if (flushScheduled) return;
	flushScheduled = true;
	queueMicrotask(() => {
		flushScheduled = false;
		const pending = pendingScopes;
		pendingScopes = new Set();

		const outermost = [];
		for (const id of pending) {
			if (!scopes.has(id)) continue;
			let hasAncestor = false;
			let p = scopes.get(id).parentId;
			while (p) {
				if (pending.has(p)) {
					hasAncestor = true;
					break;
				}
				p = scopes.get(p)?.parentId;
			}
			if (!hasAncestor) outermost.push(id);
		}

		for (const id of outermost) fetchAndReplace(id);
	});
}

async function fetchAndReplace(scopeId) {
	const scope = scopes.get(scopeId);
	if (!scope) return;

	if (scope.abortController) scope.abortController.abort();
	const ac = new AbortController();
	scope.abortController = ac;

	const signalData = scope.track.map((id) => {
		const s = signals.get(id);
		return { id, value: s ? s.sig() : null };
	});
	const url =
		scope.path + "?signals=" + encodeURIComponent(JSON.stringify(signalData));

	let html;
	try {
		const res = await fetch(url, { signal: ac.signal });
		html = await res.text();
	} catch (e) {
		if (e.name === "AbortError") return;
		throw e;
	}

	if (!scopes.has(scopeId) || scope.abortController !== ac) return;
	scope.abortController = null;

	disposeScopeContent(scopeId);

	const parent = scope.startNode.parentNode;
	for (
		let n = scope.startNode.nextSibling;
		n && n !== scope.endNode;
		n = scope.startNode.nextSibling
	) {
		parent.removeChild(n);
	}

	const fragment = document.createRange().createContextualFragment(html);
	parent.insertBefore(fragment, scope.endNode);

	scan(parent, scope.startNode, scope.endNode, scopeId);
}

function disposeScopeContent(scopeId) {
	const descendants = new Set();
	let added = true;
	while (added) {
		added = false;
		for (const [id, s] of scopes) {
			if (id === scopeId || descendants.has(id)) continue;
			if (s.parentId === scopeId || descendants.has(s.parentId)) {
				descendants.add(id);
				added = true;
			}
		}
	}

	for (const id of descendants) {
		const s = scopes.get(id);
		if (s.disposeEffect) s.disposeEffect();
		if (s.abortController) s.abortController.abort();
		scopes.delete(id);
	}

	for (const [id, s] of signals) {
		if (s.scopeId === scopeId || descendants.has(s.scopeId)) {
			signals.delete(id);
		}
	}

	const toDisposeBindings = [scopeId, ...descendants];
	for (const sid of toDisposeBindings) {
		const list = bindingsByScope.get(sid);
		if (!list) continue;
		for (const dispose of list) dispose();
		bindingsByScope.delete(sid);
	}
}

scan(document.body, null, null, null);
