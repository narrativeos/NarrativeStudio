import { describe, expect, it, vi } from "vitest";
import { detectBridge } from "./bridge";

describe("detectBridge", () => {
  it("finds the bridge the Tauri runtime injects", () => {
    expect(detectBridge({ __TAURI_INTERNALS__: { invoke: vi.fn() } })).toBe("tauri");
  });

  it("calls a bare browser window a window with no backend", () => {
    // The whole point: in a browser the object is absent, and every command
    // against it throws a TypeError that says nothing about why.
    expect(detectBridge({})).toBe("none");
    expect(detectBridge(undefined)).toBe("none");
    expect(detectBridge(null)).toBe("none");
  });

  it("does not mistake a half-injected global for a working one", () => {
    expect(detectBridge({ __TAURI_INTERNALS__: {} })).toBe("none");
    expect(detectBridge({ __TAURI_INTERNALS__: { invoke: "nope" } })).toBe("none");
  });
});
