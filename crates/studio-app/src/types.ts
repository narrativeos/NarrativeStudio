// Shared frontend types mirroring the Rust backend serialization.

export interface Project {
  project_id: string;
  name: string;
  genre: string | null;
  word_count: number | null;
  chapter_count: number | null;
  source_path: string | null;
  created_at: string;
  updated_at: string;
}

export interface RepeatedPhrase {
  text: string;
  count: number;
}

export interface Readability {
  score: number;
  level: string;
  method: string;
}

export interface T0Stats {
  word_count: number;
  char_count: number;
  block_count: number;
  top_words: [string, number][];
  pos_distribution: [string, number][];
  entity_counts: [string, number][];
  top_entities: [string, number][];
  top_noun_signals: [string, number][];
  noun_signal_count: number;
  avg_block_length: number;
  avg_sentence_length: number;
  sentence_count: number;
  avg_sentence_chars: number;
  sentence_length_distribution: [string, number][];
  adverb_count: number;
  adjective_count: number;
  top_adverbs: [string, number][];
  top_adjectives: [string, number][];
  top_repeated_phrases: RepeatedPhrase[];
  readability: Readability;
  /** `"nlp"` when the import carried segmented tokens, `"none"` when it did not.
   *  Every word-level number below is a measurement only in the first case. */
  token_source: string;
}

export interface SectionInfo {
  path: string;
  block_count: number;
  char_count: number;
}

export interface CharacterProfile {
  name: string;
  mentions: number;
  first_section: string;
  last_section: string;
  span_ratio: number;
  top_cooccurrences: [string, number][];
}

export interface ArcPoint {
  section: string;
  intensity: number;
  char_count: number;
}

export interface NarrativeArc {
  points: ArcPoint[];
  shape: "mountain" | "rising" | "falling" | "steady";
}

export interface T1Stats {
  sections: SectionInfo[];
  section_count: number;
  title_blocks: number;
  paragraph_blocks: number;
  list_blocks: number;
  block_type_distribution: [string, number][];
  longest_section: string | null;
  shortest_section: string | null;
  characters: CharacterProfile[];
  arc: NarrativeArc;
  entity_cooccurrences: [string, number][];
}

/** One appearance of a thread: a section, how strongly it is present there, and how
 *  many sections it had been absent for. */
export interface PlotlineStage {
  section: string;
  mentions: number;
  gap_before: number;
}

/** A plotline: one thread carried by a recurring entity or domain term. */
export interface Plotline {
  subject: string;
  /** Entity category (`PERSON`, `LOCATION`, …) or `SIGNAL` for a noun signal. */
  kind: string;
  mentions: number;
  sections_touched: number;
  /** Fraction of the document's sections this thread appears in (0-1). */
  coverage: number;
  first_section: string;
  last_section: string;
  stages: PlotlineStage[];
  /** Longest run of consecutive sections where the thread is absent. */
  longest_gap: number;
  shape: string;
  is_main: boolean;
}

/** A section where the set of active threads changes sharply. */
export interface TurningPoint {
  section: string;
  /** 0-1: how different the active threads are from the previous section. */
  change: number;
  appeared: string[];
  dropped: string[];
}

export interface PlotStats {
  plotlines: Plotline[];
  /** Threads that cleared the mention threshold, including unlisted ones. */
  thread_count: number;
  main_plotlines: string[];
  sub_plotlines: string[];
  turning_points: TurningPoint[];
  /** Threads mentioned in a single section: setup with no payoff, or noise. */
  unresolved_threads: string[];
  /** Threads first appearing in the last fifth of the document. */
  late_arrivals: string[];
  /** 0-100. */
  coherence_score: number;
  section_count: number;
  /** False when the document carries neither entities nor noun signals, i.e. a
   *  text-only import: the score would then describe the import, not the book. */
  readable: boolean;
  /** Why it cannot be read, when `readable` is false; empty otherwise. */
  note: string;
}

/** One segment of the pacing curve: what the section is made of and how tense it
 *  reads on the absolute 0-100 scale. */
