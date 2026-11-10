import { describe, expect, it } from "vitest";
import { isTypingTarget, matchShortcut, type KeyEvent } from "./shortcuts";

const key = (over: Partial<KeyEvent> = {}): KeyEvent => ({
  key: "x",
  metaKey: false,
  ctrlKey: false,
  shiftKey: false,
  altKey: false,
  ...over,
});

describe("shortcut matching", () => {
  it("takes the design's three bindings on the primary modifier", () => {
    expect(matchShortcut(key({ key: "o", metaKey: true }), true)).toBe("import");
    expect(matchShortcut(key({ key: "r", metaKey: true }), true)).toBe("reanalyze");
    expect(matchShortcut(key({ key: "e", metaKey: true }), true)).toBe("export");
    expect(matchShortcut(key({ key: "o", ctrlKey: true }), false)).toBe("import");
  });

  it("gives the second import path its own chord", () => {
    expect(matchShortcut(key({ key: "o", metaKey: true, shiftKey: true }), true)).toBe(
      "import-folder",
    );
  });

  it("leaves a bare key to whatever has focus", () => {
    // The whole point: typing an "e" into a field must not export a report.
    for (const k of ["o", "r", "e"]) {
      expect(matchShortcut(key({ key: k }), true)).toBeNull();
      expect(matchShortcut(key({ key: k }), false)).toBeNull();
    }
  });

  it("leaves a chord that mixes both modifiers to the system", () => {
    // ⌃⌘R and friends belong to the WebView and to the reader's own bindings.
    expect(matchShortcut(key({ key: "r", metaKey: true, ctrlKey: true }), true)).toBeNull();
    expect(matchShortcut(key({ key: "o", metaKey: true, altKey: true }), true)).toBeNull();
    expect(matchShortcut(key({ key: "r", ctrlKey: true }), true)).toBeNull();
  });

  it("does not invent a binding for a key it was never given", () => {
    expect(matchShortcut(key({ key: "n", metaKey: true }), true)).toBeNull();
    expect(matchShortcut(key({ key: "r", metaKey: true, shiftKey: true }), true)).toBeNull();
  });

  it("is case-insensitive, because caps lock is a thing", () => {
    expect(matchShortcut(key({ key: "O", metaKey: true }), true)).toBe("import");
  });
});

describe("typing target", () => {
  it("leaves the keys to fields and to anything you can type into", () => {
    for (const tagName of ["INPUT", "TEXTAREA", "SELECT", "input"]) {
      expect(isTypingTarget({ tagName } as unknown as EventTarget)).toBe(true);
    }
    expect(
      isTypingTarget({ tagName: "DIV", isContentEditable: true } as unknown as EventTarget),
    ).toBe(true);
  });

  it("takes the keys back from ordinary elements", () => {
    expect(isTypingTarget({ tagName: "BUTTON" } as unknown as EventTarget)).toBe(false);
    expect(isTypingTarget({ tagName: "DIV" } as unknown as EventTarget)).toBe(false);
    // A key press that belongs to no element at all — the window itself.
    expect(isTypingTarget(null)).toBe(false);
    expect(isTypingTarget({} as unknown as EventTarget)).toBe(false);
  });
});
