import { Panel, SectionTitle, StatCard, SECTION_ANCHOR } from "../Cards";
import LineChart from "../charts/LineChart";
import {
  formatFixed,
  formatInt,
  formatPercent,
  voiceBadgeStyle,
  voiceIssueLabel,
  voiceStanceLabel,
} from "../../lib/format";
import type { VoiceStats } from "../../types";

interface VoiceSectionProps {
  voice: VoiceStats;
}

/**
 * 作者声音 section: the habits that stay put whatever the story is about.
 *
 * Deliberately no score. Long sentences are not better than short ones and a
 * hedging narrator is not worse than an assertive one, so the section measures
 * rhythm, marks, function words, person and stance and then stops. Every rate is
 * per thousand visible characters over the same text, so two projects can be read
 * side by side; the rhythm curve is cut by the same segments the pacing, conflict,
 * theme, setting and dialogue curves use.
 */
function VoiceSection({ voice }: VoiceSectionProps) {
  // LineChart expects 0-1 values, so the curve is drawn relative to the longest
  // average in this document; the panel says so rather than hiding the scale.
  const longest = Math.max(...voice.points.map((p) => p.avg_sentence_chars), 1);
  const curve = voice.points.map((p) => ({
    label: p.section || "(root)",
    value: p.avg_sentence_chars / longest,
  }));
  const marks = voice.marks.filter((m) => m.count > 0);
  const snippetShare =
    voice.paragraph_count === 0 ? 0 : voice.single_sentence_paragraphs / voice.paragraph_count;

  return (
    <section id="voice" className={SECTION_ANCHOR}>
      <SectionTitle>作者声音</SectionTitle>

      <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
        <StatCard label="平均句长" value={`${formatFixed(voice.avg_sentence_chars)} 字`} />
        <StatCard label="句长变异系数" value={formatFixed(voice.sentence_cv, 2)} />
        <StatCard label="每句小句数" value={formatFixed(voice.clauses_per_sentence, 2)} />
        <StatCard label="平均段落长" value={`${formatFixed(voice.avg_paragraph_chars)} 字`} />
      </div>

      <p className="mt-2 text-xs text-text-muted">
        全文 {formatInt(voice.char_count)} 字 · {formatInt(voice.sentence_count)} 句（最短{" "}
        {formatInt(voice.shortest_sentence)} 字 · 最长 {formatInt(voice.longest_sentence)} 字 ·
        标准差 {formatFixed(voice.sentence_stdev)}）· 段落 {formatInt(voice.paragraph_count)} 段（
        {formatPercent(snippetShare)} 只有一句话）· 本维度不给出评分。
      </p>

      <div className="mt-3 flex flex-wrap items-center gap-2">
        <span className={`px-2 py-0.5 rounded border text-xs ${voiceBadgeStyle()}`}>
          {voiceStanceLabel(voice.stance.verdict)}
        </span>
        <span className="text-xs text-text-muted">
          犹疑 {formatInt(voice.stance.hedging)} 处（{formatFixed(voice.stance.hedging_per_10k)}{" "}
          /万字）· 断言 {formatInt(voice.stance.assertive)} 处（
          {formatFixed(voice.stance.assertive_per_10k)} /万字）
        </span>
      </div>

      <div className="mt-4">
        <Panel
          title={`句长曲线（${voice.segment_count} 段 · 相对最长段 ${formatFixed(longest)} 字）`}
        >
          {curve.length === 0 ? (
            <p className="text-xs text-text-muted">未识别到曲线：文档没有可切分的段落。</p>
          ) : (
            <LineChart points={curve} color="var(--color-dim-voice)" />
          )}
          {voice.chunked && (
            <p className="text-xs text-text-muted mt-2">
              章节不足 3 个：曲线按 {voice.segment_count} 个等长块切分，是算术结果而非作者自己的分段。
            </p>
          )}
        </Panel>
      </div>

      <div className="mt-4 grid md:grid-cols-2 gap-4">
        <Panel title={`标点密度（每千字，用到 ${formatInt(marks.length)} 种）`}>
          {marks.length === 0 ? (
            <p className="text-xs text-text-muted">这段文本没有用到清单里的标点。</p>
          ) : (
            <table className="w-full text-xs">
              <thead className="text-text-muted">
                <tr>
                  <th className="text-left py-1 font-medium">标点</th>
                  <th className="text-right py-1 font-medium">次数</th>
                  <th className="text-right py-1 font-medium">每千字</th>
                </tr>
              </thead>
              <tbody>
                {marks.map((m) => (
                  <tr key={m.mark} className="border-b border-line/40">
                    <td className="py-1">
                      <span className="font-medium">{m.mark}</span>
                      <span className="text-text-muted"> {m.label}</span>
                    </td>
                    <td className="py-1 text-right tabular-nums">{formatInt(m.count)}</td>
                    <td className="py-1 text-right tabular-nums">{formatFixed(m.per_1000, 2)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
          <p className="text-xs text-text-muted mt-2">省略号与破折号按字符计：一个 …… 记 2 次。</p>
        </Panel>

        <Panel title="功能词密度（每千字）">
          <table className="w-full text-xs">
            <thead className="text-text-muted">
              <tr>
                <th className="text-left py-1 font-medium">类别</th>
                <th className="text-left py-1 font-medium">字</th>
                <th className="text-right py-1 font-medium">每千字</th>
              </tr>
            </thead>
            <tbody>
              {voice.function_words.map((g) => (
                <tr key={g.group} className="border-b border-line/40">
                  <td className="py-1">{g.group}</td>
                  <td className="py-1 text-text-muted">{g.words}</td>
                  <td className="py-1 text-right tabular-nums">{formatFixed(g.per_1000, 2)}</td>
                </tr>
              ))}
            </tbody>
          </table>
          <p className="text-xs text-text-muted mt-2">
            按单字计数：「了解」记一个「了」。这是习惯的密度，不是语法标注。
          </p>
        </Panel>
      </div>

      <div className="mt-4 grid md:grid-cols-2 gap-4">
        <Panel title={`读得到的习惯（${voice.issues.length}）`}>
          {voice.issues.length === 0 ? (
            <p className="text-xs text-text-muted">这一次没有读出值得命名的习惯。</p>
          ) : (
            <ul className="text-xs space-y-2">
              {voice.issues.map((i) => (
                <li key={`${i.kind}-${i.detail}`}>
                  <span className={`px-2 py-0.5 rounded border text-xs ${voiceBadgeStyle()}`}>
                    {voiceIssueLabel(i.kind)}
                  </span>
                  <span className="text-text-muted"> {i.detail}</span>
                </li>
              ))}
            </ul>
          )}
        </Panel>

        <Panel title="人称与视角">
          <table className="w-full text-xs">
            <tbody>
              {(
                [
                  ["第一人称", voice.pronouns.first, voice.pronouns.first_share],
                  ["第二人称", voice.pronouns.second, voice.pronouns.second_share],
                  ["第三人称", voice.pronouns.third, voice.pronouns.third_share],
                ] as [string, number, number][]
              ).map(([label, count, share]) => (
                <tr key={label} className="border-b border-line/40">
                  <td className="py-1">{label}</td>
                  <td className="py-1 text-right tabular-nums">{formatInt(count)}</td>
                  <td className="py-1 text-right tabular-nums">{formatPercent(share)}</td>
                </tr>
              ))}
            </tbody>
          </table>
          <p className="text-xs text-text-muted mt-2">
            共 {formatInt(voice.pronouns.total)} 处人称代词。「其他」「别的」里的「他/它」不算人称。
          </p>
          <p className="text-xs text-text-muted mt-3 pt-2 border-t border-line/60">
            这一维度只描述写法：句子长短、标点密度、功能词、人称与立场。它不给出评分，也不判断好坏。
          </p>
        </Panel>
      </div>
    </section>
  );
}

export default VoiceSection;