export interface PacingPoint {
  section: string;
  char_count: number;
  sentence_count: number;
  avg_sentence_chars: number;
  /** 句长变化: stdev of sentence lengths inside the segment (chars). */
  sentence_stdev: number;
  /** Fraction of the segment's visible characters inside quotation marks. */
  dialogue_share: number;
  /** Entities per 1000 visible characters. */
  entity_density: number;
  /** 0-100 on the absolute scale, so two projects' curves can be compared. */
  tension: number;
}

/** A run of consecutive segments whose tension stays inside one band. */
export interface FlatStretch {
  from_section: string;
  to_section: string;
  sections: number;
  mean_tension: number;
  char_count: number;
}

/** A segment much longer than the average while less tense than the average. */
export interface DragSection {
  section: string;
  char_count: number;
  tension: number;
  /** Segment length over the mean segment length. */
  length_ratio: number;
}

export interface PacingStats {
  points: PacingPoint[];
  segment_count: number;
  /** True when the document had too few sections and the curve came from block chunks. */
  chunked: boolean;
  /** How much the tension moves — not the arc's shape, which says where the peak is. */
  verdict: "flat" | "varied" | "spiky";
  /** Standard deviation of tension across segments (0-100 scale). */
  tension_stdev: number;
  /** 句长变化 across the whole document (chars). */
  sentence_stdev: number;
  min_tension: number;
  max_tension: number;
  mean_section_chars: number;
  longest_flat_run: number;
  flat_stretches: FlatStretch[];
  drag_sections: DragSection[];
  /** 0-100. */
  pace_score: number;
}

/** One segment of the conflict curve. */
export interface ConflictPoint {
  section: string;
  char_count: number;
  /** Conflict vocabulary hits in the segment. */
  hits: number;
  /** Hits per 1000 visible characters. */
  density: number;
  /** 0-100 on the absolute scale, so two projects' curves can be compared. */
  intensity: number;
  /** The family with most hits here; empty when the segment has none. */
  dominant: string;
  /** Per-family hits; families with no hits are omitted. */
  by_type: [string, number][];
}

/** One conflict family as it appears in the whole document. */
export interface ConflictType {
  /** `interpersonal` | `internal` | `societal` | `survival`. */
  kind: string;
  hits: number;
  /** Share of all conflict hits. */
  share: number;
  /** Hits per 1000 visible characters across the document. */
  density: number;
  top_terms: [string, number][];
}

/** A run of consecutive segments carrying no conflict vocabulary at all. */
export interface ConflictAbsence {
  from_section: string;
  to_section: string;
  sections: number;
  char_count: number;
}

export interface ConflictStats {
  points: ConflictPoint[];
  segment_count: number;
  chunked: boolean;
  total_hits: number;
  /** Hits per 1000 visible characters across the document. */
  density: number;
  /** Families present, most hits first. */
  types: ConflictType[];
  /** The busiest family; empty when nothing was found. */
  dominant_type: string;
  /** `none` | `thin` | `present` | `heavy` — vocabulary coverage, not quality. */
  verdict: "none" | "thin" | "present" | "heavy";
  peak_section: string;
  peak_intensity: number;
  /** Where the peak sits in the document (0-1). */
  peak_position: number;
  /** Mean intensity over the final third of the segments. */
  tail_intensity: number;
  /** Share of all hits held by the single busiest segment. */
  concentration: number;
  absent_runs: ConflictAbsence[];
  longest_absent_run: number;
  /** 0-100, or null when the text carries no conflict vocabulary at all. */
  conflict_score: number | null;
}

/** A theme: one recurring subject and how far it reaches. */
export interface Theme {
  /** The cluster's busiest term; the cluster is named after it. */
  term: string;
  /** Terms folded into this theme, busiest first (`term, mentions`). */
  members: [string, number][];
  mentions: number;
  sections_touched: number;
  /** Fraction of the document's segments this theme appears in (0-1). */
  coverage: number;
  /** Share of every collected mention in the document. */
  share: number;
  first_section: string;
  last_section: string;
  /** `sustained` | `recurring` | `episodic` | `occasional`. */
  reach: string;
  /** Longest run of segments where the theme is absent. */
  longest_gap: number;
  /** Where its mentions sit: `spread` | `front` | `middle` | `back`. */
  concentration: string;
  /** Mentions per segment, aligned with the other curves' X axis. */
  curve: number[];
}

