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

export interface ProjectAnalysis {
  project_id: string;
  name: string;
  t0: T0Stats;
  t1: T1Stats;
  assessment: Assessment;
}
