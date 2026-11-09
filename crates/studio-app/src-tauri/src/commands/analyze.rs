//! Analysis commands: per-file T0/T1, the full project analysis with its
//! content-hash-keyed result cache, and document reads.

use duckdb::Connection;
use serde::Serialize;
use studio_analysis::{
    document_hash, run_assessment, run_cliche_analysis, run_conflict_analysis,
    run_dialogue_analysis, run_explicit_analysis, run_pacing_analysis, run_plot_analysis,
    run_setting_analysis, run_structure_analysis, run_t0_analysis, run_t1_analysis,
    run_theme_analysis, run_timeline_analysis, Assessment, ClicheStats, ConflictStats,
    DialogueStats, ExplicitStats, PacingStats, PlotStats, SettingStats, StructureStats, T0Stats,
    T1Stats, ThemeStats, TimelineStats,
};
use studio_core::concern::{Concern, ConcernLocation, Severity};
use studio_core::document::DocumentData;
use studio_import::parse_semantic_result;
use studio_storage::{
    get, invalidate_project, load_cached, load_document, save_concerns, save_result,
};
use tauri::State;
use uuid::Uuid;

use crate::AppState;

/// Run T0 analysis on a semantic_result.json file.
#[tauri::command]
pub(crate) fn analyze_t0(file_path: String) -> Result<T0Stats, String> {
    let content =
        std::fs::read_to_string(&file_path).map_err(|e| format!("Failed to read file: {e}"))?;
    let doc = parse_semantic_result(&content).map_err(|e| format!("Failed to parse: {e}"))?;
    run_t0_analysis(&doc).map_err(|e| e.to_string())
}

/// Run T1 structural analysis on a semantic_result.json file.
#[tauri::command]
pub(crate) fn analyze_t1(file_path: String) -> Result<T1Stats, String> {
    let content =
        std::fs::read_to_string(&file_path).map_err(|e| format!("Failed to read file: {e}"))?;
    let doc = parse_semantic_result(&content).map_err(|e| format!("Failed to parse: {e}"))?;
    run_t1_analysis(&doc).map_err(|e| e.to_string())
}

/// Analysis result for a single project (T0 + T1 + plot + pacing + conflict +
/// theme + setting + cliché + dialogue + explicit language + story structure +
/// time line + rule-based assessment).
#[derive(Debug, Clone, Serialize)]
pub struct ProjectAnalysis {
    pub project_id: String,
    pub name: String,
    pub t0: T0Stats,
    pub t1: T1Stats,
    pub plot: PlotStats,
    pub pacing: PacingStats,
    pub conflict: ConflictStats,
    pub theme: ThemeStats,
    pub setting: SettingStats,
    pub cliche: ClicheStats,
    pub dialogue: DialogueStats,
    pub explicit: ExplicitStats,
    pub structure: StructureStats,
    pub timeline: TimelineStats,
    pub assessment: Assessment,
    /// True when the numbers came from the persisted cache instead of being
    /// recomputed, so the UI can say where the result came from.
    pub cached: bool,
}

/// Load all documents of a project and merge them into a single `DocumentData`.
///
/// A project currently holds one document, but this merges any number so the
/// analysis stays correct if a project ever contains multiple documents.
pub(crate) fn load_project_document(
    conn: &Connection,
    project_id: Uuid,
) -> Result<DocumentData, String> {
    let mut stmt = conn
        .prepare("SELECT doc_id FROM documents WHERE project_id = ? ORDER BY created_at ASC")
        .map_err(|e| e.to_string())?;
    let doc_ids: Vec<String> = stmt
        .query_map([project_id.to_string()], |row| row.get::<_, String>(0))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    let mut merged = DocumentData {
        doc_id: Uuid::new_v4(),
        title: String::new(),
        blocks: Vec::new(),
        total_word_count: 0,
        total_char_count: 0,
    };

    for doc_id in doc_ids {
        let uuid = Uuid::parse_str(&doc_id).map_err(|e| e.to_string())?;
        let doc = load_document(conn, uuid).map_err(|e| e.to_string())?;
        merged.total_word_count += doc.total_word_count;
        merged.total_char_count += doc.total_char_count;
        merged.blocks.extend(doc.blocks);
    }

    Ok(merged)
}

