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

  var TICKS = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
  var MUTED = "#a1a1aa";
  var ERROR = "#fca5a5";

  var pill = null;
  var statusEl = null;
  var spinnerEl = null;
  var spinnerTimer = null;

  function createPill() {
    pill = document.createElement("div");
    pill.style.cssText =
      "position:fixed;bottom:16px;left:16px;z-index:2147483647;" +
      "display:flex;align-items:center;gap:7px;padding:7px 8px 7px 14px;" +
      "background:#0a0a0a;color:#fff;border:1px solid #3f3f46;" +
      "border-radius:8px;" +
      "font:12px/1 ui-sans-serif,system-ui,sans-serif;" +
      "-webkit-font-smoothing:antialiased;user-select:none";

    var brand = document.createElement("span");
    brand.textContent = "topcoat";
    brand.style.cssText = "color:" + MUTED;
    pill.appendChild(brand);

    statusEl = document.createElement("b");
    statusEl.style.cssText = "font-weight:600";
    pill.appendChild(statusEl);

    spinnerEl = document.createElement("span");
    spinnerEl.style.cssText =
      "display:inline-block;width:1em;text-align:center;color:#67e8f9";
    pill.appendChild(spinnerEl);

    var close = document.createElement("button");
    close.textContent = "✕";
    close.setAttribute("aria-label", "Dismiss");
    close.style.cssText =
      "all:unset;display:flex;align-items:center;justify-content:center;" +
      "width:18px;height:18px;border-radius:4px;cursor:pointer;" +
      "color:" + MUTED + ";font-size:10px";
    close.onmouseenter = function () {
      close.style.color = "#fff";
    };
    close.onmouseleave = function () {
      close.style.color = MUTED;
    };
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
    statusEl.style.color = isError ? ERROR : "#fff";
    spinnerEl.style.display = isError ? "none" : "inline-block";
    if (isError) stopSpinner();
    else startSpinner();

    // Re-attach even if previously dismissed: each event is news.
    if (!pill.parentNode) document.body.appendChild(pill);
  }

  function hideStatus() {
    stopSpinner();
    if (pill && pill.parentNode) pill.parentNode.removeChild(pill);
  }

  function startSpinner() {
    if (spinnerTimer) return;
    var tick = 0;
    spinnerEl.textContent = TICKS[0];
    spinnerTimer = setInterval(function () {
      tick = (tick + 1) % TICKS.length;
      spinnerEl.textContent = TICKS[tick];
    }, 80);
  }

  function stopSpinner() {
    if (spinnerTimer) {
      clearInterval(spinnerTimer);
      spinnerTimer = null;
    }
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
})();
