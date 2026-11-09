import { useMemo, useState } from "react";
import { Panel, SectionTitle } from "../Cards";
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
    <section>
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
    <li className="flex items-start gap-2">
      <input
        type="checkbox"
        checked={checked}
        onChange={() => onToggle(item.id)}
        className="mt-0.5 h-3.5 w-3.5 shrink-0"
        aria-label={item.label}
      />
      <div className="min-w-0">
        {/* Confirmed items fade rather than get struck through: a tick means the
            question was answered, not that the line was struck out. */}
        <p className={`text-xs ${checked ? "text-text-muted" : ""}`}>{item.label}</p>
        <p className="text-xs text-text-muted mt-0.5">{item.detail}</p>
      </div>
    </li>
  );
}

export default ChecklistSection;
