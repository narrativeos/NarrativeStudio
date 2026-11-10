import { useMemo, useState } from "react";
import { Panel, SectionTitle, SECTION_ANCHOR } from "../Cards";
import { checklistBadgeStyle, checklistGroupLabel, formatInt } from "../../lib/format";
import { readTicks, tickProgress, toggleTick, writeTicks } from "../../lib/checklist";
import type { ChecklistItem, ReviewChecklist } from "../../types";

interface ChecklistSectionProps {
  checklist: ReviewChecklist;
  /** Ticks are per project: two books are worked through separately. */
  projectId: string;
}

/**
 * 最终审查清单 section: the numbers that still need a decision, in one list.
 *
 * Nothing is measured here. Each item repeats a count another section already
 * showed, phrased as a question, so the last read has a starting point. Ticking an
 * item records reading progress in the browser store and changes no analysis result,
 * which is why the row never turns a fault colour and why the list carries no score.
 */
function ChecklistSection({ checklist, projectId }: ChecklistSectionProps) {
  // Ticks are read from the browser store, and the store is per project: switching
  // projects swaps the list rather than writing over the previous one. Holding the
  // project the ticks belong to alongside them lets the swap happen during render,
  // so no effect has to setState after the fact.
  const [state, setState] = useState(() => ({
    projectId,
    ticks: readTicks(localStorage, projectId),
  }));
  if (state.projectId !== projectId) {
    setState({ projectId, ticks: readTicks(localStorage, projectId) });
  }
  const ticks = state.ticks;

  const { done, total } = useMemo(() => tickProgress(checklist.items, ticks), [checklist, ticks]);

  function toggle(id: string) {
    setState((prev) => {
      const next = toggleTick(prev.ticks, id);
      writeTicks(localStorage, prev.projectId, next);
      return { projectId: prev.projectId, ticks: next };
    });
  }

  return (
    <section id="checklist" className={SECTION_ANCHOR}>
      <SectionTitle>最终审查清单</SectionTitle>

      <div className="mb-3 flex flex-wrap items-center gap-2">
        <span className={`px-2 py-0.5 rounded border text-xs ${checklistBadgeStyle()}`}>
          不给评分
        </span>
        <span className="text-xs text-text-muted">
          共 {formatInt(total)} 项，已确认 {formatInt(done)} 项。每一项后面是触发它的数字；勾选只是本机的阅读进度，不改变任何分析结果。
        </span>
      </div>

      {total === 0 ? (
        <Panel title="没有需要逐条确认的项">
          <p className="text-xs text-text-muted">
            以上各维度的数字都没有触发这里的任何一条规则。空清单只说明被计到的东西不需要决定，不说明稿子已经好。
          </p>
        </Panel>
      ) : (
        <div className="space-y-4">
          {checklist.groups.map(([group, count]) => {
            const items = checklist.items.filter((i) => i.group === group);
            return (
              <Panel
                key={group}
                title={`${checklistGroupLabel(group)}（${formatInt(count)} 项）`}
              >
                {items.length === 0 ? (
                  <p className="text-xs text-text-muted">这一组没有需要确认的项。</p>
                ) : (
                  <ul className="space-y-3">
                    {items.map((item) => (
                      <ChecklistRow
                        key={item.id}
                        item={item}
                        checked={ticks[item.id] === true}
                        onToggle={toggle}
                      />
                    ))}
                  </ul>
                )}
              </Panel>
            );
          })}
        </div>
      )}
    </section>
  );
}

function ChecklistRow({
  item,
  checked,
  onToggle,
}: {
  item: ChecklistItem;
  checked: boolean;
  onToggle: (id: string) => void;
}) {
  return (
    <li>
      {/* The whole row is the control. A bare checkbox beside a paragraph that is
          not its label means the only way to tick an item is to hit that box
          exactly, and 14px is well under what a trackpad lands on reliably — in a
          list meant to be worked through one item at a time. Wrapping the row in
          a label also gives the checkbox its accessible name for free. */}
      <label className="flex cursor-pointer items-start gap-2.5">
        <span className="relative mt-0.5 inline-flex h-5 w-5 shrink-0 items-center justify-center">
          <input
            type="checkbox"
            checked={checked}
            onChange={() => onToggle(item.id)}
            aria-label={item.label}
            className="peer h-5 w-5 cursor-pointer appearance-none rounded border border-line-strong bg-surface transition-colors checked:border-accent checked:bg-accent"
          />
          <svg
            aria-hidden
            viewBox="0 0 12 12"
            className="pointer-events-none absolute h-3 w-3 text-surface opacity-0 peer-checked:opacity-100"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
          >
            <path d="M2 6.5 4.5 9 10 3" />
          </svg>
        </span>
        <span className="min-w-0">
          {/* Confirmed items fade rather than get struck through: a tick means the
              question was answered, not that the line was struck out. */}
          <span className={`block text-xs ${checked ? "text-text-muted" : ""}`}>
            {item.label}
          </span>
          <span className="block text-xs text-text-muted mt-0.5">{item.detail}</span>
        </span>
      </label>
    </li>
  );
}

export default ChecklistSection;
