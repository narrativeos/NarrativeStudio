//! Knowsphere export — the machine-readable product for NarratSphere.
//!
//! `render_knowsphere` is a pure function over the same analysis output the
//! Markdown report uses, plus the project's aggregated entity identities. It
//! implements the documented mapping (product-design.md §Knowsphere 导出契约):
//!
//! | Studio data                       | Knowsphere landing                        |
//! |-----------------------------------|-------------------------------------------|
//! | entity identity (kept)            | L2 `entity` node, weight ← frequency      |
//! | theme                             | L3 `concept` node; sky `cloud` element    |
//! | place                             | L2 `entity` node (merged by name)         |
//! | timeline span                     | one L2 `fact` node                        |
//! | recommendation                    | L2 `observation` node, severity tag       |
//! | character co-occurrence           | `association` edge, strength ← count      |
//! | plotline (sustained main)         | sky `rain_band` (sustained feedback)      |
//! | plotline (main)                   | sky `tethered_balloon` (strong link)      |
//! | plotline (subplot shapes)         | sky `satellite` (distant macro)           |
//! | plotline (setup only)             | sky `free_balloon` (weak link)            |
//! | turning point                     | sky `tornado` (dynamic strong)            |
//!
//! Like the Markdown renderer this is deterministic: the same analysis and
//! the same timestamp produce byte-identical JSON. Node `position` is never
//! emitted — layout is NarratSphere's job; only narrative elements carry the
//! position the schema requires, placed deterministically in the sky.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};

use studio_analysis::{Assessment, PlotStats, SettingStats, T1Stats, ThemeStats, TimelineStats};
use studio_core::entity::EntityStat;
use studio_core::knowsphere::{
    CartesianPosition, EdgeAttributes, EdgeType, KnowledgeEdge, KnowledgeNode, KnowsphereDoc,
    Narrative, NarrativeAttributes, NarrativeType, NodeType, KNOWSPHERE_VERSION,
};

/// Everything a knowsphere export is rendered from: the same values the
/// analysis page shows (cache-valid rows included), so the export always
/// matches what the user saw. Deliberately narrower than the Markdown
/// report's input — only the dimensions that map onto the sphere.
pub struct KnowsphereInput<'a> {
    pub project_name: &'a str,
    pub project_id: &'a str,
    pub generated_at: DateTime<Utc>,
    /// SHA-256 of the analysed content (`studio_analysis::document_hash`).
    pub source_hash: &'a str,
    pub t1: &'a T1Stats,
    pub plot: &'a PlotStats,
    pub theme: &'a ThemeStats,
    pub setting: &'a SettingStats,
    pub timeline: &'a TimelineStats,
    pub assessment: &'a Assessment,
    /// Per-identity entity aggregation for the whole project.
    pub entities: &'a [EntityStat],
}

