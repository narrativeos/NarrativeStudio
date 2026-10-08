import { useEffect, useState, type ReactNode } from "react";
import { invoke } from "@tauri-apps/api/core";
import { save } from "@tauri-apps/plugin-dialog";
import BarList from "../components/charts/BarList";
import LineChart from "../components/charts/LineChart";
import RadarChart from "../components/charts/RadarChart";
import {
  arcShapeLabel,
  entityTotal,
  formatInt,
  formatPercent,
  formatScore,
  scoreColor,
  scoreTextColor,
  severityLabel,
  severityStyle,
} from "../lib/format";
import type { ProjectAnalysis, ReportMeta } from "../types";

interface AnalysisProps {
  projectId: string;
  onBack: () => void;
}

/**
 * Single-project analysis view. Runs T0 (statistical) + T1 (structural)
 * analysis plus a rule-based assessment for the selected project, and
 * presents the full portrait grouped by dimension: overview, assessment,
 * statistics, text diagnostics, structure, and narrative.
 */
function Analysis({ projectId, onBack }: AnalysisProps) {
  const [data, setData] = useState<ProjectAnalysis | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  // Bumping `reload` re-runs the load; `force` marks a run that must bypass and
  // replace the persisted cache ("重新分析").
  const [reload, setReload] = useState(0);
  const [force, setForce] = useState(false);
  // Report export: `exporting` guards the button, `exportNote` confirms the path.
  const [exporting, setExporting] = useState(false);
  const [exportNote, setExportNote] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    async function run() {
      setLoading(true);
      setError(null);
      setData(null);
      try {
        const result = await invoke<ProjectAnalysis>("analyze_project", {
          projectId,
          force,
        });
        if (!cancelled) setData(result);
      } catch (e) {
        if (!cancelled) setError(String(e));
      } finally {
        if (!cancelled) setLoading(false);
      }
    }
    run();
    return () => {
      cancelled = true;
    };
  }, [projectId, reload, force]);

  /**
   * Generate the Markdown report, then ask where to save it.
   *
   * Generation and writing are separate commands because the backend stores the
   * report first: the file the user picks is an export of a persisted report, so
   * re-exporting later cannot silently produce different numbers.
   */
  async function exportReport() {
    setExporting(true);
    setExportNote(null);
    try {
      const report = await invoke<ReportMeta>("report_generate", { projectId });
      const target = await save({
        defaultPath: `${report.title}.md`,
        filters: [{ name: "Markdown", extensions: ["md"] }],
      });
      if (!target) return; // user cancelled the dialog
      await invoke("report_export", { reportId: report.report_id, path: target });
      setExportNote(`已导出：${target}`);
    } catch (e) {
      setError(String(e));
    } finally {
      setExporting(false);
    }
  }

  if (loading) {
    return <div className="text-text-muted">分析中…</div>;
  }

  if (error) {
    return (
      <div className="text-red-400">
        <p>分析失败：</p>
        <pre className="text-xs mt-2 whitespace-pre-wrap">{error}</pre>
      </div>
    );
  }

  if (!data) return null;

  const { t0, t1 } = data;
  const sectionsBySize = [...t1.sections].sort((a, b) => b.char_count - a.char_count);

  return (
    <div className="space-y-8">
      {/* Header */}
      <div className="flex items-center gap-3">
        <button
          onClick={onBack}
          className="px-2 py-1 text-lg leading-none text-text-muted hover:text-text hover:bg-gray-700/50 rounded-md transition-colors"
          title="返回项目列表"
        >
          ←
        </button>
        <div className="min-w-0">
          <h2 className="text-xl font-semibold truncate">{data.name}</h2>
          <p className="text-xs text-text-muted mt-0.5">
            {formatInt(t0.block_count)} 块 · {formatInt(t1.section_count)} 章节 ·{" "}
            {formatInt(t0.char_count)} 字
          </p>
        </div>
        <div className="ml-auto flex items-center gap-2 shrink-0">
          {data.cached && (
            <span
              className="text-xs text-text-muted"
              title="结果来自上次分析的持久化结果；文本变化或点击重新分析后会重算"
            >
              缓存结果
            </span>
          )}
          <button
            onClick={exportReport}
            disabled={exporting}
            className="px-3 py-1.5 text-sm rounded-md bg-accent/20 text-accent hover:bg-accent/30 transition-colors disabled:opacity-50"
            title="生成 Markdown 报告并选择保存位置"
          >
            {exporting ? "生成中…" : "导出报告"}
          </button>
          <button
            onClick={() => {
              setForce(true);
              setReload((n) => n + 1);
            }}
            className="px-3 py-1.5 text-sm rounded-md bg-gray-700/60 hover:bg-gray-700 text-text transition-colors"
            title="忽略缓存，重新计算并覆盖已保存的分析结果"
          >
            重新分析
          </button>
        </div>
      </div>

      {exportNote && (
        <p className="text-xs text-accent -mt-2 truncate" title={exportNote}>
          {exportNote}
        </p>
      )}

      {/* Overview metrics */}
      <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-6 gap-3">
        <StatCard label="字数" value={formatInt(t0.word_count)} />
        <StatCard label="文本块" value={formatInt(t0.block_count)} />
        <StatCard label="章节" value={formatInt(t1.section_count)} />
        <StatCard label="句子" value={formatInt(t0.sentence_count)} />
        <StatCard label="实体" value={formatInt(entityTotal(t0))} />
        <StatCard label="名词信号" value={formatInt(t0.noun_signal_count)} />
      </div>

      {/* Overall assessment */}
      <section>
        <SectionTitle>总体评估</SectionTitle>
        <div className="grid md:grid-cols-2 gap-4">
          <Panel title={`综合评分 ${formatScore(data.assessment.overall)} / 100`}>
            <div className="flex flex-col sm:flex-row items-center gap-4">
              {/* The radar shows the shape of the portrait; the bars next to it
                  give the exact numbers it is drawn from. */}
              <div className="shrink-0 self-center sm:self-start">
                <RadarChart dimensions={data.assessment.dimensions} />
              </div>
              <div className="flex-1 w-full space-y-2.5">
                {data.assessment.dimensions.map((d) => (
                  <div key={d.key} className="flex items-center gap-2 text-xs">
                    <span className="w-16 shrink-0 text-text">{d.label}</span>
                    <div className="flex-1 h-4 bg-gray-800/60 rounded overflow-hidden">
                      <div
                        className={`h-full rounded ${scoreColor(d.score)}`}
                        style={{ width: `${Math.max(2, d.score)}%` }}
                      />
                    </div>
                    <span className="w-8 shrink-0 text-right text-text-muted tabular-nums">
                      {formatScore(d.score)}
                    </span>
                  </div>
                ))}
              </div>
            </div>
            <div className="mt-3 space-y-1">
              {data.assessment.dimensions.map((d) => (
                <p key={d.key} className="text-[11px] text-text-muted">
                  {d.label}：{d.detail}
                </p>
              ))}
            </div>
          </Panel>
          <Panel title={`关键建议（${data.assessment.recommendations.length}）`}>
            {data.assessment.recommendations.length === 0 ? (
              <p className="text-xs text-text-muted">
                未检测到明显问题，文本质量良好。
              </p>
            ) : (
              <ul className="space-y-2">
                {data.assessment.recommendations.map((r, i) => (
                  <li key={i} className="text-xs flex items-start gap-2">
                    <span className={`shrink-0 mt-0.5 px-1.5 py-0.5 rounded border text-[10px] ${severityStyle(r.severity)}`}>
                      {severityLabel(r.severity)}
                    </span>
                    <span>
                      <span className="text-text">{r.title}</span>
                      <span className="text-text-muted"> — {r.detail}</span>
                    </span>
                  </li>
                ))}
              </ul>
            )}
          </Panel>
        </div>
      </section>

      {/* Statistical analysis */}
      <section>
        <SectionTitle>统计分析</SectionTitle>
        <div className="grid md:grid-cols-2 gap-4">
          <Panel title="高频词 Top 15">
            <BarList items={t0.top_words.map(([label, value]) => ({ label, value }))} limit={15} />
          </Panel>
          <Panel title="关键实体 Top 15">
            <BarList
              items={t0.top_entities.map(([label, value]) => ({ label, value }))}
              limit={15}
              colorClass="bg-emerald-400"
            />
          </Panel>
          <Panel title="领域术语（名词信号）Top 15">
            <BarList
              items={t0.top_noun_signals.map(([label, value]) => ({ label, value }))}
              limit={15}
              colorClass="bg-amber-400"
            />
          </Panel>
          <Panel title="实体类别分布">
            <BarList
              items={t0.entity_counts.map(([label, value]) => ({ label, value }))}
              colorClass="bg-sky-400"
            />
          </Panel>
        </div>
        <div className="mt-4">
          <Panel title="词性分布">
            <BarList
              items={t0.pos_distribution.map(([label, value]) => ({ label, value }))}
              limit={12}
              colorClass="bg-violet-400"
            />
          </Panel>
        </div>
        <div className="mt-4 grid md:grid-cols-2 gap-4">
          <Panel title={`句子统计（${t0.sentence_count.toLocaleString()} 句，平均 ${t0.avg_sentence_chars.toFixed(1)} 字）`}>
            <BarList
              items={t0.sentence_length_distribution.map(([label, value]) => ({
                label: `${label} 字`,
                value,
              }))}
              colorClass="bg-indigo-400"
            />
          </Panel>
          <Panel title="可读性">
            <div className="flex items-center gap-4">
              <div
                className={`text-3xl font-semibold tabular-nums ${scoreTextColor(t0.readability.score)}`}
              >
                {formatScore(t0.readability.score)}
              </div>
              <div className="text-xs text-text-muted">
                <p>等级：{t0.readability.level}</p>
                <p>方法：{t0.readability.method}</p>
              </div>
            </div>
          </Panel>
        </div>
      </section>

      {/* Text diagnostics */}
      <section>
        <SectionTitle>文本诊断</SectionTitle>
        <div className="grid md:grid-cols-3 gap-4">
          <Panel title="重复短语 Top 15">
            <BarList
              items={t0.top_repeated_phrases.map((p) => ({ label: p.text, value: p.count }))}
              limit={15}
              colorClass="bg-pink-400"
            />
          </Panel>
          <Panel title={`高频副词 Top 15（共 ${t0.adverb_count.toLocaleString()} 个）`}>
            <BarList
              items={t0.top_adverbs.map(([label, value]) => ({ label, value }))}
              limit={15}
              colorClass="bg-orange-400"
            />
          </Panel>
          <Panel title={`高频形容词 Top 15（共 ${t0.adjective_count.toLocaleString()} 个）`}>
            <BarList
              items={t0.top_adjectives.map(([label, value]) => ({ label, value }))}
              limit={15}
              colorClass="bg-lime-400"
            />
          </Panel>
        </div>
      </section>

      {/* Structural analysis */}
      <section>
        <SectionTitle>结构分析</SectionTitle>
        <div className="grid md:grid-cols-2 gap-4">
          <Panel title="块类型分布">
            <BarList
              items={t1.block_type_distribution.map(([label, value]) => ({ label, value }))}
              colorClass="bg-rose-400"
            />
          </Panel>
          <Panel title="章节体量（按字数）">
            <BarList
              items={sectionsBySize.map((s) => ({ label: s.path || "(root)", value: s.char_count }))}
              limit={15}
              colorClass="bg-teal-400"
            />
          </Panel>
        </div>
        <div className="mt-4">
          <Panel title={`章节列表（${t1.sections.length}）`}>
            {t1.sections.length === 0 ? (
              <p className="text-xs text-text-muted">暂无章节数据</p>
            ) : (
              <div className="overflow-x-auto">
                <table className="w-full text-xs">
                  <thead>
                    <tr className="text-text-muted border-b border-gray-700">
                      <th className="text-left py-1.5 font-medium">章节</th>
                      <th className="text-right py-1.5 font-medium">块数</th>
                      <th className="text-right py-1.5 font-medium">字数</th>
                    </tr>
                  </thead>
                  <tbody>
                    {sectionsBySize.map((s) => (
                      <tr key={s.path} className="border-b border-gray-700/40">
                        <td className="py-1.5 truncate max-w-md" title={s.path}>
                          {s.path || "(root)"}
                        </td>
                        <td className="py-1.5 text-right text-text-muted tabular-nums">
                          {s.block_count}
                        </td>
                        <td className="py-1.5 text-right text-text-muted tabular-nums">
                          {s.char_count.toLocaleString()}
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

      {/* Narrative analysis */}
      <section>
        <SectionTitle>叙事分析</SectionTitle>
        <div className="grid md:grid-cols-2 gap-4">
          <Panel title={`叙事弧线（${arcShapeLabel(t1.arc.shape)}）`}>
            {t1.arc.points.length === 0 ? (
              <p className="text-xs text-text-muted">暂无数据</p>
            ) : (
              <LineChart
                points={t1.arc.points.map((p) => ({
                  label: p.section || "(root)",
                  value: p.intensity,
                }))}
              />
            )}
          </Panel>
          <Panel title="实体共现 Top 15（情节骨架）">
            <BarList
              items={t1.entity_cooccurrences.map(([label, value]) => ({ label, value }))}
              limit={15}
              colorClass="bg-cyan-400"
            />
          </Panel>
        </div>
        <div className="mt-4">
          <Panel title={`角色（${t1.characters.length}）`}>
            {t1.characters.length === 0 ? (
              <p className="text-xs text-text-muted">
                未检测到人物实体（非叙事文本或 NER 未识别人名）。
              </p>
            ) : (
              <div className="overflow-x-auto">
                <table className="w-full text-xs">
                  <thead>
                    <tr className="text-text-muted border-b border-gray-700">
                      <th className="text-left py-1.5 font-medium">角色</th>
                      <th className="text-right py-1.5 font-medium">提及</th>
                      <th className="text-right py-1.5 font-medium">跨度</th>
                      <th className="text-left py-1.5 pl-4 font-medium">首次出现</th>
                      <th className="text-left py-1.5 pl-4 font-medium">最后出现</th>
                      <th className="text-left py-1.5 pl-4 font-medium">常共现</th>
                    </tr>
                  </thead>
                  <tbody>
                    {t1.characters.map((c) => (
                      <tr key={c.name} className="border-b border-gray-700/40">
                        <td className="py-1.5 font-medium">{c.name}</td>
                        <td className="py-1.5 text-right text-text-muted tabular-nums">
                          {c.mentions}
                        </td>
                        <td className="py-1.5 text-right text-text-muted tabular-nums">
                          {formatPercent(c.span_ratio)}
                        </td>
                        <td className="py-1.5 pl-4 truncate max-w-[160px]" title={c.first_section}>
                          {c.first_section || "(root)"}
                        </td>
                        <td className="py-1.5 pl-4 truncate max-w-[160px]" title={c.last_section}>
                          {c.last_section || "(root)"}
                        </td>
                        <td className="py-1.5 pl-4 text-text-muted">
                          {c.top_cooccurrences.length === 0
                            ? "—"
                            : c.top_cooccurrences
                                .slice(0, 3)
                                .map(([n, cnt]) => `${n}(${cnt})`)
                                .join("、")}
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
    </div>
  );
}

function SectionTitle({ children }: { children: ReactNode }) {
  return (
    <h3 className="text-sm font-semibold text-text-muted uppercase tracking-wide mb-3">
      {children}
    </h3>
  );
}

function StatCard({ label, value }: { label: string; value: string }) {
  return (
    <div className="p-3 bg-surface-alt rounded-lg border border-gray-700">
      <p className="text-xs text-text-muted">{label}</p>
      <p className="text-xl font-semibold mt-1 tabular-nums">{value}</p>
    </div>
  );
}

function Panel({ title, children }: { title: string; children: ReactNode }) {
  return (
    <div className="p-4 bg-surface-alt rounded-lg border border-gray-700">
      <h4 className="text-sm font-medium text-text-muted mb-3">{title}</h4>
      {children}
    </div>
  );
}

export default Analysis;
