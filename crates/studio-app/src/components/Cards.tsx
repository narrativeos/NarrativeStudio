import type { ReactNode } from "react";

/**
 * Shared layout primitives for the analysis view.
 *
 * They live here rather than in `Analysis.tsx` so each analysis dimension can be
 * its own component without duplicating — or racing to restyle — the cards.
 */

/** Heading above one dimension's group of cards and panels. */
export function SectionTitle({ children }: { children: ReactNode }) {
  return (
    <h3 className="text-sm font-semibold text-text-muted uppercase tracking-wide mb-3">
      {children}
    </h3>
  );
}

/** A single headline number. */
export function StatCard({ label, value }: { label: string; value: string }) {
  return (
    <div className="p-3 bg-surface-alt rounded-lg border border-gray-700">
      <p className="text-xs text-text-muted">{label}</p>
      <p className="text-xl font-semibold mt-1 tabular-nums">{value}</p>
    </div>
  );
}

/** A titled box for one list, table or chart. */
export function Panel({ title, children }: { title: string; children: ReactNode }) {
  return (
    <div className="p-4 bg-surface-alt rounded-lg border border-gray-700">
      <h4 className="text-sm font-medium text-text-muted mb-3">{title}</h4>
      {children}
    </div>
  );
}