/// Core: get T0/T1/assessment for a project, from the cache when it is valid.
///
/// The cache is keyed on the SHA-256 of the analysed content, so it is only
/// reused for byte-identical input: re-importing the same file keeps the cache
/// (the new `doc_id` is not part of the hash), while any text change misses.
/// `force` (the "重新分析" button) drops the stored rows first and recomputes —
/// needed when the analysis logic itself changed, which the input hash cannot
/// detect on its own.
pub(crate) fn analyze_project_inner(
    state: &State<AppState>,
    project_id: &str,
    force: bool,
) -> Result<ProjectAnalysis, String> {
    let pid = Uuid::parse_str(project_id).map_err(|e| e.to_string())?;
    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    let project = get(&storage.conn, pid).map_err(|e| e.to_string())?;
    let doc = load_project_document(&storage.conn, pid)?;
    let input_hash = document_hash(&doc).map_err(|e| e.to_string())?;

    if force {
        invalidate_project(&storage.conn, pid).map_err(|e| e.to_string())?;
    } else {
        let t0 = load_cached::<T0Stats>(&storage.conn, pid, "t0", &input_hash)
            .map_err(|e| e.to_string())?;
        let t1 = load_cached::<T1Stats>(&storage.conn, pid, "t1", &input_hash)
            .map_err(|e| e.to_string())?;
        let plot = load_cached::<PlotStats>(&storage.conn, pid, "plot", &input_hash)
            .map_err(|e| e.to_string())?;
        let pacing = load_cached::<PacingStats>(&storage.conn, pid, "pacing", &input_hash)
            .map_err(|e| e.to_string())?;
        let conflict = load_cached::<ConflictStats>(&storage.conn, pid, "conflict", &input_hash)
            .map_err(|e| e.to_string())?;
        let theme = load_cached::<ThemeStats>(&storage.conn, pid, "theme", &input_hash)
            .map_err(|e| e.to_string())?;
        let setting = load_cached::<SettingStats>(&storage.conn, pid, "setting", &input_hash)
            .map_err(|e| e.to_string())?;
        let cliche = load_cached::<ClicheStats>(&storage.conn, pid, "cliche", &input_hash)
            .map_err(|e| e.to_string())?;
        let dialogue = load_cached::<DialogueStats>(&storage.conn, pid, "dialogue", &input_hash)
            .map_err(|e| e.to_string())?;
        let explicit = load_cached::<ExplicitStats>(&storage.conn, pid, "explicit", &input_hash)
            .map_err(|e| e.to_string())?;
        let structure = load_cached::<StructureStats>(&storage.conn, pid, "structure", &input_hash)
            .map_err(|e| e.to_string())?;
        let timeline = load_cached::<TimelineStats>(&storage.conn, pid, "timeline", &input_hash)
            .map_err(|e| e.to_string())?;
        let assessment = load_cached::<Assessment>(&storage.conn, pid, "assessment", &input_hash)
            .map_err(|e| e.to_string())?;
        if let (
            Some(t0),
            Some(t1),
            Some(plot),
            Some(pacing),
            Some(conflict),
            Some(theme),
            Some(setting),
            Some(cliche),
            Some(dialogue),
            Some(explicit),
            Some(structure),
            Some(timeline),
            Some(assessment),
        ) = (
            t0, t1, plot, pacing, conflict, theme, setting, cliche, dialogue, explicit, structure,
            timeline, assessment,
        ) {
            return Ok(ProjectAnalysis {
                project_id: pid.to_string(),
                name: project.name,
                t0,
                t1,
                plot,
                pacing,
                conflict,
                theme,
                setting,
                cliche,
                dialogue,
                explicit,
                structure,
                timeline,
                assessment,
                cached: true,
            });
        }
    }

    let t0 = run_t0_analysis(&doc).map_err(|e| e.to_string())?;
    let t1 = run_t1_analysis(&doc).map_err(|e| e.to_string())?;
    let plot = run_plot_analysis(&doc).map_err(|e| e.to_string())?;
    let pacing = run_pacing_analysis(&doc).map_err(|e| e.to_string())?;
    let conflict = run_conflict_analysis(&doc).map_err(|e| e.to_string())?;
    let theme = run_theme_analysis(&doc).map_err(|e| e.to_string())?;
    let setting = run_setting_analysis(&doc).map_err(|e| e.to_string())?;
    let cliche = run_cliche_analysis(&doc).map_err(|e| e.to_string())?;
    let dialogue = run_dialogue_analysis(&doc).map_err(|e| e.to_string())?;
    let explicit = run_explicit_analysis(&doc).map_err(|e| e.to_string())?;
    // Structure reads the other two passes rather than the text again: its landmarks
    // are the turns 情节分析 already found and the tension curve 节奏分析 already drew.
    let structure = run_structure_analysis(&doc, &pacing, &plot).map_err(|e| e.to_string())?;
    let timeline = run_timeline_analysis(&doc).map_err(|e| e.to_string())?;
    let assessment = run_assessment(&t0, &t1);

    let analysis = ProjectAnalysis {
        project_id: pid.to_string(),
        name: project.name,
        t0,
        t1,
        plot,
        pacing,
        conflict,
        theme,
        setting,
        cliche,
        dialogue,
        explicit,
        structure,
        timeline,
        assessment,
        cached: false,
    };

    // A cache-write failure must not lose the analysis the user is waiting for:
    // warn and return the fresh result, which simply costs a recompute next time.
    if let Err(e) = persist_analysis(&storage.conn, pid, &input_hash, &analysis) {
        eprintln!("分析结果写入缓存失败（本次结果仍已返回）: {e}");
    }

    Ok(analysis)
}

