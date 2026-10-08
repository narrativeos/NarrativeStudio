import { Panel, SectionTitle, StatCard } from "../Cards";
import {
  formatInt,
  formatPercent,
  timelineIssueLabel,
  timelineIssueStyle,
  timelineKindLabel,
  timelinePrecisionLabel,
  timelineSignedSpan,
  timelineSpanLabel,
} from "../../lib/format";
import type { TimelineStats } from "../../types";

interface TimelineSectionProps {
  timeline: TimelineStats;
}

/** Ticks for the marker strip: the two kinds that carry a calendar date share a hue,
 *  and the two that do not are kept visibly apart from them. */
const MARKER_TICK_COLORS: Record<string, string> = {
  absolute: "bg-sky-400",
  partial: "bg-sky-400/50",
  relative: "bg-amber-400",
  vague: "bg-text-muted",
};

/** How many rows of each table the page shows before saying what was left out. */
const MAX_ROWS = 20;

/**
 * 时间线 section: where the text says when, and whether the dates it gives agree.
 *
 * There is deliberately no score here. A story that never writes a date is not keeping
 * time badly, so the section describes: the four kinds of time expression it found and
 * where they sit on the same axis as the pacing and conflict curves, the dates it could
 * place on a calendar in the order they appear, the steps between those dates that go
 * backwards, the largest forward jumps, and the stretches that carry no marker at all.
 *
 * The distinctions matter more than the totals. Relative and vague expressions (三天后,
 * 春天) move the story but are anchored to the narration rather than to a calendar, so
 * they are counted and never ordered; a bare year is too coarse to order either. A
 * backstep is offered as something to look at — flashback is legitimate and nothing here
 * can tell the two apart — and an unmarked stretch is not a claim that time stood still.
 */
