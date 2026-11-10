import { Panel, SectionTitle, StatCard } from "../Cards";
import LineChart from "../charts/LineChart";
import {
  formatFixed,
  formatInt,
  formatOptionalScore,
  formatPercent,
  formatScore,
  placeKindLabel,
  settingIssueLabel,
  settingIssueStyle,
  settingVerdictLabel,
  settingVerdictStyle,
} from "../../lib/format";
import type { SettingStats } from "../../types";

interface SettingSectionProps {
  setting: SettingStats;
}

/**
 * 场景/设定 section: where the story happens, and whether the reader is told.
 *
 * A scene here is a run of blocks that stays at one place: the place comes from the
 * importer's location entities, and description counts as an anchor too, because
 * prose rarely re-states where it is. The section therefore reports both — the
 * places the text names and the environment vocabulary it spends characters on —
 * and flags the scenes that have neither.
 */
function SettingSection({ setting }: SettingSectionProps) {
  const score = setting.setting_score;
  const curve = setting.points.map((p) => ({
    label: p.section || "(root)",
    value: p.intensity / 100,
  }));

  return (
    <section>
      <SectionTitle>场景/设定</SectionTitle>

      <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
        <StatCard label="场景评分" value={formatOptionalScore(score)} />
        <StatCard label="场景数" value={formatInt(setting.scene_count)} />
        <StatCard
          label="地点数"
          value={setting.places_measured ? formatInt(setting.place_count) : "未测量"}
        />
        <StatCard label="环境千字密度" value={formatFixed(setting.env_density)} />
      </div>

      <div className="mt-3 flex flex-wrap items-center gap-2">
        <span
          className={`px-2 py-0.5 rounded border text-xs ${settingVerdictStyle(setting.verdict)}`}
        >
          {settingVerdictLabel(setting.verdict)}
        </span>
        <span className="text-xs text-text-muted">
          无地点锚点的场景 {setting.unanchored_scenes}/{setting.scene_count}（
          {formatPercent(setting.unanchored_share)}）· 主导地点{" "}
          {setting.dominant_place || "—"} · 环境峰值 {setting.peak_section || "—"}（
          {formatScore(setting.peak_intensity)}）
        </span>
      </div>

      {score === null && (
        <p className="mt-2 text-xs text-warn">
          既没有点名地点，也没有环境用词，因此不给出评分：文本可能不是叙事文本，也可能是导入结果里没有地点实体。
        </p>
      )}

      <div className="mt-4">
        <Panel title={`地点（${setting.places.length} 个）`}>
          {setting.places.length === 0 ? (
            <p className="text-xs text-text-muted">
              没有反复出现的地点：所有地名都只出现过一次，场景只能靠描写支撑。
            </p>
          ) : (
            <div className="overflow-x-auto">
              <table className="w-full text-xs">
                <thead>
                  <tr className="text-text-muted border-b border-line">
                    <th className="text-left py-1.5 font-medium">地点</th>
                    <th className="text-left py-1.5 font-medium">类型</th>
                    <th className="text-right py-1.5 font-medium">提及</th>
                    <th className="text-right py-1.5 font-medium">覆盖</th>
                    <th className="text-left py-1.5 font-medium">首次 → 末次</th>
                    <th className="text-left py-1.5 font-medium">缺席</th>
                  </tr>
                </thead>
                <tbody>
                  {setting.places.map((p) => (
                    <tr key={p.name} className="border-b border-line/40">
                      <td className="py-1.5 truncate max-w-[220px]" title={p.name}>
                        <span className="font-medium">{p.name}</span>
                      </td>
                      <td className="py-1.5 text-text-muted">{placeKindLabel(p.kind)}</td>
                      <td className="py-1.5 text-right tabular-nums">{p.mentions}</td>
                      <td className="py-1.5 text-right text-text-muted tabular-nums">
                        {formatPercent(p.coverage)}
                      </td>
                      <td className="py-1.5 text-text-muted truncate max-w-[200px]">
                        {p.first_section || "(root)"} → {p.last_section || "(root)"}
                      </td>
                      <td className="py-1.5 text-text-muted">
                        {p.longest_absence > 0 ? `${p.longest_absence} 段` : "—"}
                        {p.returns > 0 && ` · 回归 ${p.returns} 次`}
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
        <Panel title={`环境曲线（${setting.segment_count} 段 · 强度 0-100）`}>
          {curve.length === 0 ? (
            <p className="text-xs text-text-muted">未识别到环境曲线：文档没有可切分的段落。</p>
          ) : (
            <LineChart points={curve} color="var(--color-dim-setting)" />
          )}
          {setting.chunked && (
            <p className="text-xs text-text-muted mt-2">
              章节不足 3 个：曲线按 {setting.segment_count} 个等长块切分。
            </p>
          )}
        </Panel>
      </div>

      <div className="mt-4">
        <Panel
          title={`场景清单（显示 ${Math.min(setting.scenes.length, 20)} / ${setting.scene_count} 场）`}
        >
          {setting.scenes.length === 0 ? (
            <p className="text-xs text-text-muted">未识别到场景：文档没有可切分的段落。</p>
          ) : (
            <div className="overflow-x-auto">
              <table className="w-full text-xs">
                <thead>
                  <tr className="text-text-muted border-b border-line">
                    <th className="text-right py-1.5 font-medium">#</th>
                    <th className="text-left py-1.5 font-medium">段落</th>
                    <th className="text-left py-1.5 font-medium">地点</th>
                    <th className="text-right py-1.5 font-medium">块</th>
                    <th className="text-right py-1.5 font-medium">字数</th>
                    <th className="text-right py-1.5 font-medium">环境/千字</th>
                    <th className="text-right py-1.5 font-medium">对话</th>
                    <th className="text-left py-1.5 font-medium">在场人物</th>
                  </tr>
                </thead>
                <tbody>
                  {setting.scenes.slice(0, 20).map((s) => (
                    <tr key={s.index} className="border-b border-line/40">
                      <td className="py-1.5 text-right text-text-muted tabular-nums">
                        {s.index + 1}
                      </td>
                      <td className="py-1.5 truncate max-w-[180px]" title={s.section}>
                        {s.section || "(root)"}
                        {s.time_mark && <span className="text-text-muted"> · 时间标记</span>}
                      </td>
                      <td className="py-1.5">
                        {s.anchor ? (
                          <span className="font-medium">{s.anchor}</span>
                        ) : (
                          <span className="text-warn">（未点名）</span>
                        )}
                      </td>
                      <td className="py-1.5 text-right text-text-muted tabular-nums">{s.blocks}</td>
                      <td className="py-1.5 text-right text-text-muted tabular-nums">
                        {formatInt(s.char_count)}
                      </td>
                      <td className="py-1.5 text-right text-text-muted tabular-nums">
                        {formatFixed(s.env_density)}
                      </td>
                      <td className="py-1.5 text-right text-text-muted tabular-nums">
                        {formatPercent(s.dialogue_share)}
                      </td>
                      <td className="py-1.5 text-text-muted truncate max-w-[220px]">
                        {s.persons.length > 0
                          ? s.persons.map(([name, count]) => `${name}×${count}`).join("、")
                          : "—"}
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
        <Panel title={`场景问题（${setting.issues.length}）`}>
          {setting.issues.length === 0 ? (
            <p className="text-xs text-text-muted">没有发现需要处理的场景问题。</p>
          ) : (
            <ul className="text-xs space-y-2">
              {setting.issues.map((i) => (
                <li key={`${i.kind}-${i.section}-${i.detail}`}>
                  <span
                    className={`px-2 py-0.5 rounded border text-xs ${settingIssueStyle(i.kind)}`}
                  >
                    {settingIssueLabel(i.kind)}
                  </span>
                  <span className="text-text-muted">
                    （{i.section || "全文"}）{i.detail}
                  </span>
                </li>
              ))}
            </ul>
          )}
        </Panel>

        <Panel title="环境用词与评分说明">
          {setting.top_env_terms.length === 0 ? (
            <p className="text-xs text-text-muted">没有环境用词记录。</p>
          ) : (
            <p className="text-xs text-text-muted">
              {setting.top_env_terms.map(([term, count]) => `${term}×${count}`).join("、")}
            </p>
          )}
          <p className="text-xs text-text-muted mt-3 pt-2 border-t border-line/60">
            场景评分从 100 起扣：超过三成的场景没有地点锚点最多 −25，环境用词偏少最多 −20，
            既没有反复出现的地点也没有密集描写 −20，一段里挤进 4 个以上地点每个 −5（最多 −15）；
            地点与描写都没有时不评分。
          </p>
        </Panel>
      </div>

      <div className="mt-4">
        <Panel title="地点切分只到词面为止">
          <p className="text-xs text-text-muted">
            场景按地点实体与环境用词切分：包含关系与同段共现不会合并，地名之外的地点（例如只说「推开门」）
            只能靠描写计数。是否真的换了场景、地点是否前后一致，需要人来核对。
          </p>
        </Panel>
      </div>

    </section>
  );
}

export default SettingSection;
