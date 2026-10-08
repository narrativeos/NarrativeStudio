import { describe, expect, it } from "vitest";

import type { T0Stats } from "../types";
import {
  arcShapeLabel,
  clicheCategoryLabel,
  clicheIssueLabel,
  clicheIssueStyle,
  clicheVerdictLabel,
  clicheVerdictStyle,
  conflictTypeLabel,
  conflictVerdictLabel,
  conflictVerdictStyle,
  dialogueIssueLabel,
  dialogueIssueStyle,
  dialogueKindLabel,
  dialogueVerdictLabel,
  dialogueVerdictStyle,
  entityTotal,
  explicitAdvisoryLabel,
  explicitAdvisoryStyle,
  explicitCategoryLabel,
  explicitIssueLabel,
  explicitIssueStyle,
  explicitTierLabel,
  explicitTierStyle,
  formatFixed,
  formatInt,
  formatOptionalScore,
  formatPercent,
  formatScore,
  pacingVerdictLabel,
  pacingVerdictStyle,
  placeKindLabel,
  plotShapeLabel,
  plotShapeStyle,
  scoreColor,
  severityLabel,
  severityStyle,
  settingIssueLabel,
  settingIssueStyle,
  settingVerdictLabel,
  settingVerdictStyle,
  structureActLabel,
  structureBeatLabel,
  structureFitLabel,
  structureFitStyle,
  structureIssueLabel,
  structureIssueStyle,
  structureLandmarkLabel,
  structureModelLabel,
  structureStatusLabel,
  structureStatusStyle,
  timelineIssueLabel,
  timelineIssueStyle,
  timelineKindLabel,
  timelinePrecisionLabel,
  timelineSignedSpan,
  timelineSpanLabel,
  themeConcentrationLabel,
  themeReachLabel,
  themeReachStyle,
  themeSourceLabel,
} from "./format";

/** Only `entity_counts` matters for these helpers; the rest of T0Stats is unused. */
function t0With(entityCounts: [string, number][]): T0Stats {
  return { entity_counts: entityCounts } as T0Stats;
}

describe("entityTotal", () => {
  it("sums occurrences across categories", () => {
    expect(entityTotal(t0With([["PERSON", 3], ["LOCATION", 10], ["DATE", 2]]))).toBe(15);
  });

  it("is zero when no entities were detected", () => {
    expect(entityTotal(t0With([]))).toBe(0);
  });
});

describe("number formatting", () => {
  it("groups thousands (locale-aware separator)", () => {
    expect(formatInt(1234)).toMatch(/^1[,\s\u00A0]234$/);
  });

  it("renders scores as whole numbers", () => {
    expect(formatScore(86.4)).toBe("86");
    expect(formatScore(0)).toBe("0");
  });

  it("renders ratios as whole percents", () => {
    expect(formatPercent(0.427)).toBe("43%");
    expect(formatPercent(0)).toBe("0%");
    expect(formatPercent(1)).toBe("100%");
  });

  it("defaults to one decimal for averages", () => {
    expect(formatFixed(23.456)).toBe("23.5");
    expect(formatFixed(23.456, 2)).toBe("23.46");
  });
});

describe("scoreColor", () => {
  it("uses green at or above 80", () => {
    expect(scoreColor(80)).toBe("bg-emerald-400");
    expect(scoreColor(100)).toBe("bg-emerald-400");
  });

  it("uses amber between 60 and 79.99", () => {
    expect(scoreColor(79.9)).toBe("bg-amber-400");
    expect(scoreColor(60)).toBe("bg-amber-400");
  });

  it("uses red below 60", () => {
    expect(scoreColor(59.9)).toBe("bg-red-400");
    expect(scoreColor(0)).toBe("bg-red-400");
  });
});

describe("severity helpers", () => {
  it("maps known severities to badge classes and labels", () => {
    expect(severityStyle("high")).toContain("bg-red-500/15");
    expect(severityStyle("medium")).toContain("bg-amber-500/15");
    expect(severityLabel("high")).toBe("高");
    expect(severityLabel("low")).toBe("低");
  });

  it("falls back to the low style and the raw label for unknown severities", () => {
    expect(severityStyle("critical")).toBe(severityStyle("low"));
    expect(severityLabel("critical")).toBe("critical");
  });
});

