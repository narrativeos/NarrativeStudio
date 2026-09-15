import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { ProjectAnalysis } from "../types";

interface AnalysisProps {
  projectId: string;
  onBack: () => void;
}

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

  return (
    <div>
      <div className="flex items-center gap-3 mb-4">
        <button
          onClick={onBack}
          className="px-2 py-1 text-lg leading-none text-text-muted hover:text-text hover:bg-gray-700/50 rounded-md transition-colors"
          title="返回项目列表"
        >
          ←
        </button>
        <div>
          <h2 className="text-xl font-semibold">{data?.name ?? "Project"}</h2>
          {data && (
            <p className="text-xs text-text-muted mt-0.5">
              {data.t0.block_count} blocks · {data.t1.section_count} sections ·{" "}
              {data.t0.word_count.toLocaleString()} words
            </p>
          )}
        </div>
      </div>

      {loading && <div className="text-text-muted">Analyzing…</div>}

      {error && (
        <div className="text-red-400 mb-4">
          <p>Analysis failed:</p>
          <pre className="text-xs mt-2">{error}</pre>
        </div>
      )}

      {data && (
        <div className="space-y-6">
          <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
            <StatCard label="Words" value={data.t0.word_count.toLocaleString()} />
            <StatCard label="Blocks" value={data.t0.block_count.toString()} />
            <StatCard label="Sections" value={data.t1.section_count.toString()} />
            <StatCard
              label="Entities"
              value={data.t0.entity_counts.reduce((s, [, c]) => s + c, 0).toString()}
            />
          </div>

          <section>
            <h3 className="text-sm font-medium text-text-muted mb-2">Top Words</h3>
            <div className="flex flex-wrap gap-2">
              {data.t0.top_words.slice(0, 20).map(([word, count]) => (
                <span key={word} className="px-2 py-1 bg-surface-alt rounded text-xs">
                  {word} <span className="text-text-muted">({count})</span>
                </span>
              ))}
            </div>
          </section>

          <section>
            <h3 className="text-sm font-medium text-text-muted mb-2">POS Distribution</h3>
            <div className="flex flex-wrap gap-2">
              {data.t0.pos_distribution.map(([pos, count]) => (
                <span key={pos} className="px-2 py-1 bg-surface-alt rounded text-xs">
                  {pos} <span className="text-text-muted">({count})</span>
                </span>
              ))}
            </div>
          </section>

          <section>
            <h3 className="text-sm font-medium text-text-muted mb-2">Block Types</h3>
            <div className="flex flex-wrap gap-2">
              {data.t1.block_type_distribution.map(([bt, count]) => (
                <span key={bt} className="px-2 py-1 bg-surface-alt rounded text-xs">
                  {bt} <span className="text-text-muted">({count})</span>
                </span>
              ))}
            </div>
          </section>

          {data.t1.sections.length > 0 && (
            <section>
              <h3 className="text-sm font-medium text-text-muted mb-2">Sections</h3>
              <div className="space-y-1">
                {data.t1.sections.map((s) => (
                  <div
                    key={s.path}
                    className="flex justify-between text-xs px-3 py-2 bg-surface-alt rounded"
                  >
                    <span>{s.path || "(root)"}</span>
                    <span className="text-text-muted">
                      {s.block_count} blocks, {s.char_count} chars
                    </span>
                  </div>
                ))}
              </div>
            </section>
          )}
        </div>
      )}
    </div>
  );
}

function StatCard({ label, value }: { label: string; value: string }) {
  return (
    <div className="p-4 bg-surface-alt rounded-lg border border-gray-700">
      <p className="text-xs text-text-muted">{label}</p>
      <p className="text-2xl font-semibold mt-1">{value}</p>
    </div>
  );
}

export default Analysis;
