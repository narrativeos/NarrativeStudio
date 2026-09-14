import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";

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

function Analysis() {
  const [stats, setStats] = useState<T0Stats | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function runAnalysis() {
    setLoading(true);
    setError(null);
    try {
      const filePath = prompt("Enter path to semantic_result.json:");
      if (!filePath) return;
      const result = await invoke<T0Stats>("analyze_t0", { filePath });
      setStats(result);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }

  return (
    <div>
      <div className="flex items-center justify-between mb-4">
        <h2 className="text-xl font-semibold">T0 Analysis</h2>
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

      {stats && (
        <div className="space-y-6">
          {/* Summary cards */}
          <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
            <StatCard label="Words" value={stats.word_count.toLocaleString()} />
            <StatCard label="Blocks" value={stats.block_count.toString()} />
            <StatCard label="Entities" value={stats.entity_counts.reduce((s, [, c]) => s + c, 0).toString()} />
            <StatCard label="Noun Signals" value={stats.noun_signal_count.toString()} />
          </div>

          {/* Top words */}
          <section>
            <h3 className="text-sm font-medium text-text-muted mb-2">Top Words</h3>
            <div className="flex flex-wrap gap-2">
              {stats.top_words.slice(0, 20).map(([word, count]) => (
                <span key={word} className="px-2 py-1 bg-surface-alt rounded text-xs">
                  {word} <span className="text-text-muted">({count})</span>
                </span>
              ))}
            </div>
          </section>

          {/* POS distribution */}
          <section>
            <h3 className="text-sm font-medium text-text-muted mb-2">POS Distribution</h3>
            <div className="flex flex-wrap gap-2">
              {stats.pos_distribution.map(([pos, count]) => (
                <span key={pos} className="px-2 py-1 bg-surface-alt rounded text-xs">
                  {pos} <span className="text-text-muted">({count})</span>
                </span>
              ))}
            </div>
          </section>

          {/* Entity counts */}
          {stats.entity_counts.length > 0 && (
            <section>
              <h3 className="text-sm font-medium text-text-muted mb-2">Entities by Category</h3>
              <div className="flex flex-wrap gap-2">
                {stats.entity_counts.map(([cat, count]) => (
                  <span key={cat} className="px-2 py-1 bg-surface-alt rounded text-xs">
                    {cat} <span className="text-text-muted">({count})</span>
                  </span>
                ))}
              </div>
            </section>
          )}
        </div>
      )}

      {!stats && !loading && !error && (
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