describe("arcShapeLabel", () => {
  it("localizes the four known shapes", () => {
    expect(arcShapeLabel("mountain")).toBe("先扬后抑");
    expect(arcShapeLabel("rising")).toBe("渐强");
    expect(arcShapeLabel("falling")).toBe("渐弱");
    expect(arcShapeLabel("steady")).toBe("平稳");
  });

  it("passes unknown shapes through instead of hiding them", () => {
    expect(arcShapeLabel("wave")).toBe("wave");
  });
});

describe("plot shape helpers", () => {
  it("localizes every shape the analysis can emit", () => {
    expect(plotShapeLabel("sustained")).toBe("贯穿");
    expect(plotShapeLabel("intermittent")).toBe("断续");
    expect(plotShapeLabel("burst")).toBe("集中");
    expect(plotShapeLabel("setup_only")).toBe("仅铺垫");
    expect(plotShapeLabel("late_arrival")).toBe("迟到");
    expect(plotShapeLabel("minor")).toBe("次要");
  });

  it("passes unknown shapes through instead of hiding them", () => {
    expect(plotShapeLabel("wave")).toBe("wave");
  });

  it("marks the two shapes that ask for a rewrite as warnings", () => {
    expect(plotShapeStyle("setup_only")).toContain("text-red-400");
    expect(plotShapeStyle("late_arrival")).toContain("text-red-400");
    expect(plotShapeStyle("sustained")).toContain("text-emerald-400");
  });

  it("falls back to the neutral style for unknown shapes", () => {
    expect(plotShapeStyle("wave")).toBe(plotShapeStyle("minor"));
  });
});

describe("pacing verdict helpers", () => {
  it("localizes every verdict the analysis can emit", () => {
    expect(pacingVerdictLabel("flat")).toBe("单调");
    expect(pacingVerdictLabel("varied")).toBe("起伏");
    expect(pacingVerdictLabel("spiky")).toBe("跳跃");
  });

  it("passes an unknown verdict through instead of hiding it", () => {
    expect(pacingVerdictLabel("wave")).toBe("wave");
  });

  it("marks a curve that never moves as the case to fix", () => {
    expect(pacingVerdictStyle("flat")).toContain("text-red-400");
    expect(pacingVerdictStyle("varied")).toContain("text-emerald-400");
    expect(pacingVerdictStyle("spiky")).toContain("text-amber-400");
  });

  it("falls back to the warning style for an unknown verdict", () => {
    expect(pacingVerdictStyle("wave")).toBe(pacingVerdictStyle("flat"));
  });
});

describe("conflict helpers", () => {
  it("localizes every family the analysis can emit", () => {
    expect(conflictTypeLabel("interpersonal")).toBe("人际");
    expect(conflictTypeLabel("internal")).toBe("内心");
    expect(conflictTypeLabel("societal")).toBe("社会");
    expect(conflictTypeLabel("survival")).toBe("生存");
  });

  it("names the empty family as 无 rather than hiding it", () => {
    expect(conflictTypeLabel("")).toBe("无");
  });

  it("passes an unknown family through instead of hiding it", () => {
    expect(conflictTypeLabel("cosmic")).toBe("cosmic");
  });

  it("localizes every verdict the analysis can emit", () => {
    expect(conflictVerdictLabel("none")).toBe("未见用词");
    expect(conflictVerdictLabel("thin")).toBe("偏少");
    expect(conflictVerdictLabel("present")).toBe("适中");
    expect(conflictVerdictLabel("heavy")).toBe("密集");
  });

  it("keeps 未见用词 neutral: no vocabulary is not the same as no conflict", () => {
    expect(conflictVerdictStyle("none")).toContain("text-text-muted");
    expect(conflictVerdictStyle("thin")).toContain("text-amber-400");
    expect(conflictVerdictStyle("present")).toContain("text-emerald-400");
  });

  it("falls back to the neutral style for an unknown verdict", () => {
    expect(conflictVerdictStyle("wave")).toBe(conflictVerdictStyle("none"));
  });
});

describe("formatOptionalScore", () => {
  it("says 未评分 instead of inventing a zero", () => {
    expect(formatOptionalScore(null)).toBe("未评分");
  });

  it("formats a present score like the other scores", () => {
    expect(formatOptionalScore(72.4)).toBe("72 / 100");
  });
});

