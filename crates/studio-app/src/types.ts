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
}

export interface SectionInfo {
  path: string;
  block_count: number;
  char_count: number;
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
}

export interface ProjectAnalysis {
  project_id: string;
  name: string;
  t0: T0Stats;
  t1: T1Stats;
}
