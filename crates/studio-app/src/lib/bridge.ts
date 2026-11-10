/**
 * Whether this window can reach the Rust side at all.
 *
 * Every command in the app goes through `window.__TAURI_INTERNALS__.invoke`,
 * which the Tauri runtime injects into its own WebView and nowhere else. Open the
 * Vite server in a browser — which `pnpm dev` and the Chrome debug configuration
 * both invite — and that object is simply absent, so each `invoke` throws
 * `Cannot read properties of undefined (reading 'invoke')` and the app reports it
 * as a failed read of the project list. That reads like a corrupted library, and
 * sends someone who is only trying to style a component to look for a bug in the
 * analysis engine.
 *
 * Checked once, at startup, so the answer is a page that explains itself rather
 * than one banner per command that was tried.
 */
export type Bridge = "tauri" | "none";

/**
 * Classifies a window-like object. Takes the scope as an argument so the check
 * can be tested without installing a fake global, and so a future test of the
 * mismatched case does not have to reach into `window`.
 */
export function detectBridge(scope: unknown): Bridge {
  const internals = (
    scope as { __TAURI_INTERNALS__?: { invoke?: unknown } } | null | undefined
  )?.__TAURI_INTERNALS__;
  return typeof internals?.invoke === "function" ? "tauri" : "none";
}

export function readBridge(): Bridge {
  return detectBridge(typeof window === "undefined" ? undefined : window);
}
