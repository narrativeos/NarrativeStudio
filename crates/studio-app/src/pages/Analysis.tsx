import { useEffect, useState, type ReactNode } from "react";
import { invoke } from "@tauri-apps/api/core";
import BarList from "../components/charts/BarList";
import type { ProjectAnalysis } from "../types";

interface AnalysisProps {
  projectId: string;
  onBack: () => void;
}

/**
 * Single-project analysis view. Runs T0 (statistical) + T1 (structural)
 * analysis for the selected project and presents the full statistical
 * portrait, grouped by dimension. Deeper dimensions (radar scoring,
 * narrative arc, pacing, ...) are M2/M3 and render here as they land.
 */
function Analysis({ projectId, onBack }: AnalysisProps) {
  const [data, setData] = useState<ProjectAnalysis | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    async function run() {
      setLoading(true);
      setError(null);
      setData(null);
      try {
        const result = await invoke<ProjectAnalysis>("analyze_project", { projectId });
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
  }, [projectId]);

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
  const entityTotal = t0.entity_counts.reduce((sum, [, c]) => sum + c, 0);
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
            {t0.block_count.toLocaleString()} 块 · {t1.section_count.toLocaleString()} 章节 ·{" "}
            {t0.char_count.toLocaleString()} 字
          </p>
        </div>
      </div>

      {/* Overview metrics */}
      <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-6 gap-3">
        <StatCard label="字数" value={t0.word_count.toLocaleString()} />
        <StatCard label="文本块" value={t0.block_count.toLocaleString()} />
        <StatCard label="章节" value={t1.section_count.toLocaleString()} />
        <StatCard label="实体" value={entityTotal.toLocaleString()} />
        <StatCard label="名词信号" value={t0.noun_signal_count.toLocaleString()} />
        <StatCard label="平均句长" value={t0.avg_sentence_length.toFixed(1)} />
      </div>

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