export interface ThemeStats {
  /** Reported themes, biggest first. */
  themes: Theme[];
  /** Themes that cleared the mention threshold, including unlisted ones. */
  candidate_count: number;
  /** Mentions of every collected term, reported or not. */
  total_mentions: number;
  /** `noun_signals` | `noun_tokens` | `none` — which vocabulary was read. */
  source: "noun_signals" | "noun_tokens" | "none";
  /** Share of all mentions held by the single biggest theme. */
  top_share: number;
  /** Share held by the top three together. */
  top3_share: number;
  /** Share of all mentions contributed by terms seen exactly once. */
  singleton_share: number;
  /** The three biggest theme names. */
  dominant_terms: string[];
  section_count: number;
  /** 0-100, or null when no usable noun vocabulary was found. */
  theme_score: number | null;
}

/** One segment of the environment curve. */
export interface SettingPoint {
  section: string;
  char_count: number;
  /** Environment vocabulary hits in the segment. */
  env_hits: number;
  /** Hits per 1000 visible characters. */
  density: number;
  /** 0-100 on the absolute scale, so two projects' curves can be compared. */
  intensity: number;
  /** Distinct places named in the segment. */
  place_count: number;
  /** The place named most often here; empty when the segment names none. */
  dominant_place: string;
}

/** One named place and how far it reaches. */
export interface Place {
  name: string;
  /** `LOCATION` or `FACILITY`. */
  kind: string;
  /** Blocks that name it. */
  mentions: number;
  sections_touched: number;
  /** Fraction of the document's segments this place appears in (0-1). */
  coverage: number;
  first_section: string;
  last_section: string;
  /** Longest run of segments where the place is absent between two appearances. */
  longest_absence: number;
  /** Times it comes back after three or more absent segments. */
  returns: number;
  /** Blocks naming it per segment, aligned with the other curves' X axis. */
  curve: number[];
}

/** A scene: a run of consecutive blocks that stays at one place. */
export interface Scene {
  /** Zero-based position in document order. */
  index: number;
  /** The segment where the scene starts. */
  section: string;
  /** The place the scene is at; empty when the text never says. */
  anchor: string;
  /** `LOCATION` / `FACILITY`, empty with no anchor. */
  anchor_kind: string;
  blocks: number;
  char_count: number;
  env_hits: number;
  /** Environment hits per 1000 visible characters in the scene. */
  env_density: number;
  /** Share of the scene's characters inside quotation marks. */
  dialogue_share: number;
  /** The scene carries a date expression — usually a time jump. */
  time_mark: boolean;
  /** People present, busiest first (`name, mentions`). */
  persons: [string, number][];
}

/** Something the setting does for the reader, or fails to do. */
export interface SettingIssue {
  /** `unanchored` | `crowded` | `long_absence` | `thin_environment`. */
  kind: string;
  section: string;
  detail: string;
}

export interface SettingStats {
  points: SettingPoint[];
  segment_count: number;
  /** True when the segments are block chunks rather than sections. */
  chunked: boolean;
  /** Reported scenes, document order. */
  scenes: Scene[];
  /** All scenes, including unreported ones. */
  scene_count: number;
  /** Scenes that name no place and describe too little to stand in for one. */
  unanchored_scenes: number;
  /** Share of scenes left without an anchor. */
  unanchored_share: number;
  places: Place[];
  /** Distinct places named anywhere in the document. */
  place_count: number;
  /** Whether places could be read at all: they come from location entities,
   *  which a text-only import does not carry. */
  places_measured: boolean;
  /** Places named in at least three blocks. */
  recurring_places: number;
  total_env_hits: number;
  /** Environment hits per 1000 visible characters across the document. */
  env_density: number;
  /** Busiest environment terms across the document, most first. */
  top_env_terms: [string, number][];
  /** `none` | `thin` | `present` | `heavy` — description vocabulary, not quality. */
  verdict: "none" | "thin" | "present" | "heavy";
  dominant_place: string;
  peak_section: string;
  peak_intensity: number;
  issues: SettingIssue[];
  longest_absence: number;
  /** 0-100, or null when the document names no place and carries no description. */
  setting_score: number | null;
}