/// Fold a label for dedup: trim, lowercase, collapse whitespace. Mirrors the
/// identity folding in the entity registry, on the consumer side of the map.
fn fold(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// 16 hex of SHA-256 — the id suffix for content-derived identities that
/// the registry does not issue (themes, observations, edges, narratives).
fn sha16(key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(16);
    for byte in &digest[..8] {
        hex.push_str(&format!("{byte:02x}"));
    }
    hex
}

/// Accumulates nodes and answers "is this label already on a shell". First
/// spelling of a label wins: an entity that is also a theme stays one node,
/// which is exactly what the tether resolution needs.
#[derive(Default)]
struct NodeBook {
    nodes: Vec<KnowledgeNode>,
    by_label: HashMap<String, String>,
}

impl NodeBook {
    fn add(
        &mut self,
        id: String,
        label: String,
        layer_id: u8,
        node_type: NodeType,
        attrs: NodeAttributesBag,
    ) -> String {
        let key = fold(&label);
        if let Some(existing) = self.by_label.get(&key) {
            return existing.clone();
        }
        self.by_label.insert(key, id.clone());
        self.nodes.push(KnowledgeNode {
            id: id.clone(),
            label,
            layer_id,
            node_type,
            attributes: Some(attrs.into_bag()),
        });
        id
    }

    fn find(&self, label: &str) -> Option<String> {
        self.by_label.get(&fold(label)).cloned()
    }
}

/// Small builder hop so `add` stays readable.
struct NodeAttributesBag {
    weight: Option<f64>,
    tags: Vec<String>,
    description: Option<String>,
}

impl NodeAttributesBag {
    fn into_bag(self) -> studio_core::knowsphere::NodeAttributes {
        studio_core::knowsphere::NodeAttributes {
            description: self.description,
            tags: Some(self.tags),
            weight: self.weight.map(|w| w.clamp(0.0, 1.0)),
        }
    }
}

/// Golden angle: successive elements spread evenly around the vertical axis.
const GOLDEN_ANGLE: f64 = 2.399_963_229_728_653;

/// Deterministic sky placement: radius from the element type's base altitude,
/// polar angle stepping through a small band above the shells, azimuth on the
/// golden angle. The distance from the centre is exactly the radius, so every
/// element lands inside the atmosphere band by construction.
fn sky_position(index: usize, kind: NarrativeType) -> CartesianPosition {
    let radius = kind.base_altitude();
    let phi = 0.7 + 0.15 * ((index % 4) as f64);
    let theta = index as f64 * GOLDEN_ANGLE;
    CartesianPosition {
        x: radius * phi.sin() * theta.cos(),
        y: radius * phi.cos(),
        z: radius * phi.sin() * theta.sin(),
    }
}

fn clamp01(value: f64) -> f64 {
    value.clamp(0.0, 1.0)
}

/// Render the project into a `.knowsphere.json` document.
pub fn render_knowsphere(input: &KnowsphereInput) -> studio_core::Result<KnowsphereDoc> {
    let mut book = NodeBook::default();

    // --- L2 surface: entities, places, the time span, recommendations -------
    let max_mentions = input
        .entities
        .iter()
        .map(|e| e.mentions)
        .max()
        .unwrap_or(1)
        .max(1);
    for entity in input.entities {
        // Dates and numbers are carried by the time line and the text
        // statistics; generic UNKNOWN labels would only be noise on a shell.
        if !matches!(
            entity.category.as_str(),
            "PERSON" | "ORGANIZATION" | "LOCATION" | "FACILITY" | "PRODUCT"
        ) {
            continue;
        }
        book.add(
            entity.stable_id.clone(),
            entity.text.clone(),
            2,
            NodeType::Entity,
            NodeAttributesBag {
                weight: Some(clamp01(
                    0.1 + 0.9 * (entity.mentions as f64 / max_mentions as f64),
                )),
                tags: vec![entity.category.clone(), "studio:entity".into()],
                description: Some(format!("出现 {} 次", entity.mentions)),
            },
        );
    }
    for place in &input.setting.places {
        // A place that is also an entity of the same name is the same node.
        book.add(
            format!(
                "ent:{}",
                sha16(&format!("{}\u{1f}{}", place.kind, fold(&place.name)))
            ),
            place.name.clone(),
            2,
            NodeType::Entity,
            NodeAttributesBag {
                weight: Some(clamp01(place.coverage)),
                tags: vec![place.kind.clone(), "studio:place".into()],
                description: Some(format!("提及 {} 次", place.mentions)),
            },
        );
    }
    if !input.timeline.first_date.is_empty() {
        book.add(
            "tln:span".into(),
            format!(
                "时间跨度 {} → {}",
                input.timeline.first_date, input.timeline.last_date
            ),
            2,
            NodeType::Fact,
            NodeAttributesBag {
                weight: Some(0.5),
                tags: vec!["studio:timeline".into()],
                description: Some(format!("跨度 {} 天", input.timeline.span_days)),
            },
        );
    }
    for rec in &input.assessment.recommendations {
        book.add(
            format!(
                "obs:{}",
                sha16(&format!("{}\u{1f}{}", rec.dimension, rec.title))
            ),
            rec.title.clone(),
            2,
            NodeType::Observation,
            NodeAttributesBag {
                weight: Some(match rec.severity.as_str() {
                    "high" => 0.9,
                    "medium" => 0.6,
                    _ => 0.3,
                }),
                tags: vec!["studio:recommendation".into(), rec.severity.clone()],
                description: Some(rec.detail.clone()),
            },
        );
    }

    // --- L3 shallow: themes -----------------------------------------------
    for theme in &input.theme.themes {
        book.add(
            format!(
                "thm:{}",
                sha16(&format!("theme\u{1f}{}", fold(&theme.term)))
            ),
            theme.term.clone(),
            3,
            NodeType::Concept,
            NodeAttributesBag {
                weight: Some(clamp01(theme.coverage)),
                tags: vec!["studio:theme".into(), theme.reach.clone()],
                description: Some(format!(
                    "提及 {} 次，覆盖 {} 个章节",
                    theme.mentions, theme.sections_touched
                )),
            },
        );
    }

    // --- edges: character co-occurrences ------------------------------------
    let mut edges: Vec<KnowledgeEdge> = Vec::new();
    let max_cooc = input
        .t1
        .entity_cooccurrences
        .iter()
        .map(|(_, c)| *c)
        .max()
        .unwrap_or(1)
        .max(1);
    for (pair, count) in &input.t1.entity_cooccurrences {
        // The pair label is rendered "A ↔ B" by the analysis layer.
        let Some((a, b)) = pair.split_once(" ↔ ") else {
            continue;
        };
        let (Some(a_id), Some(b_id)) = (book.find(a), book.find(b)) else {
            continue; // endpoints must exist; a dangling edge is never written
        };
        if a_id == b_id {
            continue;
        }
        let mut sorted = [a_id.as_str(), b_id.as_str()];
        sorted.sort_unstable();
        edges.push(KnowledgeEdge {
            id: format!(
                "edg:{}",
                sha16(&format!("{}\u{1f}{}", sorted[0], sorted[1]))
            ),
            source: a_id,
            target: b_id,
            edge_type: EdgeType::Association,
            directed: Some(false),
            layer_scope: None,
            attributes: Some(EdgeAttributes {
                strength: Some(clamp01(*count as f64 / max_cooc as f64)),
                label: None,
            }),
        });
    }

    // --- layer 6: the sky ----------------------------------------------------
    // Fixed build order (plotlines → turning points → themes) keeps both the
    // golden-angle placement and the byte-for-byte output deterministic.
    let mut narratives: Vec<Narrative> = Vec::new();
    let mut place_in_sky = 0usize;
    let push_narrative = |narratives: &mut Vec<Narrative>,
                          place_in_sky: &mut usize,
                          kind: NarrativeType,
                          id: String,
                          label: String,
                          story: String,
                          stability: f64,
                          links: Vec<String>| {
        narratives.push(Narrative {
            id,
            narrative_type: kind,
            label,
            position: sky_position(*place_in_sky, kind),
            linked_nodes: (!links.is_empty()).then_some(links),
            linked_layers: None,
            attributes: Some(NarrativeAttributes {
                story: Some(story),
                stability: Some(clamp01(stability)),
            }),
        });
        *place_in_sky += 1;
    };

    for line in &input.plot.plotlines {
        let kind = match line.shape.as_str() {
            "sustained" if line.is_main => NarrativeType::RainBand,
            _ if line.is_main => NarrativeType::TetheredBalloon,
            "intermittent" | "burst" | "late_arrival" => NarrativeType::Satellite,
            "setup_only" => NarrativeType::FreeBalloon,
            _ => continue, // minor threads are noise, not narrative
        };
        let prefix = match kind {
            NarrativeType::RainBand => "贯穿线",
            NarrativeType::TetheredBalloon => "主线",
            NarrativeType::Satellite => "支线",
            _ => "未回收",
        };
        let links: Vec<String> = book.find(&line.subject).into_iter().collect();
        push_narrative(
            &mut narratives,
            &mut place_in_sky,
            kind,
            format!(
                "nrr:{}",
                sha16(&format!("plot\u{1f}{}", fold(&line.subject)))
            ),
            format!("{} · {}", line.subject, prefix),
            format!(
                "提及 {} 次，跨越 {} 个章节",
                line.mentions, line.sections_touched
            ),
            line.coverage,
            links,
        );
    }

    for turn in &input.plot.turning_points {
        let links: Vec<String> = turn
            .appeared
            .iter()
            .filter_map(|subject| book.find(subject))
            .collect();
        push_narrative(
            &mut narratives,
            &mut place_in_sky,
            NarrativeType::Tornado,
            format!(
                "nrr:{}",
                sha16(&format!("turn\u{1f}{}", fold(&turn.section)))
            ),
            format!("{} · 转折", turn.section),
            format!(
                "登场：{}；淡出：{}",
                if turn.appeared.is_empty() {
                    "无".into()
                } else {
                    turn.appeared.join("、")
                },
                if turn.dropped.is_empty() {
                    "无".into()
                } else {
                    turn.dropped.join("、")
                }
            ),
            1.0 - turn.change,
            links,
        );
    }
    for theme in &input.theme.themes {
        let Some(node_id) = book.find(&theme.term) else {
            continue;
        };
        push_narrative(
            &mut narratives,
            &mut place_in_sky,
            NarrativeType::Cloud,
            format!(
                "nrr:{}",
                sha16(&format!("cloud\u{1f}{}", fold(&theme.term)))
            ),
            format!("{} · 主题", theme.term),
            format!(
                "提及 {} 次，{} 个章节里都在",
                theme.mentions, theme.sections_touched
            ),
            theme.coverage,
            vec![node_id],
        );
    }

    // --- provenance -----------------------------------------------------------
    let mut studio = serde_json::Map::new();
    studio.insert("projectId".into(), serde_json::json!(input.project_id));
    studio.insert(
        "studioVersion".into(),
        serde_json::json!(env!("CARGO_PKG_VERSION")),
    );
    studio.insert(
        "analysisVersion".into(),
        serde_json::json!(studio_analysis::ANALYSIS_VERSION),
    );
    studio.insert("sourceHash".into(), serde_json::json!(input.source_hash));
    let mut custom = serde_json::Map::new();
    custom.insert("studio".into(), serde_json::Value::Object(studio));
    let mut metadata = serde_json::Map::new();
    metadata.insert(
        "tags".into(),
        serde_json::json!(["generated:narrativestudio"]),
    );
    metadata.insert("custom".into(), serde_json::Value::Object(custom));

    Ok(KnowsphereDoc {
        version: KNOWSPHERE_VERSION.into(),
        name: input.project_name.to_string(),
        description: Some("由 NarrativeStudio 从文本分析生成的知识体".into()),
        created: input.generated_at.to_rfc3339(),
        nodes: book.nodes,
        edges,
        narratives,
        metadata: Some(metadata),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use studio_analysis::{
        NarrativeArc, Plotline, PlotlineStage, Recommendation, Theme, TurningPoint,
    };
    use studio_core::knowsphere::{ATMOSPHERE_MAX, ATMOSPHERE_MIN};

    fn at() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-01-01T08:30:00Z")
            .unwrap()
            .with_timezone(&Utc)
    }

    fn empty_t1() -> T1Stats {
        T1Stats {
            sections: vec![],
            section_count: 0,
            title_blocks: 0,
            paragraph_blocks: 0,
            list_blocks: 0,
            block_type_distribution: vec![],
            longest_section: None,
            shortest_section: None,
            characters: vec![],
            arc: NarrativeArc {
                points: vec![],
                shape: "steady".into(),
            },
            entity_cooccurrences: vec![],
        }
    }

    fn empty_plot() -> PlotStats {
        PlotStats {
            plotlines: vec![],
            thread_count: 0,
            main_plotlines: vec![],
            sub_plotlines: vec![],
            turning_points: vec![],
            unresolved_threads: vec![],
            late_arrivals: vec![],
            coherence_score: 100.0,
            section_count: 0,
        }
    }

    fn empty_theme() -> ThemeStats {
        ThemeStats {
            themes: vec![],
            candidate_count: 0,
            total_mentions: 0,
            source: "none".into(),
            top_share: 0.0,
            top3_share: 0.0,
            singleton_share: 0.0,
            dominant_terms: vec![],
            section_count: 0,
            theme_score: None,
        }
    }

    fn empty_setting() -> SettingStats {
        SettingStats {
            points: vec![],
            segment_count: 0,
            chunked: false,
            scenes: vec![],
            scene_count: 0,
            unanchored_scenes: 0,
            unanchored_share: 0.0,
            places: vec![],
            place_count: 0,
            recurring_places: 0,
            total_env_hits: 0,
            env_density: 0.0,
            top_env_terms: vec![],
            verdict: "none".into(),
            dominant_place: String::new(),
            peak_section: String::new(),
            peak_intensity: 0.0,
            issues: vec![],
            longest_absence: 0,
            setting_score: None,
        }
    }

    fn empty_timeline() -> TimelineStats {
        TimelineStats {
            markers: vec![],
            curve: vec![],
            absolute_count: 0,
            partial_count: 0,
            relative_count: 0,
            vague_count: 0,
            forms: vec![],
            entity_date_count: 0,
            first_date: String::new(),
            last_date: String::new(),
            span_days: 0,
            backsteps: vec![],
            jumps: vec![],
            unmarked_stretches: vec![],
            issues: vec![],
            segment_count: 0,
            chunked: false,
            char_count: 0,
            readable: false,
            note: "全文没有读到任何时间表达。".into(),
        }
    }

    fn clean_assessment() -> Assessment {
        Assessment {
            overall: 100.0,
            dimensions: vec![],
            recommendations: vec![],
        }
    }

    fn stat(stable_id: &str, text: &str, category: &str, mentions: u64) -> EntityStat {
        EntityStat {
            stable_id: stable_id.into(),
            text: text.into(),
            category: category.into(),
            mentions,
            confidence: 0.9,
        }
    }

    fn input_with<'a>(
        entities: &'a [EntityStat],
        t1: &'a T1Stats,
        plot: &'a PlotStats,
        theme: &'a ThemeStats,
        setting: &'a SettingStats,
        timeline: &'a TimelineStats,
        assessment: &'a Assessment,
    ) -> KnowsphereInput<'a> {
        KnowsphereInput {
            project_name: "测试项目",
            project_id: "11111111-1111-1111-1111-111111111111",
            generated_at: at(),
            source_hash: "abc123",
            t1,
            plot,
            theme,
            setting,
            timeline,
            assessment,
            entities,
        }
    }

    #[test]
    fn empty_analysis_still_validates() {
        let (t1, plot, theme, setting, timeline, assessment) = (
            empty_t1(),
            empty_plot(),
            empty_theme(),
            empty_setting(),
            empty_timeline(),
            clean_assessment(),
        );
        let doc = render_knowsphere(&input_with(
            &[],
            &t1,
            &plot,
            &theme,
            &setting,
            &timeline,
            &assessment,
        ))
        .unwrap();
        assert_eq!(doc.validate(), Vec::<String>::new());
        assert!(doc.nodes.is_empty());
    }

    #[test]
    fn maps_entities_themes_places_and_edges() {
        let entities = vec![
            stat("ent:aaa", "马洛", "PERSON", 40),
            stat("ent:bbb", "码头", "LOCATION", 10),
            stat("ent:ccc", "2026年", "DATE", 5),
        ];
        let mut t1 = empty_t1();
        t1.entity_cooccurrences = vec![("马洛 ↔ 码头".into(), 6), ("马洛 ↔ 路人".into(), 2)];
        let mut theme = empty_theme();
        theme.themes = vec![Theme {
            term: "记忆".into(),
            members: vec![],
            mentions: 12,
            sections_touched: 4,
            coverage: 0.5,
            share: 0.2,
            first_section: "第一章".into(),
            last_section: "第四章".into(),
            reach: "sustained".into(),
            longest_gap: 1,
            concentration: "spread".into(),
            curve: vec![],
        }];
        let mut setting = empty_setting();
        setting.places = vec![studio_analysis::Place {
            name: "码头".into(),
            kind: "LOCATION".into(),
            mentions: 10,
            segments_touched: 3,
            coverage: 0.4,
            first_section: "第一章".into(),
            last_section: "第三章".into(),
            longest_absence: 2,
            returns: 1,
            curve: vec![],
        }];
        let doc = render_knowsphere(&input_with(
            &entities,
            &t1,
            &empty_plot(),
            &theme,
            &setting,
            &empty_timeline(),
            &clean_assessment(),
        ))
        .unwrap();

        assert_eq!(doc.validate(), Vec::<String>::new());
        // DATE entities are not nodes; the place merges with the LOCATION
        // entity by name, so there are exactly two entity nodes plus a concept.
        assert_eq!(doc.nodes.len(), 3);
        assert!(doc.nodes.iter().all(|n| n.layer_id <= 5));
        assert_eq!(
            doc.nodes.iter().filter(|n| n.label == "码头").count(),
            1,
            "place and entity with the same name are one node"
        );
        // The co-occurrence to an off-sphere entity ("路人") is dropped; the
        // other one lands as an association edge.
        assert_eq!(doc.edges.len(), 1);
        assert_eq!(doc.edges[0].source, "ent:aaa");
        assert_eq!(doc.edges[0].target, "ent:bbb");
        // The theme is both a concept node and a cloud tethered to it.
        let cloud = doc
            .narratives
            .iter()
            .find(|n| n.narrative_type == NarrativeType::Cloud)
            .expect("theme becomes a cloud");
        let concept = doc
            .nodes
            .iter()
            .find(|n| n.node_type == NodeType::Concept)
            .unwrap();
        let linked = cloud.linked_nodes.as_deref().unwrap();
        assert_eq!(linked, std::slice::from_ref(&concept.id));
    }

    #[test]
    fn plotlines_and_turns_become_sky_elements() {
        let mut plot = empty_plot();
        plot.plotlines = vec![
            Plotline {
                subject: "马洛".into(),
                kind: "PERSON".into(),
                mentions: 30,
                sections_touched: 8,
                coverage: 0.8,
                first_section: "第一章".into(),
                last_section: "第八章".into(),
                stages: vec![PlotlineStage {
                    section: "第一章".into(),
                    mentions: 10,
                    gap_before: 0,
                }],
                longest_gap: 2,
                shape: "sustained".into(),
                is_main: true,
            },
            Plotline {
                subject: "旧钥匙".into(),
                kind: "SIGNAL".into(),
                mentions: 3,
                sections_touched: 1,
                coverage: 0.1,
                first_section: "第二章".into(),
                last_section: "第二章".into(),
                stages: vec![],
                longest_gap: 7,
                shape: "setup_only".into(),
                is_main: false,
            },
        ];
        plot.turning_points = vec![TurningPoint {
            section: "第四章".into(),
            change: 0.6,
            appeared: vec!["码头".into()],
            dropped: vec!["马洛".into()],
        }];
        let entities = vec![
            stat("ent:aaa", "马洛", "PERSON", 30),
            stat("ent:bbb", "码头", "LOCATION", 4),
        ];
        let doc = render_knowsphere(&input_with(
            &entities,
            &empty_t1(),
            &plot,
            &empty_theme(),
            &empty_setting(),
            &empty_timeline(),
            &clean_assessment(),
        ))
        .unwrap();

        assert_eq!(doc.validate(), Vec::<String>::new());
        assert!(doc
            .narratives
            .iter()
            .any(|n| n.narrative_type == NarrativeType::RainBand));
        assert!(doc
            .narratives
            .iter()
            .any(|n| n.narrative_type == NarrativeType::FreeBalloon));
        let tornado = doc
            .narratives
            .iter()
            .find(|n| n.narrative_type == NarrativeType::Tornado)
            .unwrap();
        let linked = tornado.linked_nodes.as_deref().unwrap();
        assert_eq!(linked, &["ent:bbb".to_string()]);
        // Every element sits inside the atmosphere band by construction.
        for n in &doc.narratives {
            let d = (n.position.x.powi(2) + n.position.y.powi(2) + n.position.z.powi(2)).sqrt();
            assert!(
                (ATMOSPHERE_MIN..=ATMOSPHERE_MAX).contains(&d),
                "{d} out of band"
            );
        }
    }

    #[test]
    fn same_input_produces_byte_identical_output() {
        let entities = vec![stat("ent:aaa", "马洛", "PERSON", 40)];
        let mut timeline = empty_timeline();
        timeline.first_date = "1999-03-01".into();
        timeline.last_date = "1999-06-01".into();
        timeline.span_days = 92;
        let mut assessment = clean_assessment();
        assessment.recommendations = vec![Recommendation {
            severity: "high".into(),
            dimension: "readability".into(),
            title: "可读性偏低".into(),
            detail: "平均句长偏长".into(),
        }];
        let render_once = || {
            render_knowsphere(&input_with(
                &entities,
                &empty_t1(),
                &empty_plot(),
                &empty_theme(),
                &empty_setting(),
                &timeline,
                &assessment,
            ))
            .unwrap()
            .serialize()
        };
        assert_eq!(render_once(), render_once());
        let text = render_once();
        // Provenance rides in metadata.custom.studio.
        assert!(text.contains("\"studio\""));
        assert!(text.contains("abc123"));
        assert!(text.contains("1999-03-01"));
    }
}
