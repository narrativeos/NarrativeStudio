import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { save } from "@tauri-apps/plugin-dialog";
import { Panel, SectionTitle, StatCard, SECTION_ANCHOR } from "../components/Cards";
import SectionNav from "../components/SectionNav";
import { useShortcuts } from "../lib/shortcuts";
import ClicheSection from "../components/analysis/ClicheSection";
import ChecklistSection from "../components/analysis/ChecklistSection";
import ConflictSection from "../components/analysis/ConflictSection";
import DialogueSection from "../components/analysis/DialogueSection";
import ExplicitSection from "../components/analysis/ExplicitSection";
import SettingSection from "../components/analysis/SettingSection";
import StructureSection from "../components/analysis/StructureSection";
import ThemeSection from "../components/analysis/ThemeSection";
import TimelineSection from "../components/analysis/TimelineSection";
import VoiceSection from "../components/analysis/VoiceSection";
import BarList from "../components/charts/BarList";
import LineChart from "../components/charts/LineChart";
import RadarChart from "../components/charts/RadarChart";
import RelationshipGraph from "../components/charts/RelationshipGraph";
import {
  arcShapeLabel,
  coverageNote,
  entityTotal,
  formatFixed,
  formatInt,
  formatPercent,
  formatScore,
  pacingVerdictLabel,
  pacingVerdictStyle,
  plotShapeLabel,
  plotShapeStyle,
  scoreColor,
  scoreTextColor,
  severityLabel,
  severityStyle,
  wordsMeasured,
} from "../lib/format";
import type { ProjectAnalysis, ReportMeta } from "../types";

interface AnalysisProps {
  projectId: string;
  onBack: () => void;
}

