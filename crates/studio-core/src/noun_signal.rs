//! Noun signal model (from TraceView semantic analysis).

use serde::{Deserialize, Serialize};

/// Evidence for a noun signal detection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NounSignalEvidence {
    pub head_rel: Option<String>,
    #[serde(default)]
    pub extra: serde_json::Value,
}

/// A noun signal — a content-bearing noun phrase identified by the NLP pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NounSignal {
    pub text: String,
    pub pos: String,
    pub syntactic_role: Option<String>,
    pub score: f32,
    pub span: (usize, usize),
    pub evidence: Option<NounSignalEvidence>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_noun_signal_serde() {
        let ns = NounSignal {
            text: "analysis".to_string(),
            pos: "NN".to_string(),
            syntactic_role: Some("Subject".to_string()),
            score: 0.7,
            span: (0, 8),
            evidence: Some(NounSignalEvidence {
                head_rel: Some("dep".to_string()),
                extra: serde_json::json!({}),
            }),
        };
        let json = serde_json::to_string(&ns).unwrap();
        let deserialized: NounSignal = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.text, "analysis");
        assert_eq!(deserialized.score, 0.7);
    }
}