describe("theme labels", () => {
  it("names the vocabulary the analysis read", () => {
    expect(themeSourceLabel("noun_signals")).toBe("名词信号");
    expect(themeSourceLabel("noun_tokens")).toBe("名词词性");
    expect(themeSourceLabel("none")).toBe("无");
  });

  it("localizes every reach the analysis can emit", () => {
    expect(themeReachLabel("sustained")).toBe("贯穿");
    expect(themeReachLabel("recurring")).toBe("复现");
    expect(themeReachLabel("episodic")).toBe("偶发");
    expect(themeReachLabel("occasional")).toBe("零星");
  });

  it("reads a subject that runs through the document as the healthy case", () => {
    expect(themeReachStyle("sustained")).toContain("text-emerald-400");
    expect(themeReachStyle("episodic")).toContain("text-amber-400");
    expect(themeReachStyle("occasional")).toContain("text-text-muted");
  });

  it("falls back to the muted style for an unknown reach", () => {
    expect(themeReachStyle("scattered")).toBe(themeReachStyle("occasional"));
  });

  it("localizes where a theme's mentions sit", () => {
    expect(themeConcentrationLabel("spread")).toBe("均匀");
    expect(themeConcentrationLabel("front")).toBe("集中在开头");
    expect(themeConcentrationLabel("middle")).toBe("集中在中段");
    expect(themeConcentrationLabel("back")).toBe("集中在结尾");
  });

  it("passes an unknown label through instead of hiding it", () => {
    expect(themeSourceLabel("embeddings")).toBe("embeddings");
    expect(themeReachLabel("weekly")).toBe("weekly");
    expect(themeConcentrationLabel("sideways")).toBe("sideways");
  });
});

describe("setting labels", () => {
  it("localizes every environment verdict the analysis can emit", () => {
    expect(settingVerdictLabel("none")).toBe("未见用词");
    expect(settingVerdictLabel("thin")).toBe("偏少");
    expect(settingVerdictLabel("present")).toBe("适中");
    expect(settingVerdictLabel("heavy")).toBe("密集");
  });

  it("keeps 未见用词 neutral: no vocabulary is not the same as no setting", () => {
    expect(settingVerdictStyle("none")).toContain("text-text-muted");
    expect(settingVerdictStyle("thin")).toContain("text-amber-400");
    expect(settingVerdictStyle("present")).toContain("text-emerald-400");
  });

  it("falls back to the neutral style for an unknown verdict", () => {
    expect(settingVerdictStyle("sparse")).toBe(settingVerdictStyle("none"));
  });

  it("names the kind of place the importer found", () => {
    expect(placeKindLabel("LOCATION")).toBe("地点");
    expect(placeKindLabel("FACILITY")).toBe("场所");
    expect(placeKindLabel("ORGANIZATION")).toBe("ORGANIZATION");
  });

  it("localizes every scene issue the analysis can emit", () => {
    expect(settingIssueLabel("unanchored")).toBe("无地点锚点");
    expect(settingIssueLabel("crowded")).toBe("地点过密");
    expect(settingIssueLabel("long_absence")).toBe("长时间离开地点");
    expect(settingIssueLabel("thin_environment")).toBe("环境描写偏少");
  });

  it("reads a scene that never says where it is as the case to fix", () => {
    expect(settingIssueStyle("unanchored")).toContain("text-red-400");
    expect(settingIssueStyle("long_absence")).toContain("text-sky-400");
  });

  it("falls back to the neutral issue style for an unknown kind", () => {
    expect(settingIssueStyle("drifting")).toBe(settingIssueStyle("long_absence"));
  });
});

