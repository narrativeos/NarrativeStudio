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
  assessment: Assessment;
  /** True when the numbers were read from the persisted cache. */
  cached: boolean;
}
