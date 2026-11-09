//! What a reader has already confirmed on the final review checklist.
//!
//! The list itself is computed by the analysis (`studio_analysis::checklist`); this
//! file only holds the ticks. They are reading progress, not a measurement: nothing
//! in the analysis changes because an item is ticked, so they live in the browser
//! store beside the theme choice rather than in the database, where every other
//! project fact lives. A stored tick is keyed by the item's stable `id`, so it
//! survives a re-analysis and only goes stale if a rule itself is renamed.

export const CHECKLIST_STORAGE_PREFIX = "narrativestudio.checklist";

export type Ticks = Record<string, boolean>;

/** Where one project's ticks live. Per project: two books are read separately. */
export function checklistStorageKey(projectId: string): string {
  return `${CHECKLIST_STORAGE_PREFIX}.${projectId}`;
}

/** The ticks stored for a project. Unreadable storage reads as nothing ticked. */
export function readTicks(storage: Pick<Storage, "getItem">, projectId: string): Ticks {
  const raw = storage.getItem(checklistStorageKey(projectId));
  if (!raw) return {};
  let parsed: unknown;
  try {
    parsed = JSON.parse(raw);
  } catch {
    return {};
  }
  if (typeof parsed !== "object" || parsed === null) return {};
  const out: Ticks = {};
  for (const [id, value] of Object.entries(parsed as Record<string, unknown>)) {
    // Only true is stored: an unchecked item is simply absent, so the payload stays
    // small as a long list is worked through.
    if (value === true) out[id] = true;
  }
  return out;
}

export function writeTicks(
  storage: Pick<Storage, "setItem">,
  projectId: string,
  ticks: Ticks,
): void {
  storage.setItem(checklistStorageKey(projectId), JSON.stringify(ticks));
}

/** One item flipped; anything already false is dropped rather than stored. */
export function toggleTick(ticks: Ticks, id: string): Ticks {
  const next: Ticks = {};
  for (const [key, value] of Object.entries(ticks)) {
    if (value === true && key !== id) next[key] = true;
  }
  if (ticks[id] !== true) next[id] = true;
  return next;
}

/** How far the list has been worked through. Items no longer produced are not counted. */
export function tickProgress(
  items: readonly { id: string }[],
  ticks: Ticks,
): { done: number; total: number } {
  const done = items.filter((i) => ticks[i.id] === true).length;
  return { done, total: items.length };
}
