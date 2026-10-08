import { Panel, SectionTitle, StatCard } from "../Cards";
import LineChart from "../charts/LineChart";
import {
  clicheCategoryLabel,
  clicheIssueLabel,
  clicheIssueStyle,
  clicheVerdictLabel,
  clicheVerdictStyle,
  formatFixed,
  formatInt,
  formatOptionalScore,
  formatPercent,
  formatScore,
} from "../../lib/format";
import type { ClicheStats } from "../../types";

interface ClicheSectionProps {
  cliche: ClicheStats;
}

/**
 * 陈词滥调 section: which stock phrases the text reaches for, and how often.
 *
 * The engine carries a phrase library and reports where the text uses it. It does
 * not decide that a phrase is bad writing — 深吸一口气 in isolation is a sentence
 * like any other — so the section leads with the rate, then hands the reader the
 * phrases, the sentences they sit in, and what shape a replacement usually takes.
 */
function ClicheSection({ cliche }: ClicheSectionProps) {
  const score = cliche.cliche_score;
  const curve = cliche.points.map((p) => ({
    label: p.section || "(root)",
    value: p.intensity / 100,
  }));

  return (
    <section>
      <SectionTitle>陈词滥调</SectionTitle>

      <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
        <StatCard label="用词评分" value={formatOptionalScore(score)} />
        <StatCard label="命中" value={formatInt(cliche.total_hits)} />
        <StatCard label="每万字" value={formatFixed(cliche.per_10k)} />
        <StatCard label="命中短语" value={formatInt(cliche.distinct_phrases)} />
      </div>

      <div className="mt-3 flex flex-wrap items-center gap-2">
        <span
          className={`px-2 py-0.5 rounded border text-xs ${clicheVerdictStyle(cliche.verdict)}`}
        >
          {clicheVerdictLabel(cliche.verdict)}
        </span>
        <span className="text-xs text-text-muted">
          最常用 {cliche.top_phrase || "—"}（{formatPercent(cliche.top_share)}）· 对白内{" "}
          {formatInt(cliche.dialogue_hits)} 处 · 反复使用的短语{" "}
          {formatInt(cliche.repeated_phrases)} 个 · 峰值 {cliche.peak_section || "—"}（
          {formatScore(cliche.peak_intensity)}）
        </span>
      </div>

      {score === null && (
        <p className="mt-2 text-xs text-amber-400">
          全文 {formatInt(cliche.char_count)} 字，不足 1000 字：按万字密度评分没有意义，因此不给出评分。下面的命中仍然列出。
        </p>
      )}

      {cliche.categories.length > 0 && (
        <div className="mt-3 flex flex-wrap gap-1.5">
          {cliche.categories.map(([category, count]) => (
            <span key={category} className="px-2 py-0.5 rounded bg-gray-500/15 text-xs">
              {clicheCategoryLabel(category)} {count}
            </span>
          ))}
        </div>
      )}

      <div className="mt-4">
        <Panel title={`高频套话（${cliche.phrases.length} 个）`}>
          {cliche.phrases.length === 0 ? (
            <p className="text-xs text-text-muted">
              没有命中词库里的表达。词库收录 {formatInt(cliche.lexicon_size)}{" "}
              个常见说法，没命中只说明库里没有，不等于文字新。
            </p>
          ) : (
            <div className="overflow-x-auto">
              <table className="w-full text-xs">
                <thead>
                  <tr className="text-text-muted border-b border-gray-700">
                    <th className="text-left py-1.5 font-medium">短语</th>
                    <th className="text-left py-1.5 font-medium">类型</th>
                    <th className="text-right py-1.5 font-medium">次数</th>
                    <th className="text-right py-1.5 font-medium">占比</th>
                    <th className="text-right py-1.5 font-medium">覆盖</th>
                    <th className="text-left py-1.5 font-medium">首次 → 末次</th>
                    <th className="text-left py-1.5 font-medium">怎么改</th>
                  </tr>
                </thead>
                <tbody>
                  {cliche.phrases.map((p) => (
                    <tr key={p.phrase} className="border-b border-gray-700/40">
                      <td className="py-1.5">
                        <span className="font-medium">{p.phrase}</span>
                        {p.variants.length > 1 && (
                          <span className="text-text-muted">
                            （{p.variants.map(([v, n]) => `${v}×${n}`).join("、")}）
                          </span>
                        )}
                      </td>
                      <td className="py-1.5 text-text-muted">{clicheCategoryLabel(p.category)}</td>
                      <td className="py-1.5 text-right tabular-nums">{p.count}</td>
                      <td className="py-1.5 text-right text-text-muted tabular-nums">
                        {formatPercent(p.share)}
                      </td>
                      <td className="py-1.5 text-right text-text-muted tabular-nums">
                        {formatPercent(p.coverage)}
                      </td>
                      <td className="py-1.5 text-text-muted truncate max-w-[180px]">
                        {p.first_section} → {p.last_section}
                      </td>
                      <td className="py-1.5 text-text-muted truncate max-w-[260px]" title={p.note}>
                        {p.note}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </Panel>
      </div>

      <div className="mt-4">
        <Panel title={`用词密度曲线（${cliche.segment_count} 段 · 强度 0-100）`}>
          {curve.length === 0 ? (
            <p className="text-xs text-text-muted">未识别到曲线：文档没有可切分的段落。</p>
          ) : (
            <LineChart points={curve} color="#f59e0b" fill="rgba(245, 158, 11, 0.15)" />
          )}
          {cliche.chunked && (
            <p className="text-xs text-text-muted mt-2">
              章节不足 3 个：曲线按 {cliche.segment_count} 个等长块切分。
            </p>
          )}
        </Panel>
      </div>

      <div className="mt-4">
        <Panel
          title={`命中位置（显示 ${Math.min(cliche.hits.length, 20)} / ${cliche.total_hits} 处）`}
        >
          {cliche.hits.length === 0 ? (
            <p className="text-xs text-text-muted">没有命中。</p>
          ) : (
            <ul className="text-xs space-y-2">
              {cliche.hits.slice(0, 20).map((h) => (
                <li key={`${h.section}-${h.offset}-${h.phrase}`}>
                  <span className="font-medium">{h.phrase}</span>
                  <span className="text-text-muted">
                    （{h.section || "(root)"} · 第 {h.offset} 字）
                  </span>
                  {h.in_dialogue && (
                    <span className="px-1.5 py-0.5 ml-1 rounded border text-xs bg-sky-500/15 text-sky-400 border-sky-500/30">
                      对白内
                    </span>
                  )}
                  <div className="text-text-muted mt-0.5">{h.excerpt}</div>
                </li>
              ))}
            </ul>
          )}
        </Panel>
      </div>

      <div className="mt-4 grid md:grid-cols-2 gap-4">
        <Panel title={`用词问题（${cliche.issues.length}）`}>
          {cliche.issues.length === 0 ? (
            <p className="text-xs text-text-muted">没有发现需要处理的用词问题。</p>
          ) : (
            <ul className="text-xs space-y-2">
              {cliche.issues.map((i) => (
                <li key={`${i.kind}-${i.section}-${i.detail}`}>
                  <span
                    className={`px-2 py-0.5 rounded border text-xs ${clicheIssueStyle(i.kind)}`}
                  >
                    {clicheIssueLabel(i.kind)}
                  </span>
                  <span className="text-text-muted">
                    （{i.section || "全文"}）{i.detail}
                  </span>
                </li>
              ))}
            </ul>
          )}
        </Panel>

        <Panel title="评分说明">
          <p className="text-xs text-text-muted">
            用词评分从 100 起扣：每万字超过 4 处的部分每处 −2.5（最多 −45），反复使用的短语每次命中
            −1（最多 −20），最常用的一句占去两成以上再 −10；不足 1000 字不评分。
          </p>
          <p className="text-xs text-text-muted mt-3 pt-2 border-t border-gray-700/60">
            词库收录 {formatInt(cliche.lexicon_size)}
            个说法，只按字面匹配：命中只说明撞上库里常见的写法，不等于写得差。是否真的是陈词滥调、要不要改，由人来判断。
          </p>
        </Panel>
      </div>
    </section>
  );
}

export default ClicheSection;