/** One segment of the cliché curve. */
export interface ClichePoint {
  section: string;
  char_count: number;
  /** Library hits in the segment. */
  hits: number;
  /** Hits per 10 000 visible characters. */
  per_10k: number;
  /** 0-100 on the absolute scale, so two projects' curves can be compared. */
  intensity: number;
  /** The phrase used most here; empty when the segment has none. */
  top_phrase: string;
}

/** One library phrase and how hard the document leans on it. */
export interface ClichePhrase {
  /** The library pattern, e.g. `(淡淡|浅浅)一笑`. */
  phrase: string;
  /** `expression` | `body` | `emotion` | `simile` | `atmosphere` | `speech_tag` | `transition` | `look`. */
  category: string;
  count: number;
  /** Share of all hits this phrase accounts for (0-1). */
  share: number;
  /** The forms actually found, busiest first. */
  variants: [string, number][];
  /** Curve segments that contain at least one hit. */
  segments_touched: number;
  /** Share of the curve's segments this phrase reaches (0-1). */
  coverage: number;
  first_section: string;
  last_section: string;
  /** What shape the replacement should take. */
  note: string;
}

/** One place in the text where a library phrase was found. */
export interface ClicheHit {
  /** The curve segment the hit sits in. */
  section: string;
  /** The text actually matched. */
  phrase: string;
  /** The library pattern it came from. */
  pattern: string;
  category: string;
  /** Character offset inside the block. */
  offset: number;
  /** The surrounding characters, so the phrase can be judged in context. */
  excerpt: string;
  /** The hit is inside quotation marks: someone says it rather than narrates it. */
  in_dialogue: boolean;
  note: string;
}

/** A phrase habit worth fixing. */
export interface ClicheIssue {
  /** `repeated` | `dense_segment` | `one_note` | `speech_tags`. */
  kind: string;
  section: string;
  detail: string;
}

export interface ClicheStats {
  points: ClichePoint[];
  segment_count: number;
  /** True when the segments are block chunks rather than sections. */
  chunked: boolean;
  total_hits: number;
  /** Library patterns found at least once. */
  distinct_phrases: number;
  /** Visible characters the rate is measured over. */
  char_count: number;
  /** Hits per 10 000 visible characters across the document. */
  per_10k: number;
  /** `none` | `light` | `moderate` | `heavy` — library coverage, not quality. */
  verdict: "none" | "light" | "moderate" | "heavy";
  /** Categories present, most hits first. */
  categories: [string, number][];
  /** Reported phrases, most used first. */
  phrases: ClichePhrase[];
  /** Reported hits, document order. */
  hits: ClicheHit[];
  /** Hits inside quotation marks. */
  dialogue_hits: number;
  /** Phrases used three times or more. */
  repeated_phrases: number;
  /** Hits belonging to those repeated phrases. */
  repeat_hits: number;
  /** Share of all hits held by the single most used phrase (0-1). */
  top_share: number;
  top_phrase: string;
  peak_section: string;
  peak_intensity: number;
  issues: ClicheIssue[];
  /** How many patterns the library carries — coverage the reader can weigh. */
  lexicon_size: number;
  /** 0-100, or null when the text is too short for a per-10 000 rate to mean anything. */
  cliche_score: number | null;
}

/** One segment of the dialogue curve. */
export interface DialoguePoint {
  section: string;
  char_count: number;
  /** Visible characters of this segment that are spoken. */
  dialogue_chars: number;
  /** Share of the segment's visible characters that is spoken (0-1). */
  share: number;
  /** `share` on the 0-100 scale the other dimensions' curves use. */
  intensity: number;
  /** Pieces of speech found here. */
  units: number;
  /** Longest single piece of speech here. */
  longest_unit: number;
  blocks: number;
  blocks_with_dialogue: number;
}

