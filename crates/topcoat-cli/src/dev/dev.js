// Injected into every page served under `topcoat dev`. Reloads the page once
// a fresh build is serving, and mirrors the CLI status in a small floating
// pill in the meantime.
(() => {
  const script = document.currentScript;

  // The dev server's WebSocket endpoint lives on the same origin as this
  // script.
  const wsUrl = (() => {
    const url = new URL(script.src);
    url.protocol = url.protocol === "https:" ? "wss:" : "ws:";
    url.pathname = "/ws";
    url.search = "";
    url.hash = "";
    return url.href;
  })();

  // --- Status indicator -----------------------------------------------------
  //
  // A small floating pill mirroring the CLI status: a spinner while the dev
  // server is rebuilding, and an error label when a build fails or the app
  // exits. Disabled by rendering the script tag with
  // `data-status-indicator="false"`.
  //
  // The pill renders into a shadow tree on a custom host element, so page
  // styles can't restyle it, its own stylesheet never applies to the page,
  // and nothing is added to the page's <head>.

  const enabled = script.dataset.statusIndicator !== "false";

  const MUTED = "#a1a1aa";
  const ERROR = "#fca5a5";

  // Lexend Deca, registered under a private family name so it can never
  // collide with a "Lexend Deca" the page itself uses. Fonts are document
  // scoped (a @font-face inside a shadow tree is not reliably loaded), hence
  // the FontFace API rather than a rule in the pill's stylesheet. Loading
  // eagerly lets the pill render in its final font the moment it appears.
  const FONT = "Topcoat Dev";
  const FONT_URL =
    "https://cdn.jsdelivr.net/fontsource/fonts/lexend-deca@latest/latin-";
  if (enabled) {
    for (const weight of ["400", "600"]) {
      const font = new FontFace(
        FONT,
        `url(${FONT_URL}${weight}-normal.woff2) format("woff2")`,
        { weight, display: "swap" }
      );
      document.fonts.add(font);
      // Offline is fine: the pill falls back to the system font.
      font.load().catch(() => {});
    }
  }

  // Lucide icons (https://lucide.dev), inheriting the surrounding color.
  const lucide = (paths) =>
    '<svg xmlns="http://www.w3.org/2000/svg" width="12" height="12"' +
    ' viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"' +
    ` stroke-linecap="round" stroke-linejoin="round">${paths}</svg>`;

  const X_ICON = lucide('<path d="M18 6 6 18"/><path d="m6 6 12 12"/>');
  const SPINNER_ICON = lucide('<path d="M21 12a9 9 0 1 1-6.219-8.56"/>');

  // Inline on the host, so only an `!important` page rule targeting the host
  // element can override it; `all:initial` severs inheritance from the page.
  const HOST_STYLE = `
    all: initial;
    position: fixed;
    bottom: 16px;
    left: 16px;
    z-index: 2147483647;
    display: flex;
    align-items: center;
    gap: 7px;
    padding: 7px 8px 7px 14px;
    background: #0a0a0a;
    color: #fff;
    border: 1px solid #000;
    border-radius: 8px;
    font: 12px/1 "${FONT}", ui-sans-serif, system-ui, sans-serif;
    -webkit-font-smoothing: antialiased;
    user-select: none;
  `;

  const SHADOW_HTML = `
    <style>
      .brand {
        color: ${MUTED};
      }
      b {
        font-weight: 600;
      }

      /* all:unset strips the UA button styles. */
      .dismiss {
        all: unset;
        display: flex;
        align-items: center;
        justify-content: center;
        width: 18px;
        height: 18px;
        border-radius: 4px;
        cursor: pointer;
        color: ${MUTED};
        transition: color 0.15s ease;
      }
      .dismiss:hover {
        color: #fff;
      }

      .busy {
        color: #a5f3fc;
      }
      .spinner {
        display: flex;
        color: #a5f3fc;
        animation: spin 1s linear infinite;
      }
      @keyframes spin {
        to { transform: rotate(360deg); }
      }

      /* Error labels catch the eye with a soft highlight sweeping across
         the text once every three seconds. */
      .error {
        background-image:
          linear-gradient(100deg, ${ERROR} 30%, #fecaca 50%, ${ERROR} 70%);
        background-size: 300% 100%;
        -webkit-background-clip: text;
        background-clip: text;
        color: transparent;
        animation: shimmer 3s ease-in-out infinite;
      }
      @keyframes shimmer {
        0% { background-position: 100% 0; }
        25% { background-position: 0 0; }
        100% { background-position: 0 0; }
      }
    </style>
    <span class="brand">topcoat</span>
    <b></b>
    <span class="spinner">${SPINNER_ICON}</span>
    <button class="dismiss" aria-label="Dismiss">${X_ICON}</button>
  `;

  let pill = null;
  let statusEl = null;
  let spinnerEl = null;

  function createPill() {
    pill = document.createElement("topcoat-dev-status");
    pill.style.cssText = HOST_STYLE;

    const shadow = pill.attachShadow({ mode: "open" });
    shadow.innerHTML = SHADOW_HTML;

    statusEl = shadow.querySelector("b");
    spinnerEl = shadow.querySelector(".spinner");
    shadow.querySelector("button").onclick = hideStatus;
  }

  function showStatus(label, isError) {
    if (!enabled) return;
    if (!document.body) {
      // The script can run from <head> before the body exists; the pill can
      // only be attached once the document is ready.
      document.addEventListener(
        "DOMContentLoaded",
        () => showStatus(label, isError),
        { once: true }
      );
      return;
    }
    if (!pill) createPill();

    statusEl.textContent = label;
    statusEl.className = isError ? "error" : "busy";
    // An empty display falls back to the class's `flex`.
    spinnerEl.style.display = isError ? "none" : "";

    // Re-attach even if previously dismissed: each event is news.
    if (!pill.isConnected) document.body.append(pill);
  }

  function hideStatus() {
    pill?.remove();
  }

  // --- Dev server connection ------------------------------------------------

  const MESSAGES = {
    "reload": () => window.location.reload(),
    "rebuilding": () => showStatus("rebuilding", false),
    "build-failed": () => showStatus("build failed", true),
    "app-exited": () => showStatus("app exited", true),
  };

  function connect() {
    const ws = new WebSocket(wsUrl);
    ws.onmessage = (e) => MESSAGES[e.data]?.();
    ws.onclose = reconnect;
  }

  // Probe until the dev server is back, then reload to pick up whatever it
  // now serves.
  function reconnect() {
    setTimeout(() => {
      const probe = new WebSocket(wsUrl);
      probe.onopen = () => {
        probe.close();
        window.location.reload();
      };
      probe.onerror = () => setTimeout(connect, 1000);
    }, 500);
  }

  connect();
})();
