(function () {
  var script = document.currentScript;
  var url = new URL(script.src);
  url.protocol = url.protocol === "https:" ? "wss:" : "ws:";
  url.pathname = "/ws";
  url.search = "";
  url.hash = "";
  var wsUrl = url.toString();

  // --- Status indicator ----------------------------------------------------
  //
  // A small floating pill mirroring the CLI status: a spinner while the dev
  // server is rebuilding, and an error label when a build fails or the app
  // exits. Disabled by rendering the script tag with
  // `data-status-indicator="false"`.

  var enabled = script.getAttribute("data-status-indicator") !== "false";

  var MUTED = "#a1a1aa";
  var ERROR = "#fca5a5";

  // Lucide "x" (https://lucide.dev), inheriting the button's color.
  var X_ICON =
    '<svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24"' +
    ' fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round"' +
    ' stroke-linejoin="round"><path d="M18 6 6 18"/><path d="m6 6 12 12"/></svg>';

  // Lucide "loader-circle", spun by the stylesheet.
  var SPINNER_ICON =
    '<svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24"' +
    ' fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round"' +
    ' stroke-linejoin="round"><path d="M21 12a9 9 0 1 1-6.219-8.56"/></svg>';

  var FONT_URL =
    "https://cdn.jsdelivr.net/fontsource/fonts/lexend-deca@latest/latin-";
  // The dismiss button is styled through a stylesheet rather than inline so
  // it can have a :hover style; `all:unset` shields it from site styles.
  var STYLE_CSS =
    '@font-face{font-family:"Lexend Deca";font-style:normal;font-weight:400;' +
    "font-display:swap;src:url(" + FONT_URL + '400-normal.woff2) format("woff2")}' +
    '@font-face{font-family:"Lexend Deca";font-style:normal;font-weight:600;' +
    "font-display:swap;src:url(" + FONT_URL + '600-normal.woff2) format("woff2")}' +
    ".topcoat-dev-dismiss{all:unset;display:flex;align-items:center;" +
    "justify-content:center;width:18px;height:18px;border-radius:4px;" +
    "cursor:pointer;color:" + MUTED + ";transition:color .15s ease}" +
    ".topcoat-dev-dismiss:hover{color:#fff}" +
    ".topcoat-dev-busy{color:#a5f3fc}" +
    ".topcoat-dev-spinner{display:flex;color:#a5f3fc;" +
    "animation:topcoat-dev-spin 1s linear infinite}" +
    "@keyframes topcoat-dev-spin{to{transform:rotate(360deg)}}" +
    // Error labels catch the eye with a soft highlight sweeping across the
    // text once every three seconds.
    ".topcoat-dev-error{background-image:linear-gradient(100deg," +
    ERROR + " 30%,#fecaca 50%," + ERROR + " 70%);" +
    "background-size:300% 100%;-webkit-background-clip:text;" +
    "background-clip:text;color:transparent;" +
    "animation:topcoat-dev-shimmer 3s ease-in-out infinite}" +
    "@keyframes topcoat-dev-shimmer{0%{background-position:100% 0}" +
    "25%{background-position:0 0}100%{background-position:0 0}}";

  var pill = null;
  var statusEl = null;
  var spinnerEl = null;

  function createPill() {
    var style = document.createElement("style");
    style.textContent = STYLE_CSS;
    document.head.appendChild(style);

    pill = document.createElement("div");
    pill.style.cssText =
      "position:fixed;bottom:16px;left:16px;z-index:2147483647;" +
      "display:flex;align-items:center;gap:7px;padding:7px 8px 7px 14px;" +
      "background:#0a0a0a;color:#fff;border:1px solid #000;" +
      "border-radius:8px;" +
      "font:12px/1 'Lexend Deca',ui-sans-serif,system-ui,sans-serif;" +
      "-webkit-font-smoothing:antialiased;user-select:none";

    var brand = document.createElement("span");
    brand.textContent = "topcoat";
    brand.style.cssText = "color:" + MUTED;
    pill.appendChild(brand);

    statusEl = document.createElement("b");
    statusEl.style.cssText = "font-weight:600";
    pill.appendChild(statusEl);

    spinnerEl = document.createElement("span");
    spinnerEl.className = "topcoat-dev-spinner";
    spinnerEl.innerHTML = SPINNER_ICON;
    pill.appendChild(spinnerEl);

    var close = document.createElement("button");
    close.className = "topcoat-dev-dismiss";
    close.setAttribute("aria-label", "Dismiss");
    close.innerHTML = X_ICON;
    close.onclick = hideStatus;
    pill.appendChild(close);
  }

  function showStatus(label, isError) {
    if (!enabled) return;
    if (!document.body) {
      // The script can run from <head> before the body exists; the pill can
      // only be attached once the document is ready.
      document.addEventListener(
        "DOMContentLoaded",
        function () {
          showStatus(label, isError);
        },
        { once: true }
      );
      return;
    }
    if (!pill) createPill();

    statusEl.textContent = label;
    statusEl.className = isError ? "topcoat-dev-error" : "topcoat-dev-busy";
    // An empty display falls back to the class's `flex`.
    spinnerEl.style.display = isError ? "none" : "";

    // Re-attach even if previously dismissed: each event is news.
    if (!pill.parentNode) document.body.appendChild(pill);
  }

  function hideStatus() {
    if (pill && pill.parentNode) pill.parentNode.removeChild(pill);
  }

  // --- Dev server connection ------------------------------------------------

  function connect() {
    var ws = new WebSocket(wsUrl);
    ws.onmessage = function (e) {
      if (e.data === "reload") window.location.reload();
      else if (e.data === "rebuilding") showStatus("rebuilding", false);
      else if (e.data === "build-failed") showStatus("build failed", true);
      else if (e.data === "app-exited") showStatus("app exited", true);
    };
    ws.onclose = function () {
      setTimeout(function () {
        var retry = new WebSocket(wsUrl);
        retry.onopen = function () {
          retry.close();
          window.location.reload();
        };
        retry.onerror = function () {
          setTimeout(connect, 1000);
        };
      }, 500);
    };
  }
  connect();

  // TEMP: always show the pill to iterate on its design. Remove this.
  showStatus("rebuilding", false);
})();
