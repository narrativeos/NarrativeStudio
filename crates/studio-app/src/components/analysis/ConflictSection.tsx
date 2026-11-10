import { Panel, SectionTitle, StatCard, SECTION_ANCHOR } from "../Cards";
import LineChart from "../charts/LineChart";
import {
  conflictTypeLabel,
  conflictVerdictLabel,
  conflictVerdictStyle,
  formatFixed,
  formatInt,
  formatPercent,
  formatScore,
} from "../../lib/format";
import type { ConflictStats } from "../../types";

interface ConflictSectionProps {
  conflict: ConflictStats;
}

/**
 * 冲突分析 section: what the text fights over, how hard, and where.
 *
 * The engine counts conflict vocabulary in four families, so this view shows the
 * mix (which kind of fight the book runs on) next to the curve (where it happens).
 * When no vocabulary was found the score is absent rather than zero — the screen
 * says so instead of showing a red 0 that would read as a verdict on the story.
 */
function ConflictSection({ conflict }: ConflictSectionProps) {
  const score = conflict.conflict_score;
  const curve = conflict.points.map((p) => ({
    label: p.section || "(root)",
    value: p.intensity / 100,
  }));

  return (
    <section id="conflict" className={SECTION_ANCHOR}>
      <SectionTitle>冲突分析</SectionTitle>

      <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
        <StatCard
          label="冲突评分"
          value={score === null ? "未评分" : `${formatScore(score)} / 100`}
        />
        <StatCard label="用词命中" value={formatInt(conflict.total_hits)} />
        <StatCard label="千字密度" value={formatFixed(conflict.density)} />
        <StatCard label="最长无冲突区间" value={`${conflict.longest_absent_run} 段`} />
      </div>

      <div className="mt-3 flex flex-wrap items-center gap-2">
        <span
          className={`px-2 py-0.5 rounded border text-xs ${conflictVerdictStyle(conflict.verdict)}`}
        >
          {conflictVerdictLabel(conflict.verdict)}
        </span>
        <span className="text-xs text-text-muted">
          主类型：{conflictTypeLabel(conflict.dominant_type)} · 峰值{" "}
          {conflict.peak_section || "—"}（{formatScore(conflict.peak_intensity)}，位于全文{" "}
          {formatPercent(conflict.peak_position)}）· 结尾三分之一平均{" "}
          {formatScore(conflict.tail_intensity)}
        </span>
      </div>

      {score === null && (
        <p className="mt-2 text-xs text-warn">
          未检测到冲突用词，因此不给出评分：文本可能不是叙事文本，也可能是词库尚未覆盖这种写法。
        </p>
      )}

      <div className="mt-4">
        <Panel title={`冲突曲线（${conflict.segment_count} 段 · 强度 0-100）`}>
          {curve.length === 0 ? (
            <p className="text-xs text-text-muted">未识别到冲突曲线：文档没有可切分的段落。</p>
          ) : (
            <LineChart points={curve} label="冲突强度" color="var(--color-dim-conflict)" />
          )}
          {conflict.chunked && (
            <p className="text-xs text-text-muted mt-2">
              章节不足 3 个：曲线按 {conflict.segment_count} 个等长块切分。
            </p>
          )}
        </Panel>
      </div>
      <div className="mt-4 grid md:grid-cols-2 gap-4">
        <Panel title={`冲突类型（${conflict.types.length} 类）`}>
          {conflict.types.length === 0 ? (
            <p className="text-xs text-text-muted">未检测到任何冲突用词。</p>
          ) : (
            <div className="overflow-x-auto">
              <table className="w-full text-xs">
                <thead>
                  <tr className="text-text-muted border-b border-line">
                    <th className="text-left py-1.5 font-medium">类型</th>
                    <th className="text-right py-1.5 font-medium">命中</th>
                    <th className="text-right py-1.5 font-medium">占比</th>
                    <th className="text-right py-1.5 font-medium">千字密度</th>
                    <th className="text-left py-1.5 font-medium">高频词</th>
                  </tr>
                </thead>
                <tbody>
                  {conflict.types.map((t) => (
                    <tr key={t.kind} className="border-b border-line/40">
                      <td className="py-1.5">{conflictTypeLabel(t.kind)}</td>
                      <td className="py-1.5 text-right tabular-nums">{t.hits}</td>
                      <td className="py-1.5 text-right text-text-muted tabular-nums">
                        {formatPercent(t.share)}
                      </td>
                      <td className="py-1.5 text-right text-text-muted tabular-nums">
                        {formatFixed(t.density)}
                      </td>
                      <td className="py-1.5 text-text-muted">
                        {t.top_terms
                          .slice(0, 6)
                          .map(([term, hits]) => `${term}×${hits}`)
                          .join("、")}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </Panel>

        <Panel title={`无冲突区间（${conflict.absent_runs.length}）`}>
          {conflict.absent_runs.length === 0 ? (
            <p className="text-xs text-text-muted">没有连续 3 段以上的无冲突区间。</p>
          ) : (
            <ul className="text-xs space-y-2">
              {conflict.absent_runs.slice(0, 10).map((r) => (
                <li key={`${r.from_section}→${r.to_section}`}>
                  <span className="font-medium">{r.from_section || "(root)"}</span>
                  <span className="text-text-muted"> → </span>
                  <span className="font-medium">{r.to_section || "(root)"}</span>
                  <span className="text-text-muted">
                    （{r.sections} 段 · {formatInt(r.char_count)} 字）
                  </span>
                </li>
              ))}
            </ul>
          )}
          <p className="text-xs text-text-muted mt-3 pt-2 border-t border-line/60">
            冲突评分从 100 起扣：用词密度偏低最多 −30，单一类型占比超过 80% 最多 −15，连续无冲突区间每段
            −5（最多 −20），峰值在前三分之一而结尾三分之一不足其一半 −20，单段独占超过一半命中最多
            −15；未检测到用词时不评分。
          </p>
        </Panel>
      </div>

      <div className="mt-4">
        <Panel
          title={`逐段明细（显示 ${Math.min(conflict.points.length, 20)} / ${conflict.points.length} 段）`}
        >
          {conflict.points.length === 0 ? (
            <p className="text-xs text-text-muted">暂无数据</p>
          ) : (
            <div className="overflow-x-auto">
              <table className="w-full text-xs">
                <thead>
                  <tr className="text-text-muted border-b border-line">
                    <th className="text-left py-1.5 font-medium">段落</th>
                    <th className="text-right py-1.5 font-medium">强度</th>
                    <th className="text-right py-1.5 font-medium">命中</th>
                    <th className="text-right py-1.5 font-medium">千字密度</th>
                    <th className="text-right py-1.5 font-medium">字数</th>
                    <th className="text-left py-1.5 font-medium">主类型</th>
                  </tr>
                </thead>
                <tbody>
                  {conflict.points.slice(0, 20).map((p) => (
                    <tr key={p.section} className="border-b border-line/40">
                      <td className="py-1.5 truncate max-w-[220px]" title={p.section}>
                        {p.section || "(root)"}
                      </td>
                      <td className="py-1.5 text-right tabular-nums">{formatScore(p.intensity)}</td>
                      <td className="py-1.5 text-right text-text-muted tabular-nums">{p.hits}</td>
                      <td className="py-1.5 text-right text-text-muted tabular-nums">
                        {formatFixed(p.density)}
                      </td>
                      <td className="py-1.5 text-right text-text-muted tabular-nums">
                        {formatInt(p.char_count)}
                      </td>
                      <td className="py-1.5 text-text-muted">
                        {conflictTypeLabel(p.dominant)}
                        {p.by_type.length > 1 && (
                          <span className="text-text-muted">
                            （{p.by_type.map(([kind, hits]) => `${conflictTypeLabel(kind)}${hits}`).join("、")}）
                          </span>
                        )}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </Panel>
      </div>
    </section>
  );
}

export default ConflictSection;
