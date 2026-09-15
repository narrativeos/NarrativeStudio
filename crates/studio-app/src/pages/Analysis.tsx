import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

interface T0Stats {
  word_count: number;
  char_count: number;
  block_count: number;
  top_words: [string, number][];
  pos_distribution: [string, number][];
  entity_counts: [string, number][];
  noun_signal_count: number;
  avg_block_length: number;
  avg_sentence_length: number;
}

interface SectionInfo {
  path: string;
  block_count: number;
  char_count: number;
}

interface T1Stats {
  sections: SectionInfo[];
  section_count: number;
  title_blocks: number;
  paragraph_blocks: number;
  list_blocks: number;
  block_type_distribution: [string, number][];
  longest_section: string | null;
  shortest_section: string | null;
}

function Analysis() {
  const [t0, setT0] = useState<T0Stats | null>(null);
  const [t1, setT1] = useState<T1Stats | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function runAnalysis() {
    setLoading(true);
    setError(null);
    try {
      const filePath = await open({
        title: "选择 semantic_result.json 文件",
        filters: [{ name: "JSON", extensions: ["json"] }],
        multiple: false,
      });
      if (!filePath || typeof filePath !== "string") return;
      const [t0Result, t1Result] = await Promise.all([
        invoke<T0Stats>("analyze_t0", { filePath }),
        invoke<T1Stats>("analyze_t1", { filePath }),
      ]);
      setT0(t0Result);
      setT1(t1Result);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }

  return (
    <div>
      <div className="flex items-center justify-between mb-4">
        <h2 className="text-xl font-semibold">Analysis</h2>
        <button
          onClick={runAnalysis}
          disabled={loading}
          className="px-4 py-2 bg-accent/20 text-accent rounded-md hover:bg-accent/30 transition-colors disabled:opacity-50"
        >
          {loading ? "Analyzing..." : "Run Analysis"}
        </button>
      </div>

      {error && (
        <div className="text-red-400 mb-4">
          <p>Analysis failed:</p>
          <pre className="text-xs mt-2">{error}</pre>
        </div>
      )}

      {t0 && t1 && (
        <div className="space-y-6">
          {/* Summary cards */}
          <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
            <StatCard label="Words" value={t0.word_count.toLocaleString()} />
            <StatCard label="Blocks" value={t0.block_count.toString()} />
            <StatCard label="Sections" value={t1.section_count.toString()} />
            <StatCard label="Entities" value={t0.entity_counts.reduce((s, [, c]) => s + c, 0).toString()} />
          </div>

          {/* T0: Top words */}
          <section>
            <h3 className="text-sm font-medium text-text-muted mb-2">Top Words</h3>
            <div className="flex flex-wrap gap-2">
              {t0.top_words.slice(0, 20).map(([word, count]) => (
                <span key={word} className="px-2 py-1 bg-surface-alt rounded text-xs">
                  {word} <span className="text-text-muted">({count})</span>
                </span>
              ))}
            </div>
          </section>

          {/* T0: POS distribution */}
          <section>
            <h3 className="text-sm font-medium text-text-muted mb-2">POS Distribution</h3>
            <div className="flex flex-wrap gap-2">
              {t0.pos_distribution.map(([pos, count]) => (
                <span key={pos} className="px-2 py-1 bg-surface-alt rounded text-xs">
                  {pos} <span className="text-text-muted">({count})</span>
                </span>
              ))}
            </div>
          </section>

          {/* T1: Block type distribution */}
          <section>
            <h3 className="text-sm font-medium text-text-muted mb-2">Block Types</h3>
            <div className="flex flex-wrap gap-2">
              {t1.block_type_distribution.map(([bt, count]) => (
                <span key={bt} className="px-2 py-1 bg-surface-alt rounded text-xs">
                  {bt} <span className="text-text-muted">({count})</span>
                </span>
              ))}
            </div>
          </section>

          {/* T1: Sections */}
          {t1.sections.length > 0 && (
            <section>
              <h3 className="text-sm font-medium text-text-muted mb-2">Sections</h3>
              <div className="space-y-1">
                {t1.sections.map((s) => (
                  <div key={s.path} className="flex justify-between text-xs px-3 py-2 bg-surface-alt rounded">
                    <span>{s.path || "(root)"}</span>
                    <span className="text-text-muted">{s.block_count} blocks, {s.char_count} chars</span>
                  </div>
                ))}
              </div>
            </section>
          )}
        </div>
      )}

      {!t0 && !loading && !error && (
        <div className="text-center py-12 text-text-muted">
          <p className="text-4xl mb-4">📊</p>
          <p>Click "Run Analysis" to analyze a TraceView semantic_result.json file.</p>
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