/**
 * Single-project analysis view. Runs T0 (statistical) + T1 (structural) + plot
 * + pacing + conflict + theme + setting + cliché + dialogue + explicit language
 * + story structure + time line analysis plus a rule-based assessment for the
 * selected project, and presents the full portrait grouped by dimension: overview,
 * assessment, statistics, text diagnostics, structure, narrative, plot, pacing,
 * conflict, theme, setting, cliché, dialogue, explicit language, story structure,
 * time line, author voice and the final review checklist.
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
  const [exportingPdf, setExportingPdf] = useState(false);
  const [exportingKnowsphere, setExportingKnowsphere] = useState(false);
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

  /**
   * Recompute now, ignoring the persisted cache.
   *
   * Named because the keyboard needs it too: ⌘R is the reader's "I changed the
   * text" gesture, and a shortcut that does something different from the button
   * it mirrors is worse than no shortcut.
   */
  function reanalyze() {
    setForce(true);
    setReload((n) => n + 1);
  }

  // ⌘R and ⌘E belong to whichever project is open, so they are bound here and
  // not in the shell, which does not know what is showing. A run already in
  // flight is not started twice.
  useShortcuts({
    reanalyze: () => {
      if (!loading && !exporting) reanalyze();
    },
    export: () => {
      if (data && !exporting) void exportReport();
    },
  });

  /**
   * Export the report as print-ready HTML — the PDF path.
   *
   * The backend renders the HTML from the Markdown it already stored, so the
   * printed file carries the same numbers as the file next to it. The paper is
   * set to A4 in the file itself: opening it and pressing ⌘P is all that is
   * left, which is why no print dialog is opened from here.
   */
  async function exportPdf() {
    setExportingPdf(true);
    setExportNote(null);
    try {
      const report = await invoke<ReportMeta>("report_generate", { projectId });
      const target = await save({
        title: "导出打印就绪 HTML（用浏览器打开后 ⌘P 存为 PDF）",
        defaultPath: `${report.title}.html`,
        filters: [{ name: "HTML", extensions: ["html"] }],
      });
      if (!target) return; // user cancelled the dialog
      await invoke("report_export_html", { reportId: report.report_id, path: target });
      setExportNote(`已导出打印就绪 HTML：${target}（浏览器打开后按 ⌘P 存为 PDF）`);
    } catch (e) {
      setError(String(e));
    } finally {
      setExportingPdf(false);
    }
  }

  /**
   * Export the project as a `.knowsphere.json` file for NarratSphere.
   *
   * The backend renders it from the same (cached) analysis this page shows,
   * validates it against the Knowsphere schema contract, and only then
   * writes the chosen file — so a cancelled dialog costs nothing and a
   * written file is always loadable by NarratSphere.
   */
  async function exportKnowsphere() {
    setExportingKnowsphere(true);
    setExportNote(null);
    try {
      const target = await save({
        title: "导出知识球文件",
        defaultPath: `${data?.name ?? "project"}.knowsphere.json`,
        filters: [{ name: "Knowsphere", extensions: ["json"] }],
      });
      if (!target) return; // user cancelled the dialog
      await invoke("export_knowsphere", { projectId, path: target });
      setExportNote(`已导出知识球文件：${target}`);
    } catch (e) {
      setError(String(e));
    } finally {
      setExportingKnowsphere(false);
    }
  }

  if (loading) {
    return <div className="text-text-muted">分析中…</div>;
  }

  if (error) {
    return (
      <div className="text-bad">
        <p>分析失败：</p>
        <pre className="text-xs mt-2 whitespace-pre-wrap">{error}</pre>
      </div>
    );
  }

  if (!data) return null;

  const {
    t0,
    t1,
    plot,
    pacing,
    conflict,
    theme,
    setting,
    cliche,
    dialogue,
    explicit,
    structure,
    timeline,
    voice,
    checklist,
  } = data;
  const sectionsBySize = [...t1.sections].sort((a, b) => b.char_count - a.char_count);
  // Pacing, conflict and theme all segment the document the same way, so one list
  // of section names labels every curve's X axis.
  const segmentLabels = (conflict.points.length > 0 ? conflict.points : pacing.points).map(
    (p) => p.section,
  );

  return (
    <div className="space-y-8">
      {/* Header */}
      <div className="flex items-center gap-3">
        <button
          onClick={onBack}
          className="px-2 py-1 text-lg leading-none text-text-muted hover:text-text hover:bg-fill/50 rounded-md transition-colors"
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
            onClick={exportPdf}
            disabled={exportingPdf}
            className="px-3 py-1.5 text-sm rounded-md bg-accent/20 text-accent hover:bg-accent/30 transition-colors disabled:opacity-50"
            title="导出打印就绪 HTML：A4 排版，用浏览器打开后按 ⌘P 存为 PDF"
          >
            {exportingPdf ? "生成中…" : "导出 PDF"}
          </button>
          <button
            onClick={exportKnowsphere}
            disabled={exportingKnowsphere}
            className="px-3 py-1.5 text-sm rounded-md bg-accent/20 text-accent hover:bg-accent/30 transition-colors disabled:opacity-50"
            title="导出 .knowsphere.json 知识球文件，供 NarratSphere 打开"
          >
            {exportingKnowsphere ? "导出中…" : "导出知识球"}
          </button>
          <button
            onClick={reanalyze}
            disabled={loading || exporting}
            className="px-3 py-1.5 text-sm rounded-md bg-fill/60 hover:bg-fill text-text transition-colors disabled:opacity-50"
            title="忽略缓存，重新计算并覆盖已保存的分析结果（⌘R）"
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

      {/* What the import could not feed the engine. A text-only project reads
          differently: the missing numbers are gaps in the input, not findings. */}
      {data && coverageNote(data) && (
        <div className="mb-4 p-3 rounded-lg border border-accent/30 bg-surface-alt text-xs text-text-muted">
          {coverageNote(data)}
        </div>
      )}

      {/* Seventeen dimensions in one column: the bar that lets a reader go
          straight to the one they came for, and know which one they are in. */}
      <SectionNav />

      {/* Overview metrics */}
      <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-6 gap-3">
        <StatCard
          label="字数"
          value={data && !wordsMeasured(data.t0) ? "未测量" : formatInt(t0.word_count)}
        />
        <StatCard label="文本块" value={formatInt(t0.block_count)} />
        <StatCard label="章节" value={formatInt(t1.section_count)} />
        <StatCard label="句子" value={formatInt(t0.sentence_count)} />
        <StatCard
          label="实体"
          value={data && !data.setting.places_measured ? "未测量" : formatInt(entityTotal(t0))}
        />
        <StatCard label="名词信号" value={formatInt(t0.noun_signal_count)} />
      </div>

      {/* Statistical overview: the five T0/T1 dimensions and what they average to. */}
      <section id="overview" className={SECTION_ANCHOR}>
        <SectionTitle>统计概览</SectionTitle>
        <div className="grid md:grid-cols-2 gap-4">
          <Panel title={`统计均值 ${formatScore(data.assessment.overall)} / 100`}>
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
                    <div className="flex-1 h-4 bg-fill-soft/60 rounded overflow-hidden">
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
            {/* What the number averages, said out loud. The rest of this page is
                careful not to reduce a manuscript to one figure — 作者声音,
                故事结构, 显式语言 and the checklist carry no score by design — so
                a mean of five statistical dimensions must not read as a grade
                on the writing, and must not look like it covers the rest. */}
            <p className="mt-3 pt-2 border-t border-line/60 text-[11px] text-text-muted">
              这是上述五项的等权平均，全部来自 T0/T1 的统计量。情节、冲突、主题、场景、对话、故事结构、时间线与作者声音不参与：它们不给分（见各自小节）。这个数字描述的是行文统计，不是稿子的优劣。
            </p>
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
      <section id="statistics" className={SECTION_ANCHOR}>
        <SectionTitle>统计分析</SectionTitle>
        <div className="grid md:grid-cols-2 gap-4">
          <Panel title="高频词 Top 15">
            <BarList items={t0.top_words.map(([label, value]) => ({ label, value }))} limit={15} />
          </Panel>
          <Panel title="关键实体 Top 15">
            <BarList
              items={t0.top_entities.map(([label, value]) => ({ label, value }))}
              limit={15}
              colorClass="bg-good"
            />
          </Panel>
          <Panel title="领域术语（名词信号）Top 15">
            <BarList
              items={t0.top_noun_signals.map(([label, value]) => ({ label, value }))}
              limit={15}
              colorClass="bg-warn"
            />
          </Panel>
          <Panel title="实体类别分布">
            <BarList
              items={t0.entity_counts.map(([label, value]) => ({ label, value }))}
              colorClass="bg-info"
            />
          </Panel>
        </div>
        <div className="mt-4">
          <Panel title="词性分布">
            <BarList
              items={t0.pos_distribution.map(([label, value]) => ({ label, value }))}
              limit={12}
              colorClass="bg-cat-pos"
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
              colorClass="bg-cat-sentence"
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
      <section id="text" className={SECTION_ANCHOR}>
        <SectionTitle>文本诊断</SectionTitle>
        <div className="grid md:grid-cols-3 gap-4">
          <Panel title="重复短语 Top 15">
            <BarList
              items={t0.top_repeated_phrases.map((p) => ({ label: p.text, value: p.count }))}
              limit={15}
              colorClass="bg-cat-repeat"
            />
          </Panel>
          <Panel title={`高频副词 Top 15（共 ${t0.adverb_count.toLocaleString()} 个）`}>
            <BarList
              items={t0.top_adverbs.map(([label, value]) => ({ label, value }))}
              limit={15}
              colorClass="bg-cat-adverb"
            />
          </Panel>
          <Panel title={`高频形容词 Top 15（共 ${t0.adjective_count.toLocaleString()} 个）`}>
            <BarList
              items={t0.top_adjectives.map(([label, value]) => ({ label, value }))}
              limit={15}
              colorClass="bg-cat-adjective"
            />
          </Panel>
        </div>
      </section>

      {/* Structural analysis */}
      <section id="structure" className={SECTION_ANCHOR}>
        <SectionTitle>结构分析</SectionTitle>
        <div className="grid md:grid-cols-2 gap-4">
          <Panel title="块类型分布">
            <BarList
              items={t1.block_type_distribution.map(([label, value]) => ({ label, value }))}
              colorClass="bg-firm"
            />
          </Panel>
          <Panel title="章节体量（按字数）">
            <BarList
              items={sectionsBySize.map((s) => ({ label: s.path || "(root)", value: s.char_count }))}
              limit={15}
              colorClass="bg-cat-section"
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
                    <tr className="text-text-muted border-b border-line">
                      <th className="text-left py-1.5 font-medium">章节</th>
                      <th className="text-right py-1.5 font-medium">块数</th>
                      <th className="text-right py-1.5 font-medium">字数</th>
                    </tr>
                  </thead>
                  <tbody>
                    {sectionsBySize.map((s) => (
                      <tr key={s.path} className="border-b border-line/40">
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
      <section id="narrative" className={SECTION_ANCHOR}>
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
              colorClass="bg-cat-cooccur"
            />
          </Panel>
        </div>
        <div className="mt-4">
          <Panel title={`角色关系图（${t1.characters.length} 个角色）`}>
            <RelationshipGraph characters={t1.characters} />
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
                    <tr className="text-text-muted border-b border-line">
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
                      <tr key={c.name} className="border-b border-line/40">
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

      {/* Plot analysis */}
      <section id="plot" className={SECTION_ANCHOR}>
        <SectionTitle>情节分析</SectionTitle>
        <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
          <StatCard
            label="情节连贯性"
            value={
              plot.readable ? `${formatScore(plot.coherence_score)} / 100` : "读不到"
            }
          />
          <StatCard label="情节线" value={formatInt(plot.thread_count)} />
          <StatCard
            label="主线 / 支线"
            value={`${plot.main_plotlines.length} / ${plot.sub_plotlines.length}`}
          />
          <StatCard label="转折点" value={formatInt(plot.turning_points.length)} />
        </div>

        {!plot.readable && (
          <p className="mt-3 text-xs text-text-muted">{plot.note}</p>
        )}

        <div className="mt-4">
          <Panel
            title={`情节线（显示 ${plot.plotlines.length} / 共 ${plot.thread_count} 条 · ${plot.section_count} 个章节）`}
          >
            {plot.plotlines.length === 0 ? (
              <p className="text-xs text-text-muted">
                未识别到情节线：没有实体或名词信号达到 3 次的提及门槛。
              </p>
            ) : (
              <div className="overflow-x-auto">
                <table className="w-full text-xs">
                  <thead>
                    <tr className="text-text-muted border-b border-line">
                      <th className="text-left py-1.5 font-medium">线索</th>
                      <th className="text-left py-1.5 pl-4 font-medium">类型</th>
                      <th className="text-left py-1.5 pl-4 font-medium">形态</th>
                      <th className="text-right py-1.5 font-medium">提及</th>
                      <th className="text-right py-1.5 font-medium">覆盖</th>
                      <th className="text-right py-1.5 font-medium">最长中断</th>
                      <th className="text-left py-1.5 pl-4 font-medium">首次 → 末次</th>
                    </tr>
                  </thead>
                  <tbody>
                    {plot.plotlines.map((p) => (
                      <tr key={p.subject} className="border-b border-line/40">
                        <td className="py-1.5 font-medium">
                          {p.is_main && (
                            <span className="text-accent mr-1" title="主线">
                              ★
                            </span>
                          )}
                          {p.subject}
                        </td>
                        <td className="py-1.5 pl-4 text-text-muted">{p.kind}</td>
                        <td className="py-1.5 pl-4">
                          <span
                            className={`px-1.5 py-0.5 rounded border text-[11px] whitespace-nowrap ${plotShapeStyle(p.shape)}`}
                            title={`触达 ${p.sections_touched} 个章节 · 最长中断 ${p.longest_gap} 个章节`}
                          >
                            {plotShapeLabel(p.shape)}
                          </span>
                        </td>
                        <td className="py-1.5 text-right text-text-muted tabular-nums">{p.mentions}</td>
                        <td className="py-1.5 text-right text-text-muted tabular-nums">
                          {formatPercent(p.coverage)}
                        </td>
                        <td className="py-1.5 text-right text-text-muted tabular-nums">{p.longest_gap}</td>
                        <td className="py-1.5 pl-4 text-text-muted">
                          <span
                            className="inline-block truncate max-w-[140px] align-bottom"
                            title={p.first_section}
                          >
                            {p.first_section || "(root)"}
                          </span>
                          {" → "}
                          <span
                            className="inline-block truncate max-w-[140px] align-bottom"
                            title={p.last_section}
                          >
                            {p.last_section || "(root)"}
                          </span>
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
          <Panel title={`转折点（${plot.turning_points.length}）`}>
            {plot.turning_points.length === 0 ? (
              <p className="text-xs text-text-muted">章节之间的活跃线索没有明显变化。</p>
            ) : (
              <div className="overflow-x-auto">
                <table className="w-full text-xs">
                  <thead>
                    <tr className="text-text-muted border-b border-line">
                      <th className="text-left py-1.5 font-medium">章节</th>
                      <th className="text-right py-1.5 font-medium">变化</th>
                      <th className="text-left py-1.5 pl-4 font-medium">登场</th>
                      <th className="text-left py-1.5 pl-4 font-medium">退场</th>
                    </tr>
                  </thead>
                  <tbody>
                    {plot.turning_points.map((t) => (
                      <tr key={t.section} className="border-b border-line/40">
                        <td className="py-1.5 truncate max-w-[160px]" title={t.section}>
                          {t.section || "(root)"}
                        </td>
                        <td className="py-1.5 text-right text-text-muted tabular-nums">
                          {formatPercent(t.change)}
                        </td>
                        <td className="py-1.5 pl-4 text-good">
                          {t.appeared.join("、") || "—"}
                        </td>
                        <td className="py-1.5 pl-4 text-bad">{t.dropped.join("、") || "—"}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            )}
          </Panel>

          <Panel title="需要处理的线索">
            <div className="space-y-3 text-xs">
              <div>
                <p className="text-text-muted">铺垫未回收（{plot.unresolved_threads.length}）</p>
                <p className="mt-1 break-words">
                  {plot.unresolved_threads.length === 0
                    ? "无"
                    : plot.unresolved_threads.join("、")}
                </p>
              </div>
              <div>
                <p className="text-text-muted">结尾登场（{plot.late_arrivals.length}）</p>
                <p className="mt-1 break-words">
                  {plot.late_arrivals.length === 0 ? "无" : plot.late_arrivals.join("、")}
                </p>
              </div>
              <p className="text-text-muted pt-2 border-t border-line/60">
                连贯性从 100 起扣：铺垫未回收或结尾登场各 −8，中断满 3 个章节 −5，没有线索贯穿全文
                −15；少于 3 个章节不参与评分。
              </p>
            </div>
          </Panel>
        </div>
      </section>

      {/* Pacing analysis */}
      <section id="pacing" className={SECTION_ANCHOR}>
        <SectionTitle>节奏分析</SectionTitle>
        <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
          <StatCard label="节奏评分" value={`${formatScore(pacing.pace_score)} / 100`} />
          <StatCard label="张力波动" value={formatFixed(pacing.tension_stdev)} />
          <StatCard label="最长连续平淡" value={`${pacing.longest_flat_run} 段`} />
          <StatCard label="拖沓章节" value={formatInt(pacing.drag_sections.length)} />
        </div>

        <div className="mt-3 flex flex-wrap items-center gap-2">
          <span
            className={`px-2 py-0.5 rounded border text-xs ${pacingVerdictStyle(pacing.verdict)}`}
          >
            {pacingVerdictLabel(pacing.verdict)}
          </span>
          <span className="text-xs text-text-muted">
            张力 {formatScore(pacing.min_tension)}–{formatScore(pacing.max_tension)} · 每段均{" "}
            {formatInt(Math.round(pacing.mean_section_chars))} 字 · 句长变化{" "}
            {formatFixed(pacing.sentence_stdev)} 字
          </span>
        </div>

        <div className="mt-4">
          <Panel title={`节奏曲线（${pacing.segment_count} 段 · 张力 0-100）`}>
            {pacing.points.length === 0 ? (
              <p className="text-xs text-text-muted">未识别到节奏曲线：文档没有可切分的段落。</p>
            ) : (
              <LineChart
                points={pacing.points.map((p) => ({
                  label: p.section || "(root)",
                  value: p.tension / 100,
                }))}
              />
            )}
            {pacing.chunked && (
              <p className="text-xs text-text-muted mt-2">
                章节不足 3 个：曲线按 {pacing.segment_count} 个等长块切分，不参与评分。
              </p>
            )}
          </Panel>
        </div>

        <div className="mt-4">
          <Panel
            title={`逐段明细（显示 ${Math.min(pacing.points.length, 20)} / ${pacing.points.length} 段）`}
          >
            {pacing.points.length === 0 ? (
              <p className="text-xs text-text-muted">暂无数据</p>
            ) : (
              <div className="overflow-x-auto">
                <table className="w-full text-xs">
                  <thead>
                    <tr className="text-text-muted border-b border-line">
                      <th className="text-left py-1.5 font-medium">段落</th>
                      <th className="text-right py-1.5 font-medium">张力</th>
                      <th className="text-right py-1.5 font-medium">字数</th>
                      <th className="text-right py-1.5 font-medium">句子</th>
                      <th className="text-right py-1.5 font-medium">平均句长</th>
                      <th className="text-right py-1.5 font-medium">句长变化</th>
                      <th className="text-right py-1.5 font-medium">对话占比</th>
                    </tr>
                  </thead>
                  <tbody>
                    {pacing.points.slice(0, 20).map((p) => (
                      <tr key={p.section} className="border-b border-line/40">
                        <td className="py-1.5 truncate max-w-[220px]" title={p.section}>
                          {p.section || "(root)"}
                        </td>
                        <td className="py-1.5 text-right tabular-nums">{formatScore(p.tension)}</td>
                        <td className="py-1.5 text-right text-text-muted tabular-nums">
                          {formatInt(p.char_count)}
                        </td>
                        <td className="py-1.5 text-right text-text-muted tabular-nums">
                          {p.sentence_count}
                        </td>
                        <td className="py-1.5 text-right text-text-muted tabular-nums">
                          {formatFixed(p.avg_sentence_chars)}
                        </td>
                        <td className="py-1.5 text-right text-text-muted tabular-nums">
                          {formatFixed(p.sentence_stdev)}
                        </td>
                        <td className="py-1.5 text-right text-text-muted tabular-nums">
                          {formatPercent(p.dialogue_share)}
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
          <Panel title={`平淡段落（${pacing.flat_stretches.length}）`}>
            {pacing.flat_stretches.length === 0 ? (
              <p className="text-xs text-text-muted">没有连续 4 段以上的平淡区间。</p>
            ) : (
              <ul className="text-xs space-y-2">
                {pacing.flat_stretches.slice(0, 10).map((s) => (
                  <li key={`${s.from_section}→${s.to_section}`}>
                    <span className="font-medium">{s.from_section || "(root)"}</span>
                    <span className="text-text-muted"> → </span>
                    <span className="font-medium">{s.to_section || "(root)"}</span>
                    <span className="text-text-muted">
                      （{s.sections} 段 · 平均张力 {formatScore(s.mean_tension)} ·{" "}
                      {formatInt(s.char_count)} 字）
                    </span>
                  </li>
                ))}
              </ul>
            )}
          </Panel>

          <Panel title={`拖沓章节（${pacing.drag_sections.length}）`}>
            {pacing.drag_sections.length === 0 ? (
              <p className="text-xs text-text-muted">没有明显拖沓的章节。</p>
            ) : (
              <ul className="text-xs space-y-2">
                {pacing.drag_sections.slice(0, 10).map((d) => (
                  <li key={d.section}>
                    <span className="font-medium">{d.section || "(root)"}</span>
                    <span className="text-text-muted">
                      （{formatFixed(d.length_ratio)}× 均长 · {formatInt(d.char_count)} 字 · 张力{" "}
                      {formatScore(d.tension)}）
                    </span>
                  </li>
                ))}
              </ul>
            )}
            <p className="text-xs text-text-muted mt-3 pt-2 border-t border-line/60">
              节奏评分从 100 起扣：曲线几乎不动最多 −30，连续平淡满 4 段起每段 −6（最多 −24），拖沓章节每章
              −8（最多 −24），句长毫无变化最多 −18；少于 3 个章节不参与评分。
            </p>
          </Panel>
        </div>
      </section>

      {/* Conflict analysis */}
      <ConflictSection conflict={conflict} />

      {/* Theme analysis */}
      <ThemeSection theme={theme} segmentLabels={segmentLabels} />

      {/* Setting analysis */}
      <SettingSection setting={setting} />

      {/* Cliché analysis */}
      <ClicheSection cliche={cliche} />

      {/* Dialogue/narrative ratio */}
      <DialogueSection dialogue={dialogue} />

      {/* Explicit language: presence and strength, never a score */}
      <ExplicitSection explicit={explicit} />

      {/* Story structure: where the document's own signals place a shape, never a score */}
      <StructureSection structure={structure} />

      {/* Time line: what the text says about when, and whether its dates agree */}
      <TimelineSection timeline={timeline} />

      {/* Author voice: how the text is written, never a score */}
      <VoiceSection voice={voice} />

      {/* Final review checklist: the numbers above that still need a decision */}
      <ChecklistSection checklist={checklist} projectId={projectId} />
    </div>
  );
}

export default Analysis;