describe("cliché labels", () => {
  it("localizes every phrase verdict the analysis can emit", () => {
    expect(clicheVerdictLabel("none")).toBe("未见用词");
    expect(clicheVerdictLabel("light")).toBe("偏少");
    expect(clicheVerdictLabel("moderate")).toBe("中等");
    expect(clicheVerdictLabel("heavy")).toBe("密集");
  });

  it("keeps 未见用词 neutral: an empty library hit is not a clean bill of health", () => {
    expect(clicheVerdictStyle("none")).toContain("text-text-muted");
    expect(clicheVerdictStyle("light")).toContain("text-emerald-400");
    expect(clicheVerdictStyle("heavy")).toContain("text-red-400");
  });

  it("falls back to the neutral style for an unknown verdict", () => {
    expect(clicheVerdictStyle("sparse")).toBe(clicheVerdictStyle("none"));
  });

  it("names every phrase family the library carries", () => {
    expect(clicheCategoryLabel("expression")).toBe("表情动作");
    expect(clicheCategoryLabel("body")).toBe("身体反应");
    expect(clicheCategoryLabel("emotion")).toBe("情绪直陈");
    expect(clicheCategoryLabel("simile")).toBe("套路比喻");
    expect(clicheCategoryLabel("atmosphere")).toBe("场面套话");
    expect(clicheCategoryLabel("speech_tag")).toBe("说话套话");
    expect(clicheCategoryLabel("transition")).toBe("时间过渡");
    expect(clicheCategoryLabel("look")).toBe("外貌套话");
    expect(clicheCategoryLabel("gesture")).toBe("gesture");
  });

  it("localizes every phrase issue the analysis can emit", () => {
    expect(clicheIssueLabel("repeated")).toBe("反复使用");
    expect(clicheIssueLabel("dense_segment")).toBe("局部密集");
    expect(clicheIssueLabel("one_note")).toBe("依赖同一句");
    expect(clicheIssueLabel("speech_tags")).toBe("对白标签过多");
  });

  it("reads repeating one phrase as the habit to fix first", () => {
    expect(clicheIssueStyle("repeated")).toContain("text-red-400");
    expect(clicheIssueStyle("speech_tags")).toContain("text-sky-400");
  });

  it("falls back to the segment style for an unknown issue kind", () => {
    expect(clicheIssueStyle("stale")).toBe(clicheIssueStyle("dense_segment"));
  });
});

describe("dialogue labels", () => {
  it("localizes every mix verdict the analysis can emit", () => {
    expect(dialogueVerdictLabel("none")).toBe("未见对白");
    expect(dialogueVerdictLabel("sparse")).toBe("对白偏少");
    expect(dialogueVerdictLabel("balanced")).toBe("对白与叙述混合");
    expect(dialogueVerdictLabel("heavy")).toBe("以对白为主");
  });

  it("keeps every mix verdict off the red end: a ratio is not a grade", () => {
    for (const verdict of ["none", "sparse", "balanced", "heavy"]) {
      expect(dialogueVerdictStyle(verdict)).not.toContain("text-red-400");
    }
    expect(dialogueVerdictStyle("none")).toContain("text-text-muted");
  });

  it("falls back to the neutral style for an unknown verdict", () => {
    expect(dialogueVerdictStyle("loud")).toBe(dialogueVerdictStyle("none"));
  });

  it("names how a piece of speech was detected", () => {
    expect(dialogueKindLabel("quote")).toBe("引号");
    expect(dialogueKindLabel("dash")).toBe("破折号");
    expect(dialogueKindLabel("paren")).toBe("paren");
  });

  it("localizes every mix issue the analysis can emit", () => {
    expect(dialogueIssueLabel("no_dialogue")).toBe("没有对白");
    expect(dialogueIssueLabel("silent_segment")).toBe("长时间无对白");
    expect(dialogueIssueLabel("flooded_segment")).toBe("局部对白过密");
    expect(dialogueIssueLabel("monologue")).toBe("长独白");
    expect(dialogueIssueLabel("telegraphic")).toBe("对白过短");
    expect(dialogueIssueLabel("unquoted")).toBe("对白未加引号");
    expect(dialogueIssueLabel("uneven")).toBe("各段比例不均");
  });

  it("reads unquoted speech as punctuation, not as a mistake", () => {
    expect(dialogueIssueStyle("unquoted")).toContain("text-sky-400");
    expect(dialogueIssueStyle("monologue")).toContain("text-amber-400");
    expect(dialogueIssueStyle("no_dialogue")).toContain("text-text-muted");
  });

  it("falls back to the quiet-segment style for an unknown issue kind", () => {
    expect(dialogueIssueStyle("whispered")).toBe(dialogueIssueStyle("silent_segment"));
  });
});

