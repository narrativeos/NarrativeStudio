import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { entityTotal, formatFixed, formatInt, formatPercent, wordsMeasured } from "../lib/format";
import { ScalesIcon } from "../components/Icon";
import type { ProjectAnalysis } from "../types";

interface CompareProps {
  selection: Set<string>;
}

function Compare({ selection }: CompareProps) {
  // Results and errors are tagged with the selection signature they belong to.
  // That removes the need for a synchronous reset inside the effect (which
  // caused cascading renders) and makes it impossible for a stale response
  // from a previous selection to render under the current one.
  const [result, setResult] = useState<{ key: string; data: ProjectAnalysis[] } | null>(
    null,
  );
  const [failure, setFailure] = useState<{ key: string; message: string } | null>(null);

  const selectedIds = useMemo(() => Array.from(selection), [selection]);
  const depKey = selectedIds.join(",");
  const enoughSelected = selectedIds.length >= 2;

  useEffect(() => {
    if (!enoughSelected) return;
    const projectIds = depKey.split(",");
    let cancelled = false;
    async function run() {
      try {
        const data = await invoke<ProjectAnalysis[]>("analyze_projects", { projectIds });
        if (!cancelled) setResult({ key: depKey, data });
      } catch (e) {
        if (!cancelled) setFailure({ key: depKey, message: String(e) });
      }
    }
    run();
    return () => {
      cancelled = true;
    };
  }, [depKey, enoughSelected]);

  const data = result?.key === depKey ? result.data : null;
  const error = failure?.key === depKey ? failure.message : null;
  const loading =
    enoughSelected && result?.key !== depKey && failure?.key !== depKey;

  const metrics: { label: string; value: (a: ProjectAnalysis) => string }[] = [
    // A text-only project has no segmentation and no entities, so these two are
    // gaps in the input rather than measurements; 0 would compare as a count.
    {
      label: "词数",
      value: (a) => (wordsMeasured(a.t0) ? formatInt(a.t0.word_count) : "未测量"),
    },
    { label: "文本块", value: (a) => formatInt(a.t0.block_count) },
    { label: "段落数", value: (a) => formatInt(a.t1.section_count) },
    {
      label: "实体",
      value: (a) => (a.setting.places_measured ? formatInt(entityTotal(a.t0)) : "未测量"),
    },
    { label: "名词信号", value: (a) => formatInt(a.t0.noun_signal_count) },
    { label: "平均句长", value: (a) => formatFixed(a.t0.avg_sentence_length) },
    // Author voice is what carries across projects: the same habits whatever the
    // story is about. None of these is a ranking.
    { label: "句长离散", value: (a) => formatFixed(a.voice.sentence_cv, 2) },
    { label: "每句子句数", value: (a) => formatFixed(a.voice.clauses_per_sentence, 2) },
    {
      label: "第一人称占比",
      value: (a) => formatPercent(a.voice.pronouns.first_share),
    },
  ];

  const topWords = useMemo(() => {
    if (!data) return [];
    const map = new Map<string, Map<string, number>>();
    for (const a of data) {
      for (const [word, count] of a.t0.top_words) {
        if (!map.has(word)) map.set(word, new Map());
        map.get(word)!.set(a.project_id, count);
      }
    }
    return Array.from(map.entries())
      .map(([word, perProject]) => ({
        word,
        perProject,
        max: Math.max(...Array.from(perProject.values())),
      }))
      .sort((a, b) => b.max - a.max)
      .slice(0, 20);
  }, [data]);

  return (
    <div>
      <div className="mb-4">
        <h2 className="flex items-center gap-2 text-xl font-semibold">
          <ScalesIcon className="h-5 w-5" />
          多项目对比
        </h2>
        <p className="text-xs text-text-muted mt-1">
          已选 {selection.size} 个项目
          {selection.size < 2 ? "（请从左侧勾选至少 2 个）" : ""}
        </p>
      </div>

      {selection.size < 2 ? (
        <div className="text-center py-12 text-text-muted">
          <p className="mb-4 flex justify-center">
            <ScalesIcon className="h-12 w-12" />
          </p>
          <p>从左侧勾选至少 2 个项目进行对比。</p>
        </div>
      ) : (
        <>
          {loading && <div className="text-text-muted mb-4">分析中…</div>}
          {error && (
            <div className="text-bad mb-4">
              <p>对比失败：</p>
              <pre className="text-xs mt-2 whitespace-pre-wrap">{error}</pre>
            </div>
          )}
          {data && data.length >= 2 && (
            <div className="space-y-6">
              <div className="overflow-x-auto">
                <table className="w-full text-sm border-collapse">
                  <thead>
                    <tr>
                      <th className="text-left px-3 py-2 text-text-muted font-medium border-b border-line">
                        指标
                      </th>
                      {data.map((a) => (
                        <th
                          key={a.project_id}
                          className="text-right px-3 py-2 text-accent font-medium border-b border-line"
                        >
                          {a.name}
                        </th>
                      ))}
                    </tr>
                  </thead>
                  <tbody>
                    {metrics.map((m) => (
                      <tr key={m.label}>
                        <td className="px-3 py-2 text-text-muted border-b border-line/50">
                          {m.label}
                        </td>
                        {data.map((a) => (
                          <td
                            key={a.project_id}
                            className="px-3 py-2 text-right border-b border-line/50"
                          >
                            {m.value(a)}
                          </td>
                        ))}
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>

              <section>
                <h3 className="text-sm font-medium text-text-muted mb-2">
                  Top Words（跨项目）
                </h3>
                <div className="space-y-1">
                  {topWords.map(({ word, perProject }) => (
                    <div
                      key={word}
                      className="flex items-center gap-3 text-xs px-3 py-1.5 bg-surface-alt rounded"
                    >
                      <span className="w-32 shrink-0 truncate">{word}</span>
                      {data.map((a) => (
                        <span key={a.project_id} className="text-text-muted">
                          {a.name}: {perProject.get(a.project_id) ?? 0}
                        </span>
                      ))}
                    </div>
                  ))}
                </div>
              </section>
            </div>
          )}
        </>
      )}
    </div>
  );
}

export default Compare;