/** One piece of speech; the longest are reported so the ratio has faces on it. */
export interface DialogueSample {
  section: string;
  chars: number;
  /** `quote` (between quotation marks) or `dash` (a line opened with ——). */
  kind: string;
  excerpt: string;
}

/** Something the mix does to the reading. */
export interface DialogueIssue {
  /** `no_dialogue` | `silent_segment` | `flooded_segment` | `monologue` |
   *  `telegraphic` | `unquoted` | `uneven`. */
  kind: string;
  section: string;
  detail: string;
}

/** One segment of the sentence-rhythm curve. */
export interface VoicePoint {
  section: string;
  char_count: number;
  sentences: number;
  avg_sentence_chars: number;
  clauses_per_sentence: number;
}

/** How often one mark appears. */
export interface VoiceMark {
  mark: string;
  label: string;
  count: number;
  per_1000: number;
}

/** One group of function words, counted by character. */
export interface VoiceWordGroup {
  group: string;
  /** The characters in the group, e.g. 的地得. */
  words: string;
  count: number;
  per_1000: number;
}

/** How often the text names who is talking. */
export interface VoicePronouns {
  first: number;
  second: number;
  third: number;
  total: number;
  first_share: number;
  second_share: number;
  third_share: number;
}

/** How firmly the text stands behind what it says. */
export interface VoiceStance {
  hedging: number;
  assertive: number;
  hedging_per_10k: number;
  assertive_per_10k: number;
  /** `hedging` | `balanced` | `assertive` — a lean, not a fault. */
  verdict: string;
}

/** A habit worth naming. */
export interface VoiceIssue {
  /** `long_sentences` | `short_sentences` | `varied_rhythm` | `even_rhythm` |
   *  `comma_run` | `short_paragraphs` | `snippet_paragraphs` | `no_pronouns` |
   *  `first_person` | `hedging` | `assertive` | `classical` | `flat_marks` | `too_short`. */
  kind: string;
  detail: string;
}

/** One thing to confirm before the text is called done. */
export interface ChecklistItem {
  /** Stable key; the tick is stored under it. */
  id: string;
  /** `consistency` | `structure` | `language` | `advisory`. */
  group: string;
  label: string;
  /** The number behind the item — an item is never shown without one. */
  detail: string;
  /** `high` | `medium` | `low`: where the item sits in its group, not a fault. */
  severity: string;
}

/** The final review checklist. No score: an item is a question, not a grade. */
export interface ReviewChecklist {
  items: ChecklistItem[];
  /** Counts per group, always all four in the fixed order. */
  groups: [string, number][];
}

/** Author voice. There is no score: a long sentence is not better than a short one. */
export interface VoiceStats {
  points: VoicePoint[];
  segment_count: number;
  /** True when the segments are block chunks rather than sections. */
  chunked: boolean;
  char_count: number;
  sentence_count: number;
  avg_sentence_chars: number;
  sentence_stdev: number;
  /** `sentence_stdev / avg_sentence_chars` — the rhythm's spread, scale-free. */
  sentence_cv: number;
  shortest_sentence: number;
  longest_sentence: number;
  clause_count: number;
  avg_clause_chars: number;
  clauses_per_sentence: number;
  paragraph_count: number;
  avg_paragraph_chars: number;
  paragraph_stdev: number;
  single_sentence_paragraphs: number;
  marks: VoiceMark[];
  function_words: VoiceWordGroup[];
  pronouns: VoicePronouns;
  stance: VoiceStance;
  issues: VoiceIssue[];
}