describe("explicit language labels", () => {
  it("localizes every advisory level the analysis can emit", () => {
    expect(explicitAdvisoryLabel("none")).toBe("词库未见");
    expect(explicitAdvisoryLabel("mild")).toBe("轻微");
    expect(explicitAdvisoryLabel("moderate")).toBe("中等");
    expect(explicitAdvisoryLabel("strong")).toBe("强烈");
  });

  it("says the library found nothing rather than that the text is clean", () => {
    // "无敏感内容" would claim more than a 35-pattern net can know, so the label
    // names what was silent: the library.
    expect(explicitAdvisoryLabel("none")).toContain("词库");
    expect(explicitAdvisoryStyle("none")).toContain("text-text-muted");
  });

  it("keeps the tier scale and the advisory level on the same heat", () => {
    // The advisory level is the strongest tier found, so the two must color alike
    // even though the report abbreviates the tier to one character.
    for (const tier of ["mild", "moderate", "strong"]) {
      expect(explicitAdvisoryStyle(tier)).toBe(explicitTierStyle(tier));
    }
    expect(explicitTierLabel("mild")).toBe("轻");
    expect(explicitTierLabel("strong")).toBe("重");
  });

  it("falls back to the neutral style for an unknown advisory level", () => {
    expect(explicitAdvisoryStyle("extreme")).toBe(explicitAdvisoryStyle("none"));
    expect(explicitAdvisoryLabel("extreme")).toBe("extreme");
    expect(explicitTierStyle("explicit")).toBe(explicitTierStyle("mild"));
  });

  it("names every content family the library carries", () => {
    expect(explicitCategoryLabel("profanity")).toBe("粗口辱骂");
    expect(explicitCategoryLabel("sexual")).toBe("性描写");
    expect(explicitCategoryLabel("sexual_violence")).toBe("性暴力");
    expect(explicitCategoryLabel("violence")).toBe("暴力血腥");
    expect(explicitCategoryLabel("self_harm")).toBe("自伤");
    expect(explicitCategoryLabel("hate")).toBe("贬损歧视");
    expect(explicitCategoryLabel("substance")).toBe("毒品");
  });

  it("localizes every content notice the analysis can emit", () => {
    expect(explicitIssueLabel("self_harm")).toBe("自伤描写");
    expect(explicitIssueLabel("sexual_violence")).toBe("性暴力");
    expect(explicitIssueLabel("sexual_content")).toBe("露骨性描写");
    expect(explicitIssueLabel("gore")).toBe("血腥细节");
    expect(explicitIssueLabel("hate_slur")).toBe("贬损歧视用语");
    expect(explicitIssueLabel("substance")).toBe("毒品描写");
    expect(explicitIssueLabel("dense_segment")).toBe("局部密集");
  });

  it("colors the few-hits families hottest and leaves spread separate from content", () => {
    // Self-harm surfaces from one moderate mention and sexual violence from one strong
    // one, so neither may be dimmed next to a page of mild profanity; a dense segment
    // says where hits cluster, not what they are, and stays off the content scale.
    expect(explicitIssueStyle("self_harm")).toContain("text-red-400");
    expect(explicitIssueStyle("sexual_violence")).toContain("text-red-400");
    expect(explicitIssueStyle("gore")).toContain("text-amber-400");
    expect(explicitIssueStyle("dense_segment")).toContain("text-sky-400");
  });

  it("falls back to the density style for an unknown notice kind", () => {
    expect(explicitIssueStyle("smuggling")).toBe(explicitIssueStyle("dense_segment"));
  });
});

