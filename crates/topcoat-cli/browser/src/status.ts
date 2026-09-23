/**
 * A floating pill that shows the build status in the corner of the page.
 *
 * It lives in a shadow tree, so page styles do not affect it. When `enabled`
 * is false, it never shows anything.
 */
export class StatusIndicator {
	private pill: {
		host: HTMLElement;
		label: HTMLElement;
		spinner: HTMLElement;
	} | null = null;
	private current: { label: string; isError: boolean } | null = null;

	constructor(private readonly enabled: boolean) {
		if (!enabled) return;
		document.addEventListener("DOMContentLoaded", () => this.render(), {
			once: true,
		});
		for (const weight of ["400", "600"]) {
			const font = new FontFace(
				FONT,
				`url(${FONT_URL}${weight}-normal.woff2) format("woff2")`,
				{ weight, display: "swap" },
			);
			document.fonts.add(font);
			// Offline is fine: the pill falls back to the system font.
			font.load().catch(() => {});
		}
	}

	/** Shows `label` in the pill, styled as an error when `isError` is set. */
	show(label: string, isError = false): void {
		if (!this.enabled) return;
		this.current = { label, isError };
		this.render();
	}

	/** Removes the pill from the page until the next `show`. */
	hide(): void {
		this.current = null;
		this.pill?.host.remove();
	}

	private render(): void {
		if (!this.current || !document.body) return;
		this.pill ??= this.createPill();
		const { host, label, spinner } = this.pill;
		label.textContent = this.current.label;
		label.className = this.current.isError ? "error" : "busy";
		spinner.style.display = this.current.isError ? "none" : "";
		// Re-attach even if previously dismissed: each event is news.
		if (!host.isConnected) document.body.append(host);
	}

	private createPill() {
		const host = document.createElement("topcoat-dev-status");
		host.style.cssText = HOST_STYLE;
		const shadow = host.attachShadow({ mode: "open" });
		shadow.innerHTML = SHADOW_HTML;
		const label = shadow.querySelector("b") as HTMLElement;
		const spinner = shadow.querySelector(".spinner") as HTMLElement;
		const dismiss = shadow.querySelector("button") as HTMLButtonElement;
		dismiss.addEventListener("click", () => this.hide());
		return { host, label, spinner };
	}
}

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
// Lucide icons (https://lucide.dev), inheriting the surrounding color.
const lucide = (paths: string) =>
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
