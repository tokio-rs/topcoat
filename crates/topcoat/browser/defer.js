const patchSelector = "template[data-topcoat-defer-patch]";
const resourceSelector = "template[data-topcoat-resource]";
const jsonSelector = "template[data-topcoat-json]";
const streamedSelector = [patchSelector, resourceSelector, jsonSelector].join(",");

const topcoat = (globalThis.topcoat ??= {});
const jsonValues = (topcoat.__jsonValues ??= new Map());
const jsonWaiters = (topcoat.__jsonWaiters ??= new Map());
const loadedResources = (topcoat.__loadedResources ??= new Set());

topcoat.json = function json(key) {
  if (jsonValues.has(key)) return Promise.resolve(jsonValues.get(key));
  return new Promise((resolve) => {
    const waiters = jsonWaiters.get(key) ?? [];
    waiters.push(resolve);
    jsonWaiters.set(key, waiters);
  });
};

function applyPatch(patch) {
  const id = patch.dataset.topcoatDeferPatch;
  const starts = document.querySelectorAll(
    `template[data-topcoat-defer-start="${id}"]`,
  );

  for (const start of starts) {
    let end = start.nextSibling;
    while (
      end &&
      !(
        end instanceof HTMLTemplateElement &&
        end.dataset.topcoatDeferEnd === id
      )
    ) {
      end = end.nextSibling;
    }
    if (!end) continue;

    let node = start.nextSibling;
    while (node !== end) {
      const next = node.nextSibling;
      node.remove();
      node = next;
    }
    end.replaceWith(patch.content.cloneNode(true));
    start.remove();
  }

  patch.remove();
}

function loadResource(template) {
  const { topcoatResource: kind, topcoatResourceKey: key } = template.dataset;
  if (!loadedResources.has(key)) {
    loadedResources.add(key);
    const resource = document.createElement(kind === "module" ? "script" : "link");
    resource.dataset.topcoatResourceKey = key;
    if (kind === "module") {
      resource.type = "module";
      resource.src = template.dataset.topcoatResourceSrc;
    } else {
      resource.rel = "stylesheet";
      resource.href = template.dataset.topcoatResourceSrc;
    }
    document.head.append(resource);
  }
  template.remove();
}

function receiveJson(template) {
  const key = template.dataset.topcoatJson;
  const value = JSON.parse(template.content.textContent);
  jsonValues.set(key, value);
  for (const resolve of jsonWaiters.get(key) ?? []) resolve(value);
  jsonWaiters.delete(key);
  template.remove();
}

function applyStreamed(template) {
  if (template.matches(patchSelector)) applyPatch(template);
  else if (template.matches(resourceSelector)) loadResource(template);
  else receiveJson(template);
}

function scan(node) {
  if (!(node instanceof Element)) return;
  if (node.matches(streamedSelector)) applyStreamed(node);
  node.querySelectorAll(streamedSelector).forEach(applyStreamed);
}

document
  .querySelectorAll("link[data-topcoat-resource-key],script[data-topcoat-resource-key]")
  .forEach((resource) => loadedResources.add(resource.dataset.topcoatResourceKey));
document.querySelectorAll(streamedSelector).forEach(applyStreamed);
new MutationObserver((records) => {
  for (const record of records) {
    record.addedNodes.forEach(scan);
  }
}).observe(document.documentElement, { childList: true, subtree: true });
