import { Panel, SectionTitle, StatCard } from "../Cards";
import LineChart from "../charts/LineChart";
import BarList from "../charts/BarList";
import {
  formatInt,
  formatOptionalScore,
  formatPercent,
  themeConcentrationLabel,
  themeReachLabel,
  themeReachStyle,
  themeSourceLabel,
} from "../../lib/format";
import type { ThemeStats } from "../../types";

interface ThemeSectionProps {
  theme: ThemeStats;
  /** Segment names on the shared X axis, so the curve reads like the others. */
  segmentLabels: string[];
}

/**
 * 主题分析 section: which subjects the text keeps coming back to, and how far
 * each of them reaches.
 *
 * The engine collects recurring nouns, folds obvious variants together (出版 and
 * 出版业 are one subject), and reports coverage rather than importance — a word
 * said often is not automatically the book's theme. So this view always names the
 * vocabulary it read and never claims a motif: it shows repetition and lets the
 * writer decide whether it was on purpose.
 */
function ThemeSection({ theme, segmentLabels }: ThemeSectionProps) {
  const score = theme.theme_score;
  const leader = theme.themes[0];
  // LineChart expects 0..1; scale by the busiest segment of the shown theme so a
  // theme that is merely quiet still fills its chart.
  const peak = leader ? Math.max(...leader.curve, 1) : 1;
  const curve = leader
    ? leader.curve.map((value, idx) => ({
        label: segmentLabels[idx] ?? `第 ${idx + 1} 段`,
        value: value / peak,
      }))
    : [];

  return (
    <section>
      <SectionTitle>主题分析</SectionTitle>

      <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
        <StatCard label="主题评分" value={formatOptionalScore(score)} />
        <StatCard label="候选主题" value={formatInt(theme.candidate_count)} />
        <StatCard label="名词命中" value={formatInt(theme.total_mentions)} />
        <StatCard label="单次词占比" value={formatPercent(theme.singleton_share)} />
      </div>

      <div className="mt-3 flex flex-wrap items-center gap-2">
        <span className="px-2 py-0.5 rounded border text-xs bg-gray-500/15 text-text-muted border-gray-500/30">
          词汇来源：{themeSourceLabel(theme.source)}
        </span>
        <span className="text-xs text-text-muted">
          最大主题占比 {formatPercent(theme.top_share)} · 前三合计 {formatPercent(theme.top3_share)}
          {theme.dominant_terms.length > 0
            ? ` · ${theme.dominant_terms.join("、")}`
            : " · 无主导主题"}
        </span>
      </div>

      {theme.source === "none" && (
        <p className="mt-2 text-xs text-amber-400">
          没有可用于主题分析的词汇：导入结果里既没有名词信号也没有名词词性，或所有词都太短、太泛，因此不给出评分。
        </p>
      )}
      {theme.source === "noun_tokens" && (
        <p className="mt-2 text-xs text-sky-400">
          导入结果没有名词信号，已改用词性标注中的名词：清单仍可用，但比导入器给出的名词信号更容易混入普通词。
        </p>
      )}
      {theme.source !== "none" && score === null && (
        <p className="mt-2 text-xs text-amber-400">
          没有任何词反复出现（达到 3 次并跨越 2 段以上），因此不给出评分：这可能不是叙事文本，也可能是用词过于分散。
        </p>
      )}

      <div className="mt-4">
        <Panel title={`主题清单（${theme.themes.length} 个 · 共 ${theme.section_count} 段）`}>
          {theme.themes.length === 0 ? (
            <p className="text-xs text-text-muted">
              未识别到主题：没有词达到 3 次提及且跨越 2 段以上。
            </p>
          ) : (
            <div className="overflow-x-auto">
              <table className="w-full text-xs">
                <thead>
                  <tr className="text-text-muted border-b border-gray-700">
                    <th className="text-left py-1.5 font-medium">主题</th>
                    <th className="text-right py-1.5 font-medium">提及</th>
                    <th className="text-right py-1.5 font-medium">覆盖</th>
                    <th className="text-right py-1.5 font-medium">占比</th>
                    <th className="text-left py-1.5 font-medium">首次 → 末次</th>
                    <th className="text-left py-1.5 font-medium">贯穿度</th>
                    <th className="text-left py-1.5 font-medium">分布</th>
                  </tr>
                </thead>
                <tbody>
                  {theme.themes.map((t) => (
                    <tr key={t.term} className="border-b border-gray-700/40">
                      <td className="py-1.5 truncate max-w-[220px]" title={t.term}>
                        <span className="font-medium">{t.term}</span>
                        {t.members.length > 1 && (
                          <span className="text-text-muted">
                            （{t.members.map(([m, c]) => `${m}×${c}`).join("、")}）
                          </span>
                        )}
                      </td>
                      <td className="py-1.5 text-right tabular-nums">{t.mentions}</td>
                      <td className="py-1.5 text-right text-text-muted tabular-nums">
                        {formatPercent(t.coverage)}
                      </td>
                      <td className="py-1.5 text-right text-text-muted tabular-nums">
                        {formatPercent(t.share)}
                      </td>
                      <td className="py-1.5 text-text-muted truncate max-w-[200px]">
                        {t.first_section || "(root)"} → {t.last_section || "(root)"}
                        {t.longest_gap > 0 && ` · 空缺 ${t.longest_gap} 段`}
                      </td>
                      <td className="py-1.5">
                        <span
                          className={`px-2 py-0.5 rounded border text-xs ${themeReachStyle(t.reach)}`}
                        >
                          {themeReachLabel(t.reach)}
                        </span>
                      </td>
                      <td className="py-1.5 text-text-muted">
                        {themeConcentrationLabel(t.concentration)}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </Panel>
      </div>

      <div className="mt-4 grid md:grid-cols-2 gap-4">
        <Panel
          title={leader ? `「${leader.term}」贯穿曲线（峰值 ${peak} 次/段）` : "主题贯穿曲线"}
        >
          {curve.length === 0 ? (
            <p className="text-xs text-text-muted">未识别到主题，因此没有曲线。</p>
          ) : (
            <LineChart points={curve} color="#34d399" fill="rgba(52, 211, 153, 0.15)" />
          )}
          <p className="text-xs text-text-muted mt-2">
            横轴与节奏、冲突曲线一致（{theme.section_count} 段）：同一位置可以横向对照。
          </p>
        </Panel>

        <Panel title="主题体量">
          <BarList
            items={theme.themes.map((t) => ({ label: t.term, value: t.mentions }))}
            colorClass="bg-emerald-400"
          />
          <p className="text-xs text-text-muted mt-3 pt-2 border-t border-gray-700/60">
            主题评分从 100 起扣：没有任何主题覆盖 20% 以上段落 −25，单一主题占比超过 35% 最多 −15，
            前三主题合计不足 12% 最多 −20，仅出现一次的词贡献超过 50% 最多 −25；没有可用名词词汇时不评分。
          </p>
        </Panel>
      </div>

      <div className="mt-4">
        <Panel title="反复出现不等于母题">
          <p className="text-xs text-text-muted">
            清单按名词词汇的复现与分布计算：包含关系（出版 / 出版业）与同段共现的词已合并为一个主题，
            人名与时间表达已排除。反复出现的词可能是母题，也可能只是同一个场景被反复提及——是否有意为之，需要人来判断。
          </p>
        </Panel>
      </div>


    </section>
  );
}

export default ThemeSection;