function TimelineSection({ timeline }: TimelineSectionProps) {
  const dated = timeline.markers.filter((m) => m.resolved !== "");
  const notes = timeline.issues.filter((i) => i.kind !== "backstep" && i.kind !== "long_jump");
  const unreadableDates = timeline.entity_date_count - timeline.absolute_count;

  return (
    <section>
      <SectionTitle>时间线</SectionTitle>

      <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
        <StatCard label="时间标记" value={formatInt(timeline.markers.length)} />
        <StatCard label="绝对日期" value={formatInt(timeline.absolute_count)} />
        <StatCard label="可排序日期" value={formatInt(dated.length)} />
        <StatCard label="可见正文" value={formatInt(timeline.char_count)} />
      </div>

      <p className="mt-2 text-xs text-text-muted">
        本维度不给出评分：不写日期的故事不比写满日期的故事差。相对与模糊表达锚在叙述当下，只统计不参与排序。
      </p>

      {!timeline.readable && (
        <Panel title="本次不做时间排序">
          <p className="text-xs text-text-muted">{timeline.note}</p>
          {timeline.entity_date_count > 0 && (
            <p className="text-xs text-text-muted mt-2">
              导入的实体标注里有 {formatInt(timeline.entity_date_count)} 处日期，文本扫描没有读取它们；
              两处数字相差多少，说明有多少日期是用本文不认的写法写的（年号、农历、外文日期都可能）。
            </p>
          )}
          <p className="text-xs text-text-muted mt-2">
            识别范围是阿拉伯数字与逐字写出的中文年份，加上 天/周/月/年 的相对表达和常见的季节、时段词。
          </p>
        </Panel>
      )}

      {timeline.readable && (
        <>
          {timeline.markers.length > 0 && (
            <div className="mt-4">
              <Panel title={`标记位置（${timeline.markers.length} 处）`}>
                <div className="relative h-8 rounded bg-gray-800/60">
                  {timeline.markers.map((m, idx) => (
                    <span
                      key={`${m.kind}-${m.text}-${idx}`}
                      className={`absolute top-0 h-full w-0.5 ${
                        MARKER_TICK_COLORS[m.kind] ?? "bg-text-muted"
                      }`}
                      style={{ left: `${Math.min(99.5, Math.max(0, m.position * 100))}%` }}
                      title={`${formatPercent(m.position)} · ${timelineKindLabel(m.kind)} · ${
                        m.text
                      } · ${m.section || "(root)"}`}
                    />
                  ))}
                </div>
                <div className="mt-1 flex justify-between text-[10px] text-text-muted">
                  <span>0%</span>
                  <span className="flex items-center gap-3">
                    {(
                      [
                        ["absolute", "绝对日期"],
                        ["partial", "缺年的日期"],
                        ["relative", "相对表达"],
                        ["vague", "模糊表达"],
                      ] as const
                    ).map(([kind, label]) => (
                      <span key={kind} className="flex items-center gap-1">
                        <i
                          className={`inline-block w-2 h-2 rounded-sm ${MARKER_TICK_COLORS[kind]}`}
                        />
                        {label}
                      </span>
                    ))}
                  </span>
                  <span>100%</span>
                </div>
              </Panel>
            </div>
          )}

          {dated.length >= 2 && (
            <div className="mt-4">
              <Panel title={`日期排序（${dated.length} 个）`}>
                <div className="mb-2 text-xs text-text-muted">
                  {timeline.first_date} → {timeline.last_date}：跨度 {timelineSpanLabel(timeline.span_days)}
                </div>
                <table className="w-full text-xs">
                  <thead className="text-text-muted">
                    <tr>
                      <th className="text-right py-1.5 font-medium">位置</th>
                      <th className="text-left py-1.5 font-medium">段落</th>
                      <th className="text-left py-1.5 font-medium">日期</th>
                      <th className="text-left py-1.5 font-medium">粒度</th>
                    </tr>
                  </thead>
                  <tbody>
                    {dated.slice(0, MAX_ROWS).map((m, idx) => (
                      <tr key={`${m.resolved}-${idx}`} className="border-b border-gray-700/40">
                        <td className="py-1.5 text-right tabular-nums">
                          {formatPercent(m.position)}
                        </td>
                        <td className="py-1.5">{m.section || "(root)"}</td>
                        <td className="py-1.5 font-medium tabular-nums">{m.resolved}</td>
                        <td className="py-1.5 text-text-muted">
                          {timelinePrecisionLabel(m.precision)}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
                {dated.length > MAX_ROWS && (
                  <p className="mt-2 text-xs text-text-muted">
                    另有 {formatInt(dated.length - MAX_ROWS)} 个日期未列出。
                  </p>
                )}
              </Panel>
            </div>
          )}

          {timeline.backsteps.length > 0 && (
            <div className="mt-4">
              <Panel title={`时间倒回（${timeline.backsteps.length} 处）`}>
                <table className="w-full text-xs">
                  <thead className="text-text-muted">
                    <tr>
                      <th className="text-left py-1.5 font-medium">前一处</th>
                      <th className="text-left py-1.5 font-medium">日期</th>
                      <th className="text-left py-1.5 font-medium">后一处</th>
                      <th className="text-left py-1.5 font-medium">日期</th>
                      <th className="text-right py-1.5 font-medium">相差</th>
                    </tr>
                  </thead>
                  <tbody>
                    {timeline.backsteps.map((s, idx) => (
                      <tr
                        key={`${s.from_date}-${s.to_date}-${idx}`}
                        className="border-b border-gray-700/40"
                      >
                        <td className="py-1.5">{s.from_section || "(root)"}</td>
                        <td className="py-1.5 tabular-nums">{s.from_date}</td>
                        <td className="py-1.5">{s.to_section || "(root)"}</td>
                        <td className="py-1.5 tabular-nums">{s.to_date}</td>
                        <td className="py-1.5 text-right text-rose-400 tabular-nums">
                          早 {timelineSpanLabel(s.days)}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
                <p className="mt-2 text-xs text-text-muted">
                  日历上倒退不等于写错：倒叙是合法手法，文本分不出这两种。
                </p>
              </Panel>
            </div>
          )}

          {timeline.jumps.length > 0 && (
            <div className="mt-4">
              <Panel title={`大跳跃（${timeline.jumps.length} 处）`}>
                <table className="w-full text-xs">
                  <thead className="text-text-muted">
                    <tr>
                      <th className="text-left py-1.5 font-medium">从</th>
                      <th className="text-left py-1.5 font-medium">日期</th>
                      <th className="text-left py-1.5 font-medium">到</th>
                      <th className="text-left py-1.5 font-medium">日期</th>
                      <th className="text-right py-1.5 font-medium">跨度</th>
                    </tr>
                  </thead>
                  <tbody>
                    {timeline.jumps.map((s, idx) => (
                      <tr
                        key={`${s.from_date}-${s.to_date}-${idx}`}
                        className="border-b border-gray-700/40"
                      >
                        <td className="py-1.5">{s.from_section || "(root)"}</td>
                        <td className="py-1.5 tabular-nums">{s.from_date}</td>
                        <td className="py-1.5">{s.to_section || "(root)"}</td>
                        <td className="py-1.5 tabular-nums">{s.to_date}</td>
                        <td className="py-1.5 text-right text-text-muted tabular-nums">
                          {timelineSpanLabel(s.days)}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </Panel>
            </div>
          )}

          {timeline.unmarked_stretches.length > 0 && (
            <div className="mt-4">
              <Panel title={`没有标记的区间（${timeline.unmarked_stretches.length} 处）`}>
                <ul className="text-xs space-y-2">
                  {timeline.unmarked_stretches.map(([from, to, count], idx) => (
                    <li key={`${from}-${to}-${idx}`}>
                      「{from || "(root)"}」到「{to || "(root)"}」：{formatInt(count)} 段没有一处时间标记
                    </li>
                  ))}
                </ul>
                <p className="mt-2 text-xs text-text-muted">
                  没有标记不等于时间没有过去：这段可能只是不需要说「多久之后」。
                </p>
              </Panel>
            </div>
          )}

          <div className="mt-4">
            <Panel title={`标记样本（前 ${Math.min(timeline.markers.length, MAX_ROWS)} 处）`}>
              <table className="w-full text-xs">
                <thead className="text-text-muted">
                  <tr>
                    <th className="text-right py-1.5 font-medium">位置</th>
                    <th className="text-left py-1.5 font-medium">原文</th>
                    <th className="text-left py-1.5 font-medium">类型</th>
                    <th className="text-left py-1.5 font-medium">粒度</th>
                    <th className="text-right py-1.5 font-medium">估计距离</th>
                  </tr>
                </thead>
                <tbody>
                  {timeline.markers.slice(0, MAX_ROWS).map((m, idx) => (
                    <tr key={`${m.kind}-${m.text}-${idx}`} className="border-b border-gray-700/40">
                      <td className="py-1.5 text-right tabular-nums">{formatPercent(m.position)}</td>
                      <td className="py-1.5 font-medium" title={m.excerpt}>
                        {m.text}
                      </td>
                      <td className="py-1.5 text-text-muted">{timelineKindLabel(m.kind)}</td>
                      <td className="py-1.5 text-text-muted">
                        {timelinePrecisionLabel(m.precision)}
                      </td>
                      <td className="py-1.5 text-right text-text-muted tabular-nums">
                        {m.offset_days !== 0 ? timelineSignedSpan(m.offset_days) : "—"}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
              {timeline.markers.length > MAX_ROWS && (
                <p className="mt-2 text-xs text-text-muted">
                  另有 {formatInt(timeline.markers.length - MAX_ROWS)} 处标记未列出。
                </p>
              )}
            </Panel>
          </div>

          <div className="mt-4">
            <Panel title={`时间线提示（${notes.length}）`}>
              {notes.length === 0 ? (
                <p className="text-xs text-text-muted">
                  日期没有提出需要留意的地方。这不表示时间线清楚，只表示这些标记没有报出什么。
                </p>
              ) : (
                <ul className="text-xs space-y-2">
                  {notes.map((i, idx) => (
                    <li key={`${i.kind}-${i.section}-${idx}`}>
                      <span
                        className={`px-2 py-0.5 rounded border text-xs ${timelineIssueStyle(i.kind)}`}
                      >
                        {timelineIssueLabel(i.kind)}
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

          {unreadableDates > 0 && (
            <p className="mt-2 text-xs text-text-muted">
              导入的实体标注里有 {formatInt(timeline.entity_date_count)} 处日期，本维度读到{" "}
              {formatInt(timeline.absolute_count)} 处：相差的部分是本文用不认的写法写的日期。
            </p>
          )}

          <p className="mt-2 text-xs text-text-muted">
            年号、农历、干支和外文日期不在识别范围内；月按 30 天、年按 365 天估算；只到年份的日期粒度太粗，不参与排序。
            {timeline.chunked &&
              "本文的章节少于三段，位置落在按字数等分的块上，是算术结果而非作者自己的分段。"}
          </p>
        </>
      )}
    </section>
  );
}

export default TimelineSection;