export interface DialogueStats {
  points: DialoguePoint[];
  segment_count: number;
  /** True when the segments are block chunks rather than sections. */
  chunked: boolean;
  /** Visible characters the shares are measured over. Quotation marks themselves
   *  fall on the narrative side: they frame speech rather than speak it. */
  char_count: number;
  dialogue_chars: number;
  narrative_chars: number;
  /** 0-1. */
  dialogue_share: number;
  /** 0-1. */
  narrative_share: number;
  /** Pieces of speech found. */
  unit_count: number;
  quoted_units: number;
  dash_units: number;
  avg_unit_chars: number;
  longest_unit: number;
  /** Piece lengths in fixed buckets, short to long. */
  unit_length_distribution: [string, number][];
  blocks: number;
  blocks_with_dialogue: number;
  /** Longest run of consecutive speech-dominant blocks. */
  longest_dialogue_run: number;
  /** Longest run of consecutive blocks carrying no speech at all. */
  longest_quiet_run: number;
  /** Visible characters inside that longest quiet run. */
  longest_quiet_chars: number;
  silent_segments: number;
  flooded_segments: number;
  /** `none` | `sparse` | `balanced` | `heavy` — the mix, not its quality. */
  verdict: "none" | "sparse" | "balanced" | "heavy";
  /** Highest segment share minus lowest, over segments with enough text. */
  spread: number;
  peak_section: string;
  peak_share: number;
  quietest_section: string;
  quietest_share: number;
  /** The longest pieces of speech, longest first. */
  samples: DialogueSample[];
  issues: DialogueIssue[];
}

/** One segment of the explicit-language curve. */
export interface ExplicitPoint {
  section: string;
  char_count: number;
  /** Library hits in the segment. */
  hits: number;
  /** Hits per 10 000 visible characters in this segment. */
  per_10k: number;
  /** 0-100 on the absolute scale, so two projects' curves can be compared. */
  intensity: number;
  /** The term used most here; empty when the segment has none. */
  top_term: string;
}

/** One library pattern that matched at least once, with how often and where. */
export interface ExplicitTerm {
  /** The library pattern, e.g. `强奸|性侵|轮奸`. */
  term: string;
  /** `profanity` | `sexual` | `sexual_violence` | `violence` | `self_harm` | `hate` | `substance`. */
  category: string;
  /** `mild` | `moderate` | `strong` — the strength of the wording, not of the event. */
  tier: string;
  count: number;
  /** Share of all hits this term accounts for (0-1). */
  share: number;
  /** The forms actually found, busiest first. */
  variants: [string, number][];
  /** Curve segments that contain at least one hit. */
  segments_touched: number;
  /** Share of the curve's segments this term reaches (0-1). */
  coverage: number;
  first_section: string;
  last_section: string;
  /** What the library advises about it. */
  note: string;
}

/** One place in the text where a library term was found. */
export interface ExplicitHit {
  /** The curve segment the hit sits in. */
  section: string;
  /** The text actually matched. */
  term: string;
  /** The library pattern it came from. */
  pattern: string;
  category: string;
  tier: string;
  /** Character offset inside the block. */
  offset: number;
  /** The surrounding characters, so the hit can be judged in context. */
  excerpt: string;
  /** The hit is inside quotation marks: a character says it rather than narrates it. */
  in_dialogue: boolean;
  note: string;
}

/** A family the text carries enough of to name in a reader notice. */
export interface ExplicitIssue {
  /** `self_harm` | `sexual_violence` | `sexual_content` | `gore` | `hate_slur` |
   *  `substance` | `dense_segment`. */
  kind: string;
  section: string;
  detail: string;
}

export interface ExplicitStats {
  points: ExplicitPoint[];
  segment_count: number;
  /** True when the segments are block chunks rather than sections. */
  chunked: boolean;
  /** Visible characters the rate is measured over. */
  char_count: number;
  total_hits: number;
  /** Library patterns found at least once. */
  distinct_terms: number;
  /** Hits per 10 000 visible characters across the document. */
  per_10k: number;
  /** `none` | `mild` | `moderate` | `strong` — the strength of the words found,
   *  not a judgment on the text. */
  advisory: "none" | "mild" | "moderate" | "strong";
  /** Hit counts per tier, always all three tiers in this order. */
  tiers: [string, number][];
  /** Categories present, most hits first. */
  categories: [string, number][];
  /** Categories that carry at least one `strong` hit: what an advisory would name. */
  advisory_categories: string[];
  /** Reported terms, most used first. */
  terms: ExplicitTerm[];
  /** Reported hits, document order. */
  hits: ExplicitHit[];
  /** Hits inside quotation marks. */
  dialogue_hits: number;
  /** Share of all hits held by the single most used term (0-1). */
  top_share: number;
  top_term: string;
  peak_section: string;
  peak_intensity: number;
  issues: ExplicitIssue[];
  /** How many patterns the library carries — coverage the reader can weigh. */
  lexicon_size: number;
  /** How many surface forms those patterns expand to. */
  lexicon_variants: number;
}

