import { Panel, SectionTitle, StatCard, SECTION_ANCHOR } from "../Cards";
import BarList from "../charts/BarList";
import LineChart from "../charts/LineChart";
import {
  explicitAdvisoryLabel,
  explicitAdvisoryStyle,
  explicitCategoryLabel,
  explicitIssueLabel,
  explicitIssueStyle,
  explicitTierLabel,
  explicitTierStyle,
  formatFixed,
  formatInt,
  formatPercent,
} from "../../lib/format";
import type { ExplicitStats } from "../../types";

interface ExplicitSectionProps {
  explicit: ExplicitStats;
}

/**
 * 显式语言 section: what the text contains, stated as a content notice rather than
 * as a mark against it.
 *
 * There is deliberately no score here. A war novel is not worse than a pastoral one,
 * so the section reports presence and strength and stops: the advisory level, all
 * three strength tiers (a zero has to look like a measurement and not like a missing
 * one), the families an advisory would name, the terms that matched together with the
 * note the library carries for each, and the hits with the sentence around them so the
 * reader can check the grade against the context instead of taking it on trust.
 */
function ExplicitSection({ explicit }: ExplicitSectionProps) {
  const curve = explicit.points.map((p) => ({
    label: p.section || "(root)",
    value: p.intensity / 100,
  }));
  // A peak section only exists once some segment is long enough to carry a rate.
  const comparable = explicit.peak_section !== "";

  return (
    <section id="explicit" className={SECTION_ANCHOR}>
      <SectionTitle>显式语言</SectionTitle>

      <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
        <StatCard label="命中总数" value={formatInt(explicit.total_hits)} />
        <StatCard label="每万字命中" value={`${formatFixed(explicit.per_10k)} 处`} />
        <StatCard label="命中词条" value={formatInt(explicit.distinct_terms)} />
        <StatCard label="对白内命中" value={formatInt(explicit.dialogue_hits)} />
      </div>

      <div className="mt-3 flex flex-wrap items-center gap-2">
        <span
          className={`px-2 py-0.5 rounded border text-xs ${explicitAdvisoryStyle(explicit.advisory)}`}
        >
          内容分级：{explicitAdvisoryLabel(explicit.advisory)}
        </span>
        {/* All three tiers always render: the shape of the line is what tells a reader
            that a zero is a zero rather than a measurement that went missing. */}
        {explicit.tiers.map(([tier, count]) => (
          <span
            key={tier}
            className={`px-2 py-0.5 rounded border text-xs ${explicitTierStyle(tier)}`}
          >
            {explicitTierLabel(tier)} {formatInt(count)} 处
          </span>
        ))}
        <span className="text-xs text-text-muted">
          词库 {formatInt(explicit.lexicon_size)} 项 /{" "}
          {formatInt(explicit.lexicon_variants)} 个形变
        </span>
      </div>

      <p className="mt-2 text-xs text-text-muted">
        全文 {formatInt(explicit.char_count)} 字：命中 {formatInt(explicit.total_hits)} 处 ·
        叙述内 {formatInt(explicit.total_hits - explicit.dialogue_hits)} 处。本维度不给出评分：一部战争题材并不比田园题材更差，露骨与否是内容属性而不是质量。
      </p>

      {explicit.advisory_categories.length > 0 && (
        <p className="mt-1 text-xs text-text-muted">
          需要读者提示的内容：
          {explicit.advisory_categories.map(explicitCategoryLabel).join("、")}
          （按词库分级，只说明词面）
        </p>
      )}

      {comparable ? (
        <p className="mt-1 text-xs text-text-muted">
          最常用「{explicit.top_term}」占命中 {formatPercent(explicit.top_share)} · 峰值{" "}
          {explicit.peak_section}（强度 {formatFixed(explicit.peak_intensity)}）
        </p>
      ) : (
        <p className="mt-1 text-xs text-text-muted">
          各段均不足以支撑每万字命中率，不比较段落间的密度差异。
        </p>
      )}

      <div className="mt-4">
        <Panel title={`用词密度曲线（${explicit.segment_count} 段 · 强度 0-100）`}>
          {curve.length === 0 ? (
            <p className="text-xs text-text-muted">未识别到曲线：文档没有可切分的段落。</p>
          ) : (
            <LineChart points={curve} color="var(--color-dim-explicit)" />
          )}
          {explicit.chunked && (
            <p className="text-xs text-text-muted mt-2">
              章节不足 3 个：曲线按 {explicit.segment_count} 个等长块切分。
            </p>
          )}
        </Panel>
      </div>
      <div className="mt-4 grid md:grid-cols-2 gap-4">
        <Panel title="内容类型分布">
          {explicit.categories.length === 0 ? (
            <p className="text-xs text-text-muted">没有命中，因此没有类型分布。</p>
          ) : (
            <BarList
              items={explicit.categories.map(([category, count]) => ({
                label: explicitCategoryLabel(category),
                value: count,
              }))}
              colorClass="bg-firm"
            />
          )}
        </Panel>

        <Panel title={`内容提示（${explicit.issues.length}）`}>
          {explicit.issues.length === 0 ? (
            <p className="text-xs text-text-muted">
              没有达到提示门槛的内容类型。词库未见不等于内容温和。
            </p>
          ) : (
            <ul className="text-xs space-y-2">
              {explicit.issues.map((i, idx) => (
                <li key={`${i.kind}-${i.section}-${idx}`}>
                  <span
                    className={`px-2 py-0.5 rounded border text-xs ${explicitIssueStyle(i.kind)}`}
                  >
                    {explicitIssueLabel(i.kind)}
                  </span>
                  <span className="text-text-muted">
                    （{i.section || "全文"}）{i.detail}
                  </span>
                </li>
              ))}
            </ul>
          )}
        </Panel>
      </div>
      <div className="mt-4">
        <Panel title={`命中词条（${explicit.terms.length} 个）`}>
          {explicit.terms.length === 0 ? (
            <p className="text-xs text-text-muted">没有命中词库里的用词。</p>
          ) : (
            <div className="overflow-x-auto">
              <table className="w-full text-xs">
                <thead className="text-text-muted border-b border-line/60">
                  <tr>
                    <th className="text-left py-1.5 font-medium">词条</th>
                    <th className="text-left py-1.5 font-medium">类型</th>
                    <th className="text-left py-1.5 font-medium">强度</th>
                    <th className="text-right py-1.5 font-medium">次数</th>
                    <th className="text-right py-1.5 font-medium">占比</th>
                    <th className="text-right py-1.5 font-medium">覆盖</th>
                    <th className="text-left py-1.5 font-medium">词库说明</th>
                  </tr>
                </thead>
                <tbody>
                  {explicit.terms.map((t) => (
                    <tr key={t.term} className="border-b border-line/40">
                      <td className="py-1.5">
                        <span className="font-medium">{t.term}</span>
                        {t.variants.length > 1 && (
                          <span className="text-text-muted">
                            （{t.variants.map(([v, n]) => `${v}×${n}`).join("、")}）
                          </span>
                        )}
                      </td>
                      <td className="py-1.5 text-text-muted">
                        {explicitCategoryLabel(t.category)}
                      </td>
                      <td className="py-1.5">
                        <span
                          className={`px-1.5 py-0.5 rounded border text-xs ${explicitTierStyle(t.tier)}`}
                        >
                          {explicitTierLabel(t.tier)}
                        </span>
                      </td>
                      <td className="py-1.5 text-right tabular-nums">{t.count}</td>
                      <td className="py-1.5 text-right text-text-muted tabular-nums">
                        {formatPercent(t.share)}
                      </td>
                      <td className="py-1.5 text-right text-text-muted tabular-nums">
                        {formatPercent(t.coverage)}
                      </td>
                      <td
                        className="py-1.5 text-text-muted truncate max-w-[260px]"
                        title={t.note}
                      >
                        {t.note}
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
        <Panel
          title={`命中位置（显示 ${Math.min(explicit.hits.length, 20)} / ${explicit.total_hits} 处）`}
        >
          {explicit.hits.length === 0 ? (
            <p className="text-xs text-text-muted">没有命中词库里的用词。</p>
          ) : (
            <ul className="text-xs space-y-2">
              {explicit.hits.slice(0, 20).map((h) => (
                <li key={`${h.section}-${h.offset}-${h.term}`}>
                  <span className="font-medium">{h.term}</span>
                  <span
                    className={`px-1.5 py-0.5 ml-1 rounded border text-xs ${explicitTierStyle(h.tier)}`}
                  >
                    {explicitTierLabel(h.tier)}
                  </span>
                  <span className="text-text-muted">
                    （{h.section || "(root)"} · 第 {h.offset} 字）
                  </span>
                  {h.in_dialogue && (
                    <span className="px-1.5 py-0.5 ml-1 rounded border text-xs bg-info/15 text-info border-info/30">
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

      <div className="mt-4">
        <Panel title="分级说明">
          <p className="text-xs text-text-muted">
            词库按七个内容族（粗口辱骂、性描写、性暴力、暴力血腥、自伤、贬损歧视、毒品）与三档用词强度（轻、中、重）匹配：最长匹配优先，命中不重叠。内容分级取全文最强的那一档；自伤族在中等强度即会提示，因为这一族里一次提及比密度更值得注意。
          </p>
          <p className="text-xs text-text-muted mt-3 pt-2 border-t border-line/60">
            命中只说明词面撞上词库，不等于剧情如此；分级只反映用词强度，不反映事件本身。词库未见只说明这{" "}
            {formatInt(explicit.lexicon_size)} 个模式没有命中，不代表内容温和；是否需要读者提示由人决定。
          </p>
        </Panel>
      </div>
    </section>
  );
}

export default ExplicitSection;