/// Persist the thirteen computed dimensions under `input_hash`, plus the
/// assessment's recommendations as queryable concern rows.
///
/// The recommendation rows are what let a later feature (report export) list a
/// project's concerns without re-running the analysis.
pub(crate) fn persist_analysis(
    conn: &Connection,
    project_id: Uuid,
    input_hash: &str,
    analysis: &ProjectAnalysis,
) -> Result<(), String> {
    // Plot, pacing, conflict, theme, setting, cliché, dialogue, explicit language, story
    // structure and time line sit at the T1 tier: same input, same cost, purely structural.
    // Their scores are stored so the analysis history can be compared without unpacking the
    // payload; conflict, theme, setting and cliché have no score when the text offers
    // nothing for them to measure, and dialogue, explicit language, story structure and time
    // line never have one at all — 30% speech is neither better nor worse than 5%, a war
    // novel is not worse than a pastoral for carrying violence, a story that refuses
    // three-act shape may be refusing it on purpose, and a story that never writes a date is
    // not keeping time badly. The row keeps that absence instead of inventing a 0.
    let dimensions: Vec<(&str, &str, Option<f32>, String)> = vec![
        (
            "t0",
            "t0",
            None,
            serde_json::to_string(&analysis.t0).map_err(|e| e.to_string())?,
        ),
        (
            "t1",
            "t1",
            None,
            serde_json::to_string(&analysis.t1).map_err(|e| e.to_string())?,
        ),
        (
            "plot",
            "t1",
            Some(analysis.plot.coherence_score as f32),
            serde_json::to_string(&analysis.plot).map_err(|e| e.to_string())?,
        ),
        (
            "pacing",
            "t1",
            Some(analysis.pacing.pace_score as f32),
            serde_json::to_string(&analysis.pacing).map_err(|e| e.to_string())?,
        ),
        (
            "conflict",
            "t1",
            analysis.conflict.conflict_score.map(|s| s as f32),
            serde_json::to_string(&analysis.conflict).map_err(|e| e.to_string())?,
        ),
        (
            "theme",
            "t1",
            analysis.theme.theme_score.map(|s| s as f32),
            serde_json::to_string(&analysis.theme).map_err(|e| e.to_string())?,
        ),
        (
            "setting",
            "t1",
            analysis.setting.setting_score.map(|s| s as f32),
            serde_json::to_string(&analysis.setting).map_err(|e| e.to_string())?,
        ),
        (
            "cliche",
            "t1",
            analysis.cliche.cliche_score.map(|s| s as f32),
            serde_json::to_string(&analysis.cliche).map_err(|e| e.to_string())?,
        ),
        (
            "dialogue",
            "t1",
            None,
            serde_json::to_string(&analysis.dialogue).map_err(|e| e.to_string())?,
        ),
        (
            "explicit",
            "t1",
            None,
            serde_json::to_string(&analysis.explicit).map_err(|e| e.to_string())?,
        ),
        (
            "structure",
            "t1",
            None,
            serde_json::to_string(&analysis.structure).map_err(|e| e.to_string())?,
        ),
        (
            "timeline",
            "t1",
            None,
            serde_json::to_string(&analysis.timeline).map_err(|e| e.to_string())?,
        ),
    ];
    for (dimension, tier, score, data) in dimensions {
        save_result(conn, project_id, dimension, tier, input_hash, score, &data)
            .map_err(|e| e.to_string())?;
    }

    // The assessment row is written apart from the loop because its id anchors the
    // concern rows.
    let assessment_id = save_result(
        conn,
        project_id,
        "assessment",
        "t1",
        input_hash,
        Some(analysis.assessment.overall as f32),
        &serde_json::to_string(&analysis.assessment).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;

    let concerns: Vec<Concern> = analysis
        .assessment
        .recommendations
        .iter()
        .map(|r| Concern {
            severity: concern_severity(&r.severity),
            title: r.title.clone(),
            description: r.detail.clone(),
            location: ConcernLocation {
                block_id: None,
                section_path: None,
                span_start: None,
                span_end: None,
            },
            suggestion: None,
        })
        .collect();
    save_concerns(conn, assessment_id, &concerns).map_err(|e| e.to_string())?;
    Ok(())
}

/// The rule engine spells severity as `"high" | "medium" | "low"`; anything else
/// keeps the concern at `Medium` rather than dropping it.
pub(crate) fn concern_severity(severity: &str) -> Severity {
    match severity {
        "high" => Severity::High,
        "low" => Severity::Low,
        _ => Severity::Medium,
    }
}

/// Run T0/T1 analysis on a single project (data loaded from the database).
///
/// Marked `async` so the DB load + T0/T1 analysis run on Tauri's async runtime
/// (a worker thread) rather than the main/UI thread. Keeping this heavy work off
/// the main thread is what prevents the frontend from freezing on large docs.
///
/// `force: true` re-runs the analysis even when a valid cached result exists.
#[tauri::command]
pub(crate) async fn analyze_project(
    state: State<'_, AppState>,
    project_id: String,
    force: Option<bool>,
) -> Result<ProjectAnalysis, String> {
    analyze_project_inner(&state, &project_id, force.unwrap_or(false))
}

/// Run T0/T1 analysis on multiple projects for cross-project comparison.
///
/// `async` for the same reason as [`analyze_project`]: keep the heavy work off
/// the main/UI thread so the frontend stays responsive. Comparison always reads
/// the cache when it is valid — there is no UI affordance to force it here.
#[tauri::command]
pub(crate) async fn analyze_projects(
    state: State<'_, AppState>,
    project_ids: Vec<String>,
) -> Result<Vec<ProjectAnalysis>, String> {
    project_ids
        .iter()
        .map(|id| analyze_project_inner(&state, id, false))
        .collect()
}

/// List documents for a project.
#[tauri::command]
pub(crate) fn list_documents(
    state: State<AppState>,
    project_id: String,
) -> Result<Vec<(String, String)>, String> {
    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    let mut stmt = storage
        .conn
        .prepare(
            "SELECT doc_id, title FROM documents WHERE project_id = ? ORDER BY created_at DESC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([project_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|e| e.to_string())?;
    let mut docs = Vec::new();
    for row in rows {
        docs.push(row.map_err(|e| e.to_string())?);
    }
    Ok(docs)
}

/// Load a document by ID.
#[tauri::command]
pub(crate) fn load_document_cmd(
    state: State<AppState>,
    doc_id: String,
) -> Result<studio_core::document::DocumentData, String> {
    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    let doc_uuid: uuid::Uuid =
        uuid::Uuid::parse_str(&doc_id).map_err(|e: uuid::Error| e.to_string())?;
    load_document(&storage.conn, doc_uuid).map_err(|e| e.to_string())
}
