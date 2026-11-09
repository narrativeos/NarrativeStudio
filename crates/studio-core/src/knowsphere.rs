//! Mirror of the NarratSphere `.knowsphere.json` v1.0 document contract.
//!
//! This is the **export contract** between NarrativeStudio (the data-building
//! layer) and NarratSphere (the aggregation/consumption layer). It mirrors the
//! closed vocabulary of the schema — node types, edge types, narrative types —
//! so an exported file can be validated *before* it is written, instead of
//! relying on the consumer to reject it later.
//!
//! Deliberate omissions:
//! - `layers` is left out of the export: omitting it selects NarratSphere's
//!   built-in six layers, keeping layer identity (names, radii, colors) a
//!   single source of truth on the consumer side.
//! - `view` is left out: camera state is session data, not content.
//! - node `position` is never emitted: layout is NarratSphere's job (it packs
//!   nodes deterministically on each shell). Narrative elements *must* carry a
//!   cartesian position because the schema requires it; the exporter places
//!   them deterministically inside the atmosphere band.

use serde::{Deserialize, Serialize};

/// Schema version this crate emits. Bump together with a documented change.
pub const KNOWSPHERE_VERSION: &str = "1.0";

/// Atmosphere band: narrative elements float between these distances (in
/// sphere radii) above the centre. Mirrors NarratSphere's `ATMOSPHERE`.
pub const ATMOSPHERE_MIN: f64 = 13.0;
pub const ATMOSPHERE_MAX: f64 = 26.0;

/// Closed node vocabulary (data-model.md §Nodes).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeType {
    Concept,
    Entity,
    Fact,
    Rule,
    Axiom,
    Application,
    Observation,
}

/// Closed edge vocabulary (data-model.md §Edges).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeType {
    Mapping,
    Support,
    Inheritance,
    Association,
    NarrativeLink,
}

/// Closed narrative-element vocabulary with the fixed semantics NarratSphere
/// assigns: tethered_balloon = strong link, free_balloon = weak link,
/// cloud = medium-weak, tornado = dynamic-strong, rain_band = sustained
/// feedback, satellite = distant macro.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NarrativeType {
    TetheredBalloon,
    FreeBalloon,
    Cloud,
    Tornado,
    RainBand,
    Satellite,
}

