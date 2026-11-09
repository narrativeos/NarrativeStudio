//! What the user picked for light or dark, and what that paints.
//!
//! The choice is theirs (`system` follows the OS), the resolved theme lands on
//! `<html data-theme>`, and the stylesheet in `index.css` does the rest. Only the
//! pure parts live here so the rules can be tested without a document.

export type Theme = "light" | "dark";
export type ThemeChoice = Theme | "system";

export const THEME_STORAGE_KEY = "narrativestudio.theme";

const CHOICES: readonly ThemeChoice[] = ["light", "dark", "system"];

/** The theme to paint: the pick when it is explicit, the OS otherwise. */
export function resolveTheme(choice: ThemeChoice, prefersDark: boolean): Theme {
  if (choice === "light" || choice === "dark") return choice;
  return prefersDark ? "dark" : "light";
}

/** A stored choice, or `system` when nothing usable was stored. */
export function readChoice(
  storage: Pick<Storage, "getItem">,
): ThemeChoice {
  const stored = storage.getItem(THEME_STORAGE_KEY);
  return CHOICES.includes(stored as ThemeChoice) ? (stored as ThemeChoice) : "system";
}

export function writeChoice(
  storage: Pick<Storage, "setItem">,
  choice: ThemeChoice,
): void {
  storage.setItem(THEME_STORAGE_KEY, choice);
}

/** Paint the choice onto the document; returns the theme actually applied. */
export function applyTheme(choice: ThemeChoice, prefersDark: boolean): Theme {
  const theme = resolveTheme(choice, prefersDark);
  document.documentElement.dataset.theme = theme;
  return theme;
}