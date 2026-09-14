//! Analysis dimension and result types.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::concern::Concern;

/// All supported analysis dimensions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisDimension {
    OverallAssessment,
    PointsOfConcern,
    NarrativeArc,
    PlotAnalysis,
    StoryElements,
    CharacterAnalysis,
    PacingAnalysis,
    ConflictAnalysis,
    ThemeAnalysis,
    SettingAnalysis,
    AuthorVoice,
    DialogueNarrative,
    Readability,
    ClichesFinder,
    ExplicitLanguage,
    RepetitivePhrases,
    AdverbsAdjectives,
    MisspellingsFinder,
    KeyRecommendations,
    StoryStructure,
}

impl AnalysisDimension {
    /// Get the string key for this dimension.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::OverallAssessment => "overall_assessment",
            Self::PointsOfConcern => "points_of_concern",
            Self::NarrativeArc => "narrative_arc",
            Self::PlotAnalysis => "plot_analysis",
            Self::StoryElements => "story_elements",
            Self::CharacterAnalysis => "character_analysis",
            Self::PacingAnalysis => "pacing_analysis",
            Self::ConflictAnalysis => "conflict_analysis",
            Self::ThemeAnalysis => "theme_analysis",
            Self::SettingAnalysis => "setting_analysis",
            Self::AuthorVoice => "author_voice",
            Self::DialogueNarrative => "dialogue_narrative",
            Self::Readability => "readability",
            Self::ClichesFinder => "cliches_finder",
            Self::ExplicitLanguage => "explicit_language",
            Self::RepetitivePhrases => "repetitive_phrases",
            Self::AdverbsAdjectives => "adverbs_adjectives",
            Self::MisspellingsFinder => "misspellings_finder",
            Self::KeyRecommendations => "key_recommendations",
            Self::StoryStructure => "story_structure",
        }
    }
}

/// Task execution tier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskTier {
    /// Pure statistics — deterministic, < 100ms, no LLM.
    T0,
    /// Structural analysis — deterministic, < 2s, no LLM.
    T1,
    /// LLM-enhanced — non-deterministic, 5-60s, optional.
    T2,
}

/// Options for running analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisOptions {
    /// Which LLM tier to use.
    #[serde(default)]
    pub llm_tier: LlmTier,
    /// Specific dimensions to analyze (empty = all).
    #[serde(default)]
    pub dimensions: Vec<AnalysisDimension>,
}

impl Default for AnalysisOptions {
    fn default() -> Self {
        Self {
            llm_tier: LlmTier::None,
            dimensions: vec![],
        }
    }
}

/// LLM usage tier.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LlmTier {
    /// No LLM — pure local analysis.
    #[default]
    None,
    /// Quick AI — 3 LLM tasks, ~20s.
    Quick,
    /// Deep AI — 8 LLM tasks, ~60s.
    Deep,
}

/// Result of a single dimension analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    pub dimension: AnalysisDimension,
    pub score: Option<f32>,
    pub data: serde_json::Value,
    pub concerns: Vec<Concern>,
    pub computed_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dimension_as_str() {
        assert_eq!(AnalysisDimension::Readability.as_str(), "readability");
        assert_eq!(AnalysisDimension::NarrativeArc.as_str(), "narrative_arc");
    }

    #[test]
    fn test_dimension_serde() {
        let dim = AnalysisDimension::PacingAnalysis;
        let json = serde_json::to_string(&dim).unwrap();
        assert_eq!(json, "\"pacing_analysis\"");
        let deserialized: AnalysisDimension = serde_json::from_str(&json).unwrap();
        assert_eq!(dim, deserialized);
    }

    #[test]
    fn test_analysis_options_default() {
        let opts = AnalysisOptions::default();
        assert_eq!(opts.llm_tier, LlmTier::None);
        assert!(opts.dimensions.is_empty());
    }
}