impl NarrativeType {
    /// Base altitude for the type: heavier meanings sit lower, nearer what
    /// they explain (data-model.md §Narrative elements).
    pub fn base_altitude(self) -> f64 {
        match self {
            Self::TetheredBalloon => 14.0,
            Self::Cloud => 15.5,
            Self::FreeBalloon => 17.0,
            Self::Tornado => 19.0,
            Self::RainBand => 21.0,
            Self::Satellite => 23.5,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeAttributes {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    /// Relative importance in [0, 1]; drives marker size on the consumer side.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeNode {
    pub id: String,
    pub label: String,
    /// 1–5. Layer 6 rejects nodes, so the exporter must never emit 6 here.
    pub layer_id: u8,
    #[serde(rename = "type")]
    pub node_type: NodeType,
    /// Never emitted: layout belongs to NarratSphere.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attributes: Option<NodeAttributes>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EdgeAttributes {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strength: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeEdge {
    pub id: String,
    pub source: String,
    pub target: String,
    #[serde(rename = "type")]
    pub edge_type: EdgeType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub directed: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub layer_scope: Option<LayerScope>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attributes: Option<EdgeAttributes>,
}

/// Derived from the endpoints' layers; the exporter omits it and lets the
/// consumer derive it, so the file can never disagree with the scene.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LayerScope {
    Intra,
    Cross,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CartesianPosition {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NarrativeAttributes {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub story: Option<String>,
    /// How firmly the element is anchored, [0, 1]. High = it barely drifts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stability: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Narrative {
    pub id: String,
    #[serde(rename = "type")]
    pub narrative_type: NarrativeType,
    pub label: String,
    /// Required by the schema; must lie inside the atmosphere band.
    pub position: CartesianPosition,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linked_nodes: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linked_layers: Option<Vec<u8>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attributes: Option<NarrativeAttributes>,
}

/// The exported document. `layers` and `view` are intentionally absent.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowsphereDoc {
    pub version: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// ISO-8601.
    pub created: String,
    pub nodes: Vec<KnowledgeNode>,
    pub edges: Vec<KnowledgeEdge>,
    pub narratives: Vec<Narrative>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Map<String, serde_json::Value>>,
}

impl KnowsphereDoc {
    /// Self-check against the schema rules the exporter can still see.
    /// Returns human-readable issues; an empty vec means the file is safe to
    /// write. Mirrors the checks NarratSphere's `validate.ts` would report.
    pub fn validate(&self) -> Vec<String> {
        use std::collections::HashSet;
        let mut issues = Vec::new();

        if self.version != KNOWSPHERE_VERSION {
            issues.push(format!(
                "version: expected \"{KNOWSPHERE_VERSION}\", got \"{}\"",
                self.version
            ));
        }
        if self.name.trim().is_empty() {
            issues.push("name: empty".into());
        }

        let mut node_ids = HashSet::new();
        for n in &self.nodes {
            if !node_ids.insert(n.id.as_str()) {
                issues.push(format!("nodes.id.duplicate: {}", n.id));
            }
            if !(1..=5).contains(&n.layer_id) {
                issues.push(format!(
                    "nodes.layerId: {} has layerId {} (must be 1-5)",
                    n.id, n.layer_id
                ));
            }
            if let Some(w) = n.attributes.as_ref().and_then(|a| a.weight) {
                if !(0.0..=1.0).contains(&w) {
                    issues.push(format!("nodes.weight: {} is {w} (must be in [0, 1])", n.id));
                }
            }
        }

        let mut edge_ids = HashSet::new();
        for e in &self.edges {
            if !edge_ids.insert(e.id.as_str()) {
                issues.push(format!("edges.id.duplicate: {}", e.id));
            }
            if !node_ids.contains(e.source.as_str()) {
                issues.push(format!(
                    "edges.source: {} points at missing node {}",
                    e.id, e.source
                ));
            }
            if !node_ids.contains(e.target.as_str()) {
                issues.push(format!(
                    "edges.target: {} points at missing node {}",
                    e.id, e.target
                ));
            }
            if let Some(s) = e.attributes.as_ref().and_then(|a| a.strength) {
                if !(0.0..=1.0).contains(&s) {
                    issues.push(format!(
                        "edges.strength: {} is {s} (must be in [0, 1])",
                        e.id
                    ));
                }
            }
        }

        let mut narr_ids = HashSet::new();
        for n in &self.narratives {
            if !narr_ids.insert(n.id.as_str()) {
                issues.push(format!("narratives.id.duplicate: {}", n.id));
            }
            let d = (n.position.x * n.position.x
                + n.position.y * n.position.y
                + n.position.z * n.position.z)
                .sqrt();
            if !(ATMOSPHERE_MIN..=ATMOSPHERE_MAX).contains(&d) {
                issues.push(format!(
                    "narratives.altitude: {} is at {d:.2} \
                     (must be in [{ATMOSPHERE_MIN}, {ATMOSPHERE_MAX}])",
                    n.id
                ));
            }
            if let Some(s) = n.attributes.as_ref().and_then(|a| a.stability) {
                if !(0.0..=1.0).contains(&s) {
                    issues.push(format!(
                        "narratives.stability: {} is {s} (must be in [0, 1])",
                        n.id
                    ));
                }
            }
            if let Some(links) = &n.linked_nodes {
                for l in links {
                    if !node_ids.contains(l.as_str()) {
                        issues.push(format!(
                            "narratives.linkedNodes.dangling: {} links to missing node {l}",
                            n.id
                        ));
                    }
                }
            }
        }

        issues
    }

    /// Deterministic serialization: two-space indent, trailing newline —
    /// matching NarratSphere's `serializeProject` convention so repeated
    /// exports of the same analysis produce byte-identical files.
    pub fn serialize(&self) -> String {
        let mut text =
            serde_json::to_string_pretty(self).expect("knowsphere doc is serializable by shape");
        text.push('\n');
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc() -> KnowsphereDoc {
        KnowsphereDoc {
            version: KNOWSPHERE_VERSION.into(),
            name: "测试知识体".into(),
            description: None,
            created: "2026-01-01T08:30:00+00:00".into(),
            nodes: vec![KnowledgeNode {
                id: "ent:abc".into(),
                label: "马洛".into(),
                layer_id: 2,
                node_type: NodeType::Entity,
                attributes: Some(NodeAttributes {
                    description: Some("出现 42 次".into()),
                    tags: Some(vec!["PERSON".into()]),
                    weight: Some(0.8),
                }),
            }],
            edges: vec![KnowledgeEdge {
                id: "edg:def".into(),
                source: "ent:abc".into(),
                target: "ent:abc".into(),
                edge_type: EdgeType::Association,
                directed: None,
                layer_scope: None,
                attributes: Some(EdgeAttributes {
                    strength: Some(0.5),
                    label: None,
                }),
            }],
            narratives: vec![Narrative {
                id: "nrr:ghi".into(),
                narrative_type: NarrativeType::Tornado,
                label: "第三章 · 转折".into(),
                position: CartesianPosition {
                    x: 12.0,
                    y: 11.0,
                    z: 0.0,
                },
                linked_nodes: Some(vec!["ent:abc".into()]),
                linked_layers: None,
                attributes: Some(NarrativeAttributes {
                    story: Some("转折出现".into()),
                    stability: Some(0.4),
                }),
            }],
            metadata: None,
        }
    }

    #[test]
    fn valid_doc_has_no_issues() {
        assert_eq!(doc().validate(), Vec::<String>::new());
    }

    #[test]
    fn catches_bad_layer_version_and_dangling_links() {
        let mut d = doc();
        d.version = "2.0".into();
        d.nodes[0].layer_id = 6;
        d.narratives[0].linked_nodes = Some(vec!["ent:missing".into()]);
        let issues = d.validate();
        assert!(issues.iter().any(|i| i.starts_with("version")));
        assert!(issues.iter().any(|i| i.starts_with("nodes.layerId")));
        assert!(issues
            .iter()
            .any(|i| i.starts_with("narratives.linkedNodes.dangling")));
    }

    #[test]
    fn catches_out_of_band_narrative() {
        let mut d = doc();
        d.narratives[0].position = CartesianPosition {
            x: 2.0,
            y: 2.0,
            z: 2.0,
        };
        assert!(d
            .validate()
            .iter()
            .any(|i| i.starts_with("narratives.altitude")));
    }

    #[test]
    fn serializes_camel_case_and_round_trips() {
        let text = doc().serialize();
        assert!(text.contains("\"layerId\": 2"));
        assert!(text.contains("\"linkedNodes\""));
        assert!(text.ends_with("}\n"));
        let back: KnowsphereDoc = serde_json::from_str(&text).unwrap();
        assert_eq!(back.nodes.len(), 1);
        assert_eq!(back.nodes[0].node_type, NodeType::Entity);
        assert_eq!(back.narratives[0].narrative_type, NarrativeType::Tornado);
        assert_eq!(back.edges[0].edge_type, EdgeType::Association);
    }

    #[test]
    fn omits_layers_view_and_node_positions() {
        let text = doc().serialize();
        assert!(!text.contains("\"layers\""));
        assert!(!text.contains("\"view\""));
        assert!(!text.contains("\"radiusOffset\""));
        // Only narratives carry a position; nodes never do.
        assert_eq!(text.matches("\"position\"").count(), 1);
    }
}
