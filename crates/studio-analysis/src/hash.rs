//! Deterministic fingerprint of the input an analysis run consumes.
//!
//! The analysis cache (see `studio_storage::analysis_repo`) is keyed on this
//! value: if two documents hash identically, every T0/T1/assessment number
//! derived from them is identical too, so the stored result can be returned
//! instead of recomputing.

use sha2::{Digest, Sha256};
use studio_core::document::DocumentData;

use crate::Result;

/// Bumped whenever the analysis logic changes in a way that alters its output.
///
/// The fingerprint covers the *input*, not the code that reads it, so a new
/// analysis algorithm must invalidate existing cache rows by changing this
/// salt. Bump it in the same commit that changes analysis output.
pub const ANALYSIS_VERSION: &str = "m2-v1";

/// SHA-256 (lowercase hex) of the analysed content of a document.
///
/// Each block is hashed through its JSON form, which covers every field the
/// analyzers read and keeps the fingerprint automatically in sync with the
/// document model; the block's byte length is mixed in first so block
/// boundaries cannot shift. Block order is significant — the analyzers depend
/// on it — and is preserved.
///
/// `doc_id` is deliberately *not* hashed: it is a fresh UUID on every import,
/// so including it would invalidate the cache each time the same source file is
/// re-imported.
pub fn document_hash(doc: &DocumentData) -> Result<String> {
    let mut hasher = Sha256::new();
    hasher.update(ANALYSIS_VERSION.as_bytes());
    for block in &doc.blocks {
        let json = serde_json::to_vec(block)?;
        hasher.update((json.len() as u64).to_le_bytes());
        hasher.update(&json);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use studio_core::document::SemanticBlock;

    fn block(id: &str, content: &str) -> SemanticBlock {
        SemanticBlock {
            source_block_id: id.into(),
            content: content.into(),
            section_path: "第一章".into(),
            block_type: "paragraph".into(),
            title: None,
            tokens: vec![],
            entities: vec![],
            noun_signals: vec![],
        }
    }

    fn doc(id: uuid::Uuid, blocks: Vec<SemanticBlock>) -> DocumentData {
        DocumentData {
            doc_id: id,
            title: "T".into(),
            blocks,
            total_word_count: 0,
            total_char_count: 0,
        }
    }

    #[test]
    fn test_hash_is_stable() {
        let d = doc(
            uuid::Uuid::new_v4(),
            vec![block("b1", "甲"), block("b2", "乙")],
        );
        assert_eq!(document_hash(&d).unwrap(), document_hash(&d).unwrap());
    }

    #[test]
    fn test_hash_ignores_doc_id() {
        // Re-importing the same source produces a new doc_id but the same
        // blocks; the cache must still hit.
        let a = doc(uuid::Uuid::new_v4(), vec![block("b1", "甲")]);
        let b = doc(uuid::Uuid::new_v4(), vec![block("b1", "甲")]);
        assert_eq!(document_hash(&a).unwrap(), document_hash(&b).unwrap());
    }

    #[test]
    fn test_hash_tracks_content_and_order() {
        let base = uuid::Uuid::new_v4();
        let ab = doc(base, vec![block("b1", "甲"), block("b2", "乙")]);
        let ac = doc(base, vec![block("b1", "甲"), block("b2", "丙")]);
        let ba = doc(base, vec![block("b2", "乙"), block("b1", "甲")]);
        let h = |d: &DocumentData| document_hash(d).unwrap();
        assert_ne!(h(&ab), h(&ac));
        assert_ne!(h(&ab), h(&ba));
    }

    #[test]
    fn test_hash_tracks_block_boundaries() {
        // Same total characters, different block split — must not collide.
        let base = uuid::Uuid::new_v4();
        let two = doc(base, vec![block("b1", "甲乙"), block("b2", "丙")]);
        let one = doc(base, vec![block("b1", "甲乙丙")]);
        assert_ne!(document_hash(&two).unwrap(), document_hash(&one).unwrap());
    }
}
