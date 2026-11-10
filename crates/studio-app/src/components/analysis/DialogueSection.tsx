import { Panel, SectionTitle, StatCard, SECTION_ANCHOR } from "../Cards";
import BarList from "../charts/BarList";
import LineChart from "../charts/LineChart";
import {
  dialogueIssueLabel,
  dialogueIssueStyle,
  dialogueKindLabel,
  dialogueVerdictLabel,
  dialogueVerdictStyle,
  formatFixed,
  formatInt,
  formatPercent,
} from "../../lib/format";
import type { DialogueStats } from "../../types";

interface DialogueSectionProps {
  dialogue: DialogueStats;
}

/**
 * 对话/叙事比例 section: how the text divides between spoken and narrated, and where.
 *
 * There is deliberately no score here. 30% speech is neither better nor worse than
 * 5%, so the section counts and stops: the split, the curve over the same segments the
 * pacing, conflict, theme and setting curves use, how long the pieces of speech are,
 * and the shapes that usually need a look (a 300-character monologue, eight blocks
 * without one line of speech, speech that never wears quotes).
 */
function DialogueSection({ dialogue }: DialogueSectionProps) {
  const curve = dialogue.points.map((p) => ({
    label: p.section || "(root)",
    value: p.intensity / 100,
  }));
  // Segments shorter than 300 characters give a share that means nothing, so the
  // engine leaves them out of the comparison; the UI says so instead of showing 0%.
  const comparable = dialogue.peak_section !== "";

  return (
    <section id="dialogue" className={SECTION_ANCHOR}>
      <SectionTitle>对话/叙事比例</SectionTitle>

      <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
        <StatCard label="对白占比" value={formatPercent(dialogue.dialogue_share)} />
        <StatCard label="叙述占比" value={formatPercent(dialogue.narrative_share)} />
        <StatCard label="对白处数" value={formatInt(dialogue.unit_count)} />
        <StatCard label="平均每处" value={`${formatFixed(dialogue.avg_unit_chars)} 字`} />
      </div>

      <div className="mt-3 flex flex-wrap items-center gap-2">
        <span
          className={`px-2 py-0.5 rounded border text-xs ${dialogueVerdictStyle(dialogue.verdict)}`}
        >
          {dialogueVerdictLabel(dialogue.verdict)}
        </span>
        <span className="text-xs text-text-muted">
          引号 {formatInt(dialogue.quoted_units)} 处 · 破折号{" "}
          {formatInt(dialogue.dash_units)} 处 · 最长一处 {formatInt(dialogue.longest_unit)} 字 ·
          含对白 {formatInt(dialogue.blocks_with_dialogue)} / {formatInt(dialogue.blocks)} 块
        </span>
      </div>

      <p className="mt-2 text-xs text-text-muted">
        全文 {formatInt(dialogue.char_count)} 字：对白{" "}
        {formatInt(dialogue.dialogue_chars)} 字 · 叙述{" "}
        {formatInt(dialogue.narrative_chars)} 字。比例只说明对白与叙述的多少，不说明好坏，因此本维度不给出评分。
      </p>

      {comparable ? (
        <p className="mt-1 text-xs text-text-muted">
          最密 {dialogue.peak_section}（{formatPercent(dialogue.peak_share)}）· 最疏{" "}
          {dialogue.quietest_section || "—"}（{formatPercent(dialogue.quietest_share)}）· 相差{" "}
          {formatPercent(dialogue.spread)} · 连续无对白最长{" "}
          {formatInt(dialogue.longest_quiet_run)} 块 /{" "}
          {formatInt(dialogue.longest_quiet_chars)} 字 · 连续对白最长{" "}
          {formatInt(dialogue.longest_dialogue_run)} 块
        </p>
      ) : (
        <p className="mt-1 text-xs text-text-muted">
          各段均不足 300 字，不比较段落间的对白差异。连续无对白最长{" "}
          {formatInt(dialogue.longest_quiet_run)} 块 /{" "}
          {formatInt(dialogue.longest_quiet_chars)} 字。
        </p>
      )}

      <div className="mt-4">
        <Panel title={`对白占比曲线（${dialogue.segment_count} 段 · 强度 0-100）`}>
          {curve.length === 0 ? (
            <p className="text-xs text-text-muted">未识别到曲线：文档没有可切分的段落。</p>
          ) : (
            <LineChart points={curve} color="var(--color-dim-dialogue)" />
          )}
          {dialogue.chunked && (
            <p className="text-xs text-text-muted mt-2">
              章节不足 3 个：曲线按 {dialogue.segment_count} 个等长块切分。
            </p>
          )}
        </Panel>
      </div>

      <div className="mt-4 grid md:grid-cols-2 gap-4">
        <Panel title="对白长度分布">
          {dialogue.unit_count === 0 ? (
            <p className="text-xs text-text-muted">没有检测到对白，因此没有长度分布。</p>
          ) : (
            <BarList
              items={dialogue.unit_length_distribution.map(([label, value]) => ({
                label: `${label} 字`,
                value,
              }))}
              colorClass="bg-info"
            />
          )}
        </Panel>

        <Panel title={`最长的几处对白（${dialogue.samples.length} 处）`}>
          {dialogue.samples.length === 0 ? (
            <p className="text-xs text-text-muted">没有检测到对白。</p>
          ) : (
            <ul className="text-xs space-y-2">
              {dialogue.samples.map((s, idx) => (
                <li key={`${s.section}-${idx}-${s.chars}`}>
                  <span className="font-medium">{s.chars} 字</span>
                  <span className="text-text-muted">
                    （{s.section || "(root)"} · {dialogueKindLabel(s.kind)}）
                  </span>
                  <div className="text-text-muted mt-0.5">「{s.excerpt}」</div>
                </li>
              ))}
            </ul>
          )}
        </Panel>
      </div>

      <div className="mt-4 grid md:grid-cols-2 gap-4">
        <Panel title={`对白问题（${dialogue.issues.length}）`}>
          {dialogue.issues.length === 0 ? (
            <p className="text-xs text-text-muted">没有发现需要处理的比例问题。</p>
          ) : (
            <ul className="text-xs space-y-2">
              {dialogue.issues.map((i) => (
                <li key={`${i.kind}-${i.section}-${i.detail}`}>
                  <span
                    className={`px-2 py-0.5 rounded border text-xs ${dialogueIssueStyle(i.kind)}`}
                  >
                    {dialogueIssueLabel(i.kind)}
                  </span>
                  <span className="text-text-muted">
                    （{i.section || "全文"}）{i.detail}
                  </span>
                </li>
              ))}
            </ul>
          )}
        </Panel>

        <Panel title="识别说明">
          <p className="text-xs text-text-muted">
            对白按两种信号识别：引号（“ ” 「 」 『 』）内的文字，以及以——开头的行。引号在换行处收口，所以跨段的长对白不会把中间的叙述也算成对白；已被引号覆盖的破折号行不重复计数。
          </p>
          <p className="text-xs text-text-muted mt-3 pt-2 border-t border-line/60">
            原文不用引号时这里会漏判：未见对白不等于没有人说话。引号本身计入叙述一侧。
          </p>
        </Panel>
      </div>
    </section>
  );
}

export default DialogueSection;
