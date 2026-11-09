import { describe, expect, it } from "vitest";

import {
  checklistStorageKey,
  readTicks,
  tickProgress,
  toggleTick,
  writeTicks,
  CHECKLIST_STORAGE_PREFIX,
  type Ticks,
} from "./checklist";

const store = (entries: [string, string][] = []) => {
  const data = new Map<string, string>(entries);
  return {
    getItem: (k: string) => data.get(k) ?? null,
    setItem: (k: string, v: string) => void data.set(k, v),
    data,
  };
};

describe("checklist ticks", () => {
  it("keeps one project's ticks apart from another's", () => {
    const storage = store();
    writeTicks(storage, "p-1", { "plot-unresolved": true });
    expect(storage.data.has(`${CHECKLIST_STORAGE_PREFIX}.p-1`)).toBe(true);
    expect(readTicks(storage, "p-2")).toEqual({});
    expect(checklistStorageKey("p-1")).toBe(`${CHECKLIST_STORAGE_PREFIX}.p-1`);
  });

  it("reads unreadable storage as nothing ticked", () => {
    expect(readTicks(store([[`${CHECKLIST_STORAGE_PREFIX}.p`, "not json"]]), "p")).toEqual({});
    expect(readTicks(store([[`${CHECKLIST_STORAGE_PREFIX}.p`, "[1,2]"]]), "p")).toEqual({});
    expect(readTicks(store(), "p")).toEqual({});
  });

  it("keeps only what is actually ticked", () => {
    const storage = store([[`${CHECKLIST_STORAGE_PREFIX}.p`, JSON.stringify({ a: true, b: false, c: 1 })]]);
    expect(readTicks(storage, "p")).toEqual({ a: true });
  });

  it("flips one item without disturbing the others", () => {
    const before: Ticks = { a: true, b: true };
    expect(toggleTick(before, "c")).toEqual({ a: true, b: true, c: true });
    expect(toggleTick(before, "a")).toEqual({ b: true });
    // The input is never mutated: React state must change identity to re-render.
    expect(before).toEqual({ a: true, b: true });
  });

  it("counts only the items the analysis still produces", () => {
    const items = [{ id: "a" }, { id: "b" }];
    expect(tickProgress(items, { a: true })).toEqual({ done: 1, total: 2 });
    expect(tickProgress(items, { a: true, b: true, gone: true })).toEqual({ done: 2, total: 2 });
    expect(tickProgress([], { a: true })).toEqual({ done: 0, total: 0 });
  });
});