describe("story structure labels", () => {
  it("names every shape the analysis tries", () => {
    expect(structureModelLabel("three_act")).toBe("三幕结构");
    expect(structureModelLabel("kisho_tenketsu")).toBe("起承转合");
    expect(structureModelLabel("hero_journey")).toBe("英雄之旅");
    expect(structureModelLabel("save_the_cat")).toBe("save_the_cat");
  });

  it("names every beat of every model", () => {
    const beats = [
      "setup",
      "first_turn",
      "midpoint",
      "second_turn",
      "climax",
      "resolution",
      "kisho",
      "sho",
      "ten",
      "ketsu",
      "ordinary_world",
      "call",
      "threshold",
      "tests",
      "inmost_cave",
      "resurrection",
      "return",
    ];
    for (const beat of beats) {
      // A beat left untranslated would show an English id in a Chinese table.
      expect(structureBeatLabel(beat)).not.toBe(beat);
    }
    expect(structureBeatLabel("midpoint")).toBe("中点");
    expect(structureBeatLabel("inmost_cave")).toBe("最深洞穴");
  });

  it("never paints an absent beat as a fault", () => {
    // 未见 says the signals showed nothing there, so it stays neutral rather than red.
    expect(structureStatusLabel("found")).toBe("命中");
    expect(structureStatusLabel("displaced")).toBe("错位");
    expect(structureStatusLabel("missing")).toBe("未见");
    expect(structureStatusStyle("found")).toContain("text-emerald-400");
    expect(structureStatusStyle("displaced")).toContain("text-amber-400");
    expect(structureStatusStyle("missing")).toContain("text-text-muted");
    expect(structureStatusStyle("missing")).not.toContain("red");
  });

  it("keeps the fit a word rather than a number", () => {
    // The dimension is descriptive: the verdict word always travels with its counts,
    // so it may not read like a grade.
    expect(structureFitLabel("matched")).toBe("吻合");
    expect(structureFitLabel("partial")).toBe("部分吻合");
    expect(structureFitLabel("unclear")).toBe("信号不足");
    expect(structureFitStyle("unclear")).toContain("text-text-muted");
    expect(structureFitStyle("nonsense")).toBe(structureFitStyle("unclear"));
  });

  it("distinguishes the kinds of landmark", () => {
    expect(structureLandmarkLabel("opening")).toBe("开篇");
    expect(structureLandmarkLabel("turn")).toBe("线程转折");
    expect(structureLandmarkLabel("peak")).toBe("张力高点");
    expect(structureLandmarkLabel("trough")).toBe("张力低点");
    expect(structureLandmarkLabel("climax")).toBe("张力最高点");
  });

  it("names the acts of the document's own split", () => {
    expect(structureActLabel("act_one")).toBe("第一幕");
    expect(structureActLabel("act_two")).toBe("第二幕");
    expect(structureActLabel("act_three")).toBe("第三幕");
    expect(structureActLabel("act_four")).toBe("act_four");
  });

  it("localizes every structure note the analysis can emit", () => {
    for (const kind of [
      "no_early_turn",
      "missing_midpoint",
      "climax_early",
      "no_descent",
      "act_imbalance",
      "too_few_landmarks",
    ]) {
      expect(structureIssueLabel(kind)).not.toBe(kind);
    }
    expect(structureIssueLabel("climax_early")).toBe("张力最高点偏早");
  });

  it("marks a limit on the reading differently from a claim about the story", () => {
    // too_few_landmarks is about how little the signals showed, not about the document,
    // so it stays neutral while the five shape notes carry colour.
    expect(structureIssueStyle("act_imbalance")).toContain("text-amber-400");
    expect(structureIssueStyle("too_few_landmarks")).toContain("text-text-muted");
    expect(structureIssueStyle("unknown_note")).toBe(structureIssueStyle("too_few_landmarks"));
  });
});

describe("time line labels", () => {
  it("names the four kinds of time expression apart", () => {
    // A reader weighs these differently: a dated day, a date missing its year, a move
    // measured from the narration, and a flavour of when.
    expect(timelineKindLabel("absolute")).toBe("绝对日期");
    expect(timelineKindLabel("partial")).toBe("缺年的日期");
    expect(timelineKindLabel("relative")).toBe("相对表达");
    expect(timelineKindLabel("vague")).toBe("模糊表达");
  });

  it("says how precisely a marker pins the moment", () => {
    expect(timelinePrecisionLabel("day")).toBe("到日");
    expect(timelinePrecisionLabel("year")).toBe("只到年");
    expect(timelinePrecisionLabel("month_day")).toBe("月日");
    expect(timelinePrecisionLabel("time_of_day")).toBe("时段");
    expect(timelinePrecisionLabel("unknown")).toBe("unknown");
  });

  it("names what the dimension found, or could not do", () => {
    expect(timelineIssueLabel("backstep")).toBe("时间倒回");
    expect(timelineIssueLabel("no_order")).toBe("无法排序");
    expect(timelineIssueLabel("unmarked_stretch")).toBe("长时间无标记");
    expect(timelineIssueLabel("unknown")).toBe("unknown");
  });

  it("separates the calendar's own finding from the limits of the reading", () => {
    // backstep is settled by the dates themselves; no_markers is about the scan.
    expect(timelineIssueStyle("backstep")).toContain("text-rose-400");
    expect(timelineIssueStyle("long_jump")).toContain("text-amber-400");
    expect(timelineIssueStyle("no_markers")).toContain("text-text-muted");
    expect(timelineIssueStyle("unknown_note")).toBe(timelineIssueStyle("no_order"));
  });

  it("says a distance the way a reader would", () => {
    expect(timelineSpanLabel(0.5)).toBe("不足一天");
    expect(timelineSpanLabel(3)).toBe("3 天");
    expect(timelineSpanLabel(60)).toBe("2 个月");
    expect(timelineSpanLabel(4899)).toBe("13.4 年");
    expect(timelineSpanLabel(-4809)).toBe("13.2 年");
  });

  it("keeps the direction a distance was measured in", () => {
    expect(timelineSignedSpan(3)).toBe("+3 天");
    expect(timelineSignedSpan(-60)).toBe("−2 个月");
    expect(timelineSignedSpan(0)).toBe("同一天");
  });
});
