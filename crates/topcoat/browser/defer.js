const patchSelector = "template[data-topcoat-defer-patch]";

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

function scan(node) {
  if (!(node instanceof Element)) return;
  if (node.matches(patchSelector)) applyPatch(node);
  node.querySelectorAll(patchSelector).forEach(applyPatch);
}

document.querySelectorAll(patchSelector).forEach(applyPatch);
new MutationObserver((records) => {
  for (const record of records) {
    record.addedNodes.forEach(scan);
  }
}).observe(document.documentElement, { childList: true, subtree: true });