/** A place where the document's own signals say something happens. */
export interface StructureLandmark {
  /** `opening` | `turn` | `peak` | `trough` | `climax`. */
  kind: string;
  /** The curve segment the landmark sits in. */
  section: string;
  /** Document fraction (0-1), by character mass. */
  position: number;
  /** How strong the signal is: thread change (0-1) for a turn, tension (0-100) otherwise. */
  strength: number;
  /** What the signals show there. */
  detail: string;
}

/** One beat a model expects, and what the document puts there. */
export interface StructureBeat {
  beat: string;
  /** Where the model expects it, as document fractions. */
  window: [number, number];
  /** `found` | `displaced` | `missing`. */
  status: string;
  /** Where the supporting landmark is; empty when the beat is 未见. */
  section: string;
  /** Position of that landmark (0-1); 0 when missing. */
  position: number;
  /** How far outside the window the nearest landmark sits (0 when inside). */
  offset: number;
  /** Which landmark answers the beat, in words. */
  evidence: string;
}

/** One classic shape tried against the document. */
export interface StructureModel {
  /** `three_act` | `kisho_tenketsu` | `hero_journey`. */
  model: string;
  /** `matched` | `partial` | `unclear` — how many beats land where expected. */
  fit: string;
  beats: StructureBeat[];
  found: number;
  displaced: number;
  missing: number;
}

/** One act of the document's own three-way split, cut at its two strongest turns. */
export interface StructureAct {
  /** `act_one` | `act_two` | `act_three`. */
  name: string;
  from_section: string;
  to_section: string;
  char_count: number;
  /** Share of the document's visible characters (0-1). */
  share: number;
  mean_tension: number;
  min_tension: number;
  max_tension: number;
}

/** Something the shape does that a writer would want to know about. */
export interface StructureIssue {
  /** `no_early_turn` | `missing_midpoint` | `climax_early` | `no_descent` |
   *  `act_imbalance` | `too_few_landmarks`. */
  kind: string;
  section: string;
  detail: string;
}

/**
 * Story structure analysis. There is no score field: the dimension describes where
 * the document's own signals place a shape, it does not grade the shape.
 */
export interface StructureStats {
  /** The landmarks the matching worked from, reading order. */
  landmarks: StructureLandmark[];
  /** The document's own three-way split; empty when fewer than two turns were found. */
  acts: StructureAct[];
  /** The shapes tried against the document; empty when `readable` is false. */
  models: StructureModel[];
  issues: StructureIssue[];
  segment_count: number;
  /** True when the segments are block chunks rather than sections. */
  chunked: boolean;
  char_count: number;
  /** Turns among the landmarks. */
  turn_count: number;
  /** False when the document is too short, or has no tension curve to place beats on. */
  readable: boolean;
  /** Why `readable` is false, in words; empty when it is true. */
  note: string;
}

/** One time expression found in the text. */
export interface TimelineMarker {
  /** The characters found, e.g. `2003年3月12日`. */
  text: string;
  /** `absolute` | `partial` | `relative` | `vague`. */
  kind: string;
  /** `year` | `month` | `day` | `month_day` | `offset` | `deictic` | `season` |
   *  `time_of_day` | `weekday` | `solar_term` | `festival` | `era_year` | `era_day` |
   *  `ganzhi_year` | `ganzhi_day` | `lunar_month` | `lunar_month_day` | `lunar_day`. */
  precision: string;
  /** The segment the marker sits in, on the shared curve axis. */
  section: string;
  /** Document fraction at the marker (0-1). */
  position: number;
  /** The calendar date as `2003-03-12` / `2003-03` / `2003`; a 年号 or 干支 year as the
   *  Gregorian year it resolves to (`乾隆三年` → `1738`); empty unless the expression
   *  carries a year the analysis could place. */
  resolved: string;
  /** Estimated signed distance in days for a relative marker (月≈30天, 年≈365天). */
  offset_days: number;
  /** Surrounding characters, with ellipses where they were cut. */
  excerpt: string;
}

