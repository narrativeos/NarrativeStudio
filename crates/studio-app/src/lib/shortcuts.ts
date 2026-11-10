import { useEffect, useRef } from "react";

/** The commands the app binds to the keyboard. */
export type Shortcut = "import" | "import-folder" | "reanalyze" | "export";

/** The subset of a keyboard event this decision needs. */
export interface KeyEvent {
  key: string;
  metaKey: boolean;
  ctrlKey: boolean;
  shiftKey: boolean;
  altKey: boolean;
}

/**
 * Which command a key press asks for, or null when it asks for nothing.
 *
 * The design asks for ⌘O, ⌘R and ⌘E; none of them existed. Two rules keep this
 * from stealing keys the WebView or the reader already needs: the primary
 * modifier has to be down and the *other* one up, so ⌘R is ours and ⌃⌘R is not;
 * and a bare letter never matches, so typing "e" stays typing.
 *
 * ⌘⇧O is the second import path rather than a fourth binding because the two
 * entries differ in what they can measure — a file, or a TraceView directory —
 * and that difference deserves its own key instead of a dialog step.
 */
export function matchShortcut(event: KeyEvent, isMac: boolean): Shortcut | null {
  const primary = isMac ? event.metaKey : event.ctrlKey;
  const secondary = isMac ? event.ctrlKey : event.metaKey;
  if (!primary || secondary || event.altKey) return null;

  switch (event.key.toLowerCase()) {
    case "o":
      return event.shiftKey ? "import-folder" : "import";
    case "r":
      return event.shiftKey ? null : "reanalyze";
    case "e":
      return event.shiftKey ? null : "export";
    default:
      return null;
  }
}

/**
 * Whether the key press belongs to whatever the reader is typing into.
 *
 * A shortcut that fires while a field has focus destroys what was being typed,
 * and in a writing tool that is the worst possible bug.
 */
export function isTypingTarget(target: EventTarget | null): boolean {
  if (!target || typeof target !== "object") return false;
  // Duck-typed rather than `instanceof HTMLElement`, which only exists in a
  // browser: the decision is worth testing on its own, and it should not need a
  // DOM to be tested.
  const element = target as { tagName?: unknown; isContentEditable?: unknown };
  if (typeof element.tagName !== "string") return false;
  return (
    element.isContentEditable === true ||
    ["INPUT", "TEXTAREA", "SELECT"].includes(element.tagName.toUpperCase())
  );
}

/**
 * Bind the shortcuts while a component is mounted.
 *
 * Handlers are read through a ref so passing fresh closures every render does
 * not re-register the listener; a handler that is absent leaves its key alone,
 * which is what lets the project list own ⌘O and the analysis page own ⌘R
 * without either having to know the other exists.
 */
export function useShortcuts(handlers: Partial<Record<Shortcut, () => void>>) {
  const latest = useRef(handlers);
  useEffect(() => {
    latest.current = handlers;
  });

  useEffect(() => {
    const isMac = /Mac|iPhone|iPad/.test(navigator.platform);

    function onKeyDown(event: KeyboardEvent) {
      if (isTypingTarget(event.target)) return;
      const shortcut = matchShortcut(event, isMac);
      if (!shortcut) return;
      const handler = latest.current[shortcut];
      if (!handler) return;
      // ⌘R is the WebView's reload; without this the reader loses the page.
      event.preventDefault();
      handler();
    }

    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);
}
