import { Panel, SectionTitle, StatCard } from "../Cards";
import {
  formatFixed,
  formatInt,
  formatPercent,
  structureActLabel,
  structureBeatLabel,
  structureFitLabel,
  structureFitStyle,
  structureIssueLabel,
  structureIssueStyle,
  structureLandmarkLabel,
  structureLandmarkStyle,
  structureModelLabel,
  structureStatusLabel,
  structureStatusStyle,
} from "../../lib/format";
import type { StructureStats } from "../../types";

interface StructureSectionProps {
  structure: StructureStats;
}

/** Ticks for the landmark strip: the two tension extremes share a hue because they are
 *  one measurement read in opposite directions. */
const LANDMARK_TICK_COLORS: Record<string, string> = {
  opening: "bg-text-muted",
  turn: "bg-sky-400",
  peak: "bg-amber-400",
  trough: "bg-sky-400",
  climax: "bg-rose-400",
};

/**
 * 故事结构 section: where the document's own signals place a shape, and how the classic
 * shapes sit against that.
 *
 * There is deliberately no score here. Fitting three-act shape is not a virtue and
 * refusing it is not a flaw, so the section describes: the landmarks it worked from
 * (thread changes and the tension curve, both proxy signals), the document's own
 * three-way split cut at its two strongest turns, and every beat of every model with
 * where it landed or how far outside the window the nearest signal was. A beat the
 * signals did not show keeps its row marked 未见 — the absence is a limit on the reading,
 * not a claim that the story lacks the beat, and the note at the bottom says so.
 */
