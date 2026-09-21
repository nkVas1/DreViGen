/**
 * Where the front end finds out what it is running inside.
 *
 * The same React tree renders in a Tauri window and in a browser tab, and a handful of things
 * genuinely differ between them: whether the window has its own title bar, whether there are
 * safe-area insets to respect, and — later — which transport the bridge to the Rust core uses.
 *
 * Detection happens once, here, rather than being sprinkled through components. A component
 * that asks "am I in Tauri?" is a component that will be wrong on the fifth platform.
 */

/** Which shell the front end is running in. */
export type Shell = 'tauri' | 'browser';

/** What the surrounding environment provides. */
export interface Platform {
  /** The shell. */
  readonly shell: Shell;
  /** Coarse pointer as the primary input — a phone or tablet rather than a mouse. */
  readonly touch: boolean;
  /** The viewer has asked their system for less motion. */
  readonly reducedMotion: boolean;
  /** The page is installed to a home screen or running as a standalone window. */
  readonly installed: boolean;
}

/**
 * Tauri injects this before the front end loads. Reading it is the only reliable way to tell
 * the shells apart: a user agent can be anything, and the Tauri webview on Android is Chrome.
 */
declare global {
  interface Window {
    readonly __TAURI_INTERNALS__?: unknown;
  }
}

function query(feature: string): boolean {
  return typeof window !== 'undefined' && window.matchMedia?.(feature).matches === true;
}

/** Reads the environment. Cheap; call it once and pass the result down. */
export function detectPlatform(): Platform {
  const inTauri = typeof window !== 'undefined' && window.__TAURI_INTERNALS__ !== undefined;
  return {
    shell: inTauri ? 'tauri' : 'browser',
    touch: query('(pointer: coarse)'),
    reducedMotion: query('(prefers-reduced-motion: reduce)'),
    installed: inTauri || query('(display-mode: standalone)'),
  };
}

// ── theme ──────────────────────────────────────────────────────────────────

/**
 * The themes a viewer can choose.
 *
 * `system` is a real choice and the default, not the absence of one: it stamps nothing on the
 * document, which is what lets `prefers-color-scheme` decide. The other three stamp
 * `data-theme`, which the generated stylesheet is written to respect in both directions.
 */
export type ThemeChoice = 'system' | 'light' | 'dark' | 'contrast';

const THEME_KEY = 'drevigen.theme';

/** Applies a theme choice to the document. */
export function applyTheme(choice: ThemeChoice): void {
  const root = document.documentElement;
  if (choice === 'system') {
    root.removeAttribute('data-theme');
  } else {
    root.setAttribute('data-theme', choice);
  }
  try {
    localStorage.setItem(THEME_KEY, choice);
  } catch {
    // A private window, or blocked site data. The choice still applies for this session; it
    // simply will not be remembered, which is a smaller failure than not applying it.
  }
  tellShell(choice);
}

/**
 * Tells the native shell which theme the page settled on.
 *
 * The webview colours everything inside itself, and nothing outside it. On Windows the title
 * bar would keep the system accent colour — a bright bar above a vellum page. So the shell is
 * told, and it repaints the frame to match.
 *
 * Deliberately a dynamic import: in a browser the module is never fetched, and in an older
 * shell that does not know the command the rejection is swallowed. A window frame is not worth
 * a broken theme switch.
 */
function tellShell(choice: ThemeChoice): void {
  if (typeof window === 'undefined' || window.__TAURI_INTERNALS__ === undefined) return;

  void import('@tauri-apps/api/core')
    .then(({ invoke }) => invoke('set_window_theme', { theme: choice }))
    .catch(() => {
      // The frame keeps the system colour, which is where it started.
    });
}

/** Reads the remembered theme, defaulting to following the system. */
export function readTheme(): ThemeChoice {
  try {
    const stored = localStorage.getItem(THEME_KEY);
    if (stored === 'light' || stored === 'dark' || stored === 'contrast' || stored === 'system') {
      return stored;
    }
  } catch {
    // See applyTheme.
  }
  return 'system';
}

// ── text scale ─────────────────────────────────────────────────────────────

/**
 * Document text scale.
 *
 * Required by [elder-ux-and-accessibility.md] and not optional: 100 / 125 / 150 / 200 %, and it
 * must **reflow** rather than zoom a bitmap. Implemented by scaling the root font size, so
 * every length expressed in `rem` follows and the layout genuinely re-wraps.
 */
export const TEXT_SCALES = [100, 125, 150, 200] as const;

/** A permitted text scale, as a percentage. */
export type TextScale = (typeof TEXT_SCALES)[number];

const SCALE_KEY = 'drevigen.textScale';

/** The root font size at 100 %, in pixels. The body floor from the art direction is 17 px. */
const BASE_PX = 17;

/** Applies a text scale to the document. */
export function applyTextScale(scale: TextScale): void {
  document.documentElement.style.fontSize = `${(BASE_PX * scale) / 100}px`;
  try {
    localStorage.setItem(SCALE_KEY, String(scale));
  } catch {
    // See applyTheme.
  }
}

/** Reads the remembered text scale, defaulting to 100 %. */
export function readTextScale(): TextScale {
  try {
    const stored = Number(localStorage.getItem(SCALE_KEY));
    const match = TEXT_SCALES.find((s) => s === stored);
    if (match) return match;
  } catch {
    // See applyTheme.
  }
  return 100;
}