/** One step between two resolvable dates, in reading order. */
export interface TimelineStep {
  from_section: string;
  from_date: string;
  to_section: string;
  to_date: string;
  /** Days between them: positive forward, negative when the story goes back. */
  days: number;
}

/** A finding about the document's timekeeping. */
export interface TimelineIssue {
  /** `no_markers` | `too_short` | `no_order` | `backstep` | `long_jump` |
   *  `unmarked_stretch` | `unanchored_ganzhi` | `ambiguous_era` | `calendar_note` |
   *  `term_collision`. */
  kind: string;
  section: string;
  detail: string;
}

/** Marker count for one segment of the shared curve axis. */
export interface TimelinePoint {
  label: string;
  markers: number;
}

/**
 * Time-line analysis. There is no score field: the dimension describes how a document
 * keeps time rather than grading how well it does.
 */
export interface TimelineStats {
  /** The expressions found, reading order. */
  markers: TimelineMarker[];
  /** Marker counts per segment, on the same axis as the pacing and conflict curves. */
  curve: TimelinePoint[];
  absolute_count: number;
  partial_count: number;
  relative_count: number;
  vague_count: number;
  /** How the expressions are written, most common first: `day`, `month_day`, `offset`,
   *  `season`, and the calendar-table forms — `era_year`, `ganzhi_year`,
   *  `lunar_month_day`, `solar_term`, `festival`. A form is listed only when the text
   *  really used it. */
  forms: [string, number][];
  /** `DATE` entities the import carries, which the scan does not read. The gap between
   *  this and the counts above is dates written in a form the scan cannot read. */
  entity_date_count: number;
  first_date: string;
  last_date: string;
  /** Days between the first and last resolvable date; 0 when fewer than two. */
  span_days: number;
  /** Steps between resolvable dates that go backwards in the calendar. */
  backsteps: TimelineStep[];
  /** The largest forward steps. */
  jumps: TimelineStep[];
  /** Runs of segments carrying no marker: [first label, last label, how many]. */
  unmarked_stretches: [string, string, number][];
  issues: TimelineIssue[];
  segment_count: number;
  /** True when the segments are block chunks rather than sections. */
  chunked: boolean;
  char_count: number;
  /** False when the text says nothing about when, or is too short to compare. */
  readable: boolean;
  /** Why `readable` is false, in words; empty when it is true. */
  note: string;
}

export interface DimensionScore {
  key: string;
  label: string;
  score: number;
  detail: string;
}

export interface Recommendation {
  severity: "high" | "medium" | "low";
  dimension: string;
  title: string;
  detail: string;
}

export interface Assessment {
  overall: number;
  dimensions: DimensionScore[];
  recommendations: Recommendation[];
}

/** A generated Markdown report. The body comes back with the metadata so the UI
 *  can suggest a filename and show what is about to be written. */
export interface ReportMeta {
  report_id: string;
  project_id: string;
  title: string;
  created_at: string;
  markdown: string;
  exported_path: string | null;
}

export interface ProjectAnalysis {
  project_id: string;
  name: string;
  t0: T0Stats;
  t1: T1Stats;
  plot: PlotStats;
  pacing: PacingStats;
  conflict: ConflictStats;
  theme: ThemeStats;
  setting: SettingStats;
  cliche: ClicheStats;
  dialogue: DialogueStats;
  explicit: ExplicitStats;
  structure: StructureStats;
  timeline: TimelineStats;
  voice: VoiceStats;
  /** The numbers that still need a decision, gathered from the dimensions above. */
  checklist: ReviewChecklist;
  assessment: Assessment;
  /** True when the numbers were read from the persisted cache. */
  cached: boolean;
}