function StructureSection({ structure }: StructureSectionProps) {
  return (
    <section>
      <SectionTitle>故事结构</SectionTitle>

      <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
        <StatCard label="地标" value={formatInt(structure.landmarks.length)} />
        <StatCard label="转折点" value={formatInt(structure.turn_count)} />
        <StatCard label="曲线段落" value={formatInt(structure.segment_count)} />
        <StatCard label="可见正文" value={formatInt(structure.char_count)} />
      </div>

      <p className="mt-2 text-xs text-text-muted">
        本维度不给出评分：契合三幕不是优点，有意拒绝这个形状也不是缺点。地标来自线程变化与张力曲线，两者都是代理信号。
      </p>

      {!structure.readable && (
        <Panel title="本次不做模型对照">
          <p className="text-xs text-text-muted">{structure.note}</p>
          <p className="text-xs text-text-muted mt-2">
            「未见」只说明这些信号没有显示这一环，不等于故事缺少它；节拍区间（如 25%/50%/25%）是写作手册里的约定，不是测量。
          </p>
        </Panel>
      )}

      {structure.readable && (
        <>
          {structure.landmarks.length > 0 && (
            <div className="mt-4">
              <Panel title={`地标位置（${structure.landmarks.length} 个）`}>
                <div className="relative h-8 rounded bg-gray-800/60">
                  {structure.landmarks.map((l, idx) => (
                    <span
                      key={`${l.kind}-${l.section}-${idx}`}
                      className={`absolute top-0 h-full w-0.5 ${
                        LANDMARK_TICK_COLORS[l.kind] ?? "bg-text-muted"
                      }`}
                      style={{ left: `${Math.min(99.5, Math.max(0, l.position * 100))}%` }}
                      title={`${formatPercent(l.position)} · ${structureLandmarkLabel(l.kind)} · ${
                        l.section || "(root)"
                      }`}
                    />
                  ))}
                </div>
                <div className="mt-1 flex justify-between text-xs text-text-muted">
                  <span>0%</span>
                  <span>100%</span>
                </div>
                {structure.chunked && (
                  <p className="text-xs text-text-muted mt-2">
                    章节不足 3 个：位置落在按字数等分的 {formatInt(structure.segment_count)}{" "}
                    个块上，是算术结果，不是作者自己的分段。
                  </p>
                )}
              </Panel>
            </div>
          )}

          <div className="mt-4">
            <Panel title={`地标（${structure.landmarks.length} 个）`}>
              {structure.landmarks.length === 0 ? (
                <p className="text-xs text-text-muted">
                  线程变化与张力曲线都没有指出任何地标。
                </p>
              ) : (
                <div className="overflow-x-auto">
                  <table className="w-full text-xs">
                    <thead className="text-text-muted border-b border-gray-700/60">
                      <tr>
                        <th className="text-right py-1.5 font-medium">位置</th>
                        <th className="text-left py-1.5 font-medium">段落</th>
                        <th className="text-left py-1.5 font-medium">类型</th>
                        <th className="text-right py-1.5 font-medium">强度</th>
                        <th className="text-left py-1.5 font-medium">依据</th>
                      </tr>
                    </thead>
                    <tbody>
                      {structure.landmarks.map((l, idx) => (
                        <tr
                          key={`${l.kind}-${l.section}-${idx}`}
                          className="border-b border-gray-700/40"
                        >
                          <td className="py-1.5 text-right tabular-nums">
                            {formatPercent(l.position)}
                          </td>
                          <td className="py-1.5">{l.section || "(root)"}</td>
                          <td className="py-1.5">
                            <span
                              className={`px-1.5 py-0.5 rounded border text-xs ${structureLandmarkStyle(
                                l.kind,
                              )}`}
                            >
                              {structureLandmarkLabel(l.kind)}
                            </span>
                          </td>
                          <td className="py-1.5 text-right text-text-muted tabular-nums">
                            {formatFixed(l.strength)}
                          </td>
                          <td className="py-1.5 text-text-muted">{l.detail}</td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              )}
            </Panel>
          </div>
          <div className="mt-4">
            <Panel title="幕的切分（按本文自己的转折点）">
              {structure.acts.length === 0 ? (
                <p className="text-xs text-text-muted">
                  可辨的转折点不足两个，本文没有可切的三幕。
                </p>
              ) : (
                <div className="overflow-x-auto">
                  <table className="w-full text-xs">
                    <thead className="text-text-muted border-b border-gray-700/60">
                      <tr>
                        <th className="text-left py-1.5 font-medium">幕</th>
                        <th className="text-left py-1.5 font-medium">起</th>
                        <th className="text-left py-1.5 font-medium">止</th>
                        <th className="text-right py-1.5 font-medium">字数</th>
                        <th className="text-right py-1.5 font-medium">占比</th>
                        <th className="text-right py-1.5 font-medium">张力（低/均/高）</th>
                      </tr>
                    </thead>
                    <tbody>
                      {structure.acts.map((a) => (
                        <tr key={a.name} className="border-b border-gray-700/40">
                          <td className="py-1.5 font-medium">{structureActLabel(a.name)}</td>
                          <td className="py-1.5 text-text-muted">{a.from_section || "(root)"}</td>
                          <td className="py-1.5 text-text-muted">{a.to_section || "(root)"}</td>
                          <td className="py-1.5 text-right tabular-nums">
                            {formatInt(a.char_count)}
                          </td>
                          <td className="py-1.5 text-right text-text-muted tabular-nums">
                            {formatPercent(a.share)}
                          </td>
                          <td className="py-1.5 text-right tabular-nums">
                            {`${formatFixed(a.min_tension)} / ${formatFixed(a.mean_tension)} / ${formatFixed(
                              a.max_tension,
                            )}`}
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
            <Panel title={`经典模型对照（${structure.models.length} 个模型）`}>
              {structure.models.length === 0 ? (
                <p className="text-xs text-text-muted">没有可对照的模型。</p>
              ) : (
                <ul className="text-xs space-y-1.5">
                  {structure.models.map((m) => (
                    <li key={m.model}>
                      <span
                        className={`px-2 py-0.5 rounded border text-xs ${structureFitStyle(m.fit)}`}
                      >
                        {structureModelLabel(m.model)}：{structureFitLabel(m.fit)}
                      </span>
                      <span className="text-text-muted">
                        （命中 {formatInt(m.found)} · 错位 {formatInt(m.displaced)} · 未见{" "}
                        {formatInt(m.missing)}）
                      </span>
                    </li>
                  ))}
                </ul>
              )}
            </Panel>
          </div>

          {structure.models.map((m) => (
            <div className="mt-4" key={`beats-${m.model}`}>
              <Panel title={`${structureModelLabel(m.model)}：节拍落点`}>
                <div className="overflow-x-auto">
                  <table className="w-full text-xs">
                    <thead className="text-text-muted border-b border-gray-700/60">
                      <tr>
                        <th className="text-left py-1.5 font-medium">节拍</th>
                        <th className="text-right py-1.5 font-medium">期望区间</th>
                        <th className="text-left py-1.5 font-medium">判定</th>
                        <th className="text-left py-1.5 font-medium">落点</th>
                        <th className="text-right py-1.5 font-medium">偏差</th>
                        <th className="text-left py-1.5 font-medium">依据</th>
                      </tr>
                    </thead>
                    <tbody>
                      {m.beats.map((b) => (
                        <tr key={`${m.model}-${b.beat}`} className="border-b border-gray-700/40">
                          <td className="py-1.5 font-medium">{structureBeatLabel(b.beat)}</td>
                          <td className="py-1.5 text-right text-text-muted tabular-nums">
                            {`${formatPercent(b.window[0])}–${formatPercent(b.window[1])}`}
                          </td>
                          <td className="py-1.5">
                            <span
                              className={`px-1.5 py-0.5 rounded border text-xs ${structureStatusStyle(
                                b.status,
                              )}`}
                            >
                              {structureStatusLabel(b.status)}
                            </span>
                          </td>
                          <td className="py-1.5">
                            {b.section ? (
                              <>
                                {b.section || "(root)"}
                                <span className="text-text-muted">（{formatPercent(b.position)}）</span>
                              </>
                            ) : (
                              <span className="text-text-muted">—</span>
                            )}
                          </td>
                          <td className="py-1.5 text-right text-text-muted tabular-nums">
                            {b.offset > 0 ? formatPercent(b.offset) : "—"}
                          </td>
                          <td className="py-1.5 text-text-muted">{b.evidence || "—"}</td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              </Panel>
            </div>
          ))}

          <div className="mt-4">
            <Panel title={`结构提示（${structure.issues.length}）`}>
              {structure.issues.length === 0 ? (
                <p className="text-xs text-text-muted">
                  曲线的形状没有提出需要留意的地方。这不表示结构好，只表示这些信号没有报出什么。
                </p>
              ) : (
                <ul className="text-xs space-y-2">
                  {structure.issues.map((i, idx) => (
                    <li key={`${i.kind}-${i.section}-${idx}`}>
                      <span
                        className={`px-2 py-0.5 rounded border text-xs ${structureIssueStyle(i.kind)}`}
                      >
                        {structureIssueLabel(i.kind)}
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

          <p className="mt-2 text-xs text-text-muted">
            「未见」只说明线程变化与张力曲线没有显示这一环，不等于故事缺少它；节拍区间（如 25%/50%/25%）是写作手册里的约定，不是测量。
          </p>
        </>
      )}
    </section>
  );
}

export default StructureSection;
