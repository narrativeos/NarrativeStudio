//! Analysis commands: per-file T0/T1, the full project analysis with its
//! content-hash-keyed result cache, and document reads.

use duckdb::Connection;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::collections::HashMap;
use studio_analysis::{
    document_hash, run_assessment, run_cliche_analysis, run_conflict_analysis,
    run_dialogue_analysis, run_explicit_analysis, run_pacing_analysis, run_plot_analysis,
    run_review_checklist, run_setting_analysis, run_structure_analysis, run_t0_analysis,
    run_t1_analysis, run_theme_analysis, run_timeline_analysis, run_voice_analysis, Assessment,
    ChecklistSource, ClicheStats, ConflictStats, DialogueStats, ExplicitStats, PacingStats,
    PlotStats, ReviewChecklist, SettingStats, StructureStats, T0Stats, T1Stats, ThemeStats,
    TimelineStats, VoiceStats,
};
use studio_core::concern::{Concern, ConcernLocation, Severity};
use studio_core::document::DocumentData;
use studio_import::parse_semantic_result;
use studio_storage::{
    get, invalidate_project, load_document, load_project_documents, load_results_for_hash,
    save_analysis_bundle, DimensionRow,
};
use tauri::State;
use uuid::Uuid;

use crate::AppState;

// The dimension keys, spelled once: the cache read and the persist table both
// name rows through these, so a rename cannot land on one side only. The
// frontend's `analysis-shape.ts` keeps the matching list.
const D_T0: &str = "t0";
const D_T1: &str = "t1";
const D_PLOT: &str = "plot";
const D_PACING: &str = "pacing";
const D_CONFLICT: &str = "conflict";
const D_THEME: &str = "theme";
const D_SETTING: &str = "setting";
const D_CLICHE: &str = "cliche";
const D_DIALOGUE: &str = "dialogue";
const D_EXPLICIT: &str = "explicit";
const D_STRUCTURE: &str = "structure";
const D_TIMELINE: &str = "timeline";
const D_VOICE: &str = "voice";
const D_CHECKLIST: &str = "checklist";
const D_ASSESSMENT: &str = "assessment";

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
/// time line + author voice + the final review checklist + rule-based assessment).
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
    pub voice: VoiceStats,
    /// The numbers that still need a decision, gathered from the dimensions above.
    pub checklist: ReviewChecklist,
    pub assessment: Assessment,
    /// True when the numbers came from the persisted cache instead of being
    /// recomputed, so the UI can say where the result came from.
    pub cached: bool,
}

/// Load all documents of a project and merge them into a single `DocumentData`.
///
/// A project currently holds one document, but this merges any number so the
/// analysis stays correct if a project ever contains multiple documents. The
/// batched loader keeps the whole read at a constant number of queries no
/// matter how many documents a project holds.
pub(crate) fn load_project_document(
    conn: &Connection,
    project_id: Uuid,
) -> Result<DocumentData, String> {
    let docs = load_project_documents(conn, project_id).map_err(|e| e.to_string())?;

    let mut merged = DocumentData {
        doc_id: Uuid::new_v4(),
        title: String::new(),
        blocks: Vec::new(),
        total_word_count: 0,
        total_char_count: 0,
    };

    for doc in docs {
        merged.total_word_count += doc.total_word_count;
        merged.total_char_count += doc.total_char_count;
        merged.blocks.extend(doc.blocks);
    }

    Ok(merged)
}

/// Core: get the full analysis for a project, from the cache when it is valid.
///
/// The cache is keyed on the SHA-256 of the analysed content, so it is only
/// reused for byte-identical input: re-importing the same file keeps the cache
/// (the new `doc_id` is not part of the hash), while any text change misses.
/// `force` (the "重新分析" button) drops the stored rows first and recomputes —
/// needed when the analysis logic itself changed, which the input hash cannot
/// detect on its own.
///
/// Returns the analysis and the `input_hash` it is keyed on, so a caller that
/// also needs the hash (`export_knowsphere`) does not read and hash the whole
/// document a second time.
///
/// The database lock is held for the queries — project row, documents, hash,
/// cache check — and again for the write at the end, never across the
/// CPU-heavy analysis passes in between. One mutex serves every command in
/// the app; holding it through a computation would block even
/// `list_projects` until the analysis finished.
pub(crate) fn analyze_project_inner(
    state: &State<AppState>,
    project_id: &str,
    force: bool,
) -> Result<(ProjectAnalysis, String), String> {
    let pid = Uuid::parse_str(project_id).map_err(|e| e.to_string())?;
    let (project, doc, input_hash, cached) = {
        let storage = state.storage.lock().map_err(|e| e.to_string())?;
        let project = get(&storage.conn, pid).map_err(|e| e.to_string())?;
        let doc = load_project_document(&storage.conn, pid)?;
        let input_hash = document_hash(&doc).map_err(|e| e.to_string())?;
        if force {
            invalidate_project(&storage.conn, pid).map_err(|e| e.to_string())?;
        }
        let cached = if force {
            None
        } else {
            cached_analysis(&storage.conn, pid, &project.name, &input_hash)?
        };
        (project, doc, input_hash, cached)
    };

    if let Some(analysis) = cached {
        return Ok((analysis, input_hash));
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
    let voice = run_voice_analysis(&doc).map_err(|e| e.to_string())?;
    // The checklist reads what the passes above already counted; it measures nothing.
    let checklist = run_review_checklist(&ChecklistSource {
        t0: &t0,
        plot: &plot,
        pacing: &pacing,
        conflict: &conflict,
        setting: &setting,
        cliche: &cliche,
        dialogue: &dialogue,
        explicit: &explicit,
        structure: &structure,
        timeline: &timeline,
        voice: &voice,
    });
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
        voice,
        checklist,
        assessment,
        cached: false,
    };

    // A cache-write failure must not lose the analysis the user is waiting for:
    // warn and return the fresh result, which simply costs a recompute next time.
    // The lock is taken again here for the write alone; the passes above ran
    // without it.
    {
        let storage = state.storage.lock().map_err(|e| e.to_string())?;
        if let Err(e) = persist_analysis(&storage.conn, pid, &input_hash, &analysis) {
            eprintln!("分析结果写入缓存失败（本次结果仍已返回）: {e}");
        }
    }

    Ok((analysis, input_hash))
}

/// The persisted bundle for `input_hash`, if every dimension is present and
/// deserializes into the type the page renders.
///
/// One query loads the whole bundle; a missing or unreadable dimension is a
/// miss for the bundle as a whole, which recomputes it fresh — the same cure
/// that "重新分析" applies to a stale payload.
fn cached_analysis(
    conn: &Connection,
    project_id: Uuid,
    name: &str,
    input_hash: &str,
) -> Result<Option<ProjectAnalysis>, String> {
    let rows = load_results_for_hash(conn, project_id, input_hash).map_err(|e| e.to_string())?;
    let analysis: Option<ProjectAnalysis> = (|| {
        Some(ProjectAnalysis {
            project_id: project_id.to_string(),
            name: name.to_string(),
            t0: dim(&rows, D_T0)?,
            t1: dim(&rows, D_T1)?,
            plot: dim(&rows, D_PLOT)?,
            pacing: dim(&rows, D_PACING)?,
            conflict: dim(&rows, D_CONFLICT)?,
            theme: dim(&rows, D_THEME)?,
            setting: dim(&rows, D_SETTING)?,
            cliche: dim(&rows, D_CLICHE)?,
            dialogue: dim(&rows, D_DIALOGUE)?,
            explicit: dim(&rows, D_EXPLICIT)?,
            structure: dim(&rows, D_STRUCTURE)?,
            timeline: dim(&rows, D_TIMELINE)?,
            voice: dim(&rows, D_VOICE)?,
            checklist: dim(&rows, D_CHECKLIST)?,
            assessment: dim(&rows, D_ASSESSMENT)?,
            cached: true,
        })
    })();
    Ok(analysis)
}

/// One dimension of a loaded bundle, typed. A row that no longer matches its
/// type counts as absent: the bundle misses and the dimension is recomputed.
fn dim<T: DeserializeOwned>(rows: &HashMap<String, String>, dimension: &str) -> Option<T> {
    rows.get(dimension)
        .and_then(|data| serde_json::from_str(data).ok())
}

/// Persist every computed dimension under `input_hash`, plus the checklist
/// and the assessment's recommendations as queryable rows — in one
/// transaction, so the cache never holds half of one analysis and half of
/// the next.
///
/// The recommendation rows are what let a later feature (report export) list a
/// project's concerns without re-running the analysis; they hang off the
/// assessment row, which the bundle write anchors for them.
pub(crate) fn persist_analysis(
    conn: &Connection,
    project_id: Uuid,
    input_hash: &str,
    analysis: &ProjectAnalysis,
) -> Result<(), String> {
    // Plot, pacing, conflict, theme, setting, cliché, dialogue, explicit language, story
    // structure, time line and author voice sit at the T1 tier: same input, same cost, purely
    // structural.
    // Their scores are stored so the analysis history can be compared without unpacking the
    // payload; conflict, theme, setting and cliché have no score when the text offers
    // nothing for them to measure, and dialogue, explicit language, story structure, time
    // line, author voice and the review checklist never have one at all — 30% speech is
    // neither better nor worse than 5%, a war novel is not worse than a pastoral for
    // carrying violence, a story that refuses three-act shape may be refusing it on
    // purpose, a story that never writes a date is not keeping time badly, long sentences
    // are not better or worse than short ones, and a list of questions has no number to
    // carry. The rows keep that absence instead of inventing a 0.
    let t0 = serde_json::to_string(&analysis.t0).map_err(|e| e.to_string())?;
    let t1 = serde_json::to_string(&analysis.t1).map_err(|e| e.to_string())?;
    let plot = serde_json::to_string(&analysis.plot).map_err(|e| e.to_string())?;
    let pacing = serde_json::to_string(&analysis.pacing).map_err(|e| e.to_string())?;
    let conflict = serde_json::to_string(&analysis.conflict).map_err(|e| e.to_string())?;
    let theme = serde_json::to_string(&analysis.theme).map_err(|e| e.to_string())?;
    let setting = serde_json::to_string(&analysis.setting).map_err(|e| e.to_string())?;
    let cliche = serde_json::to_string(&analysis.cliche).map_err(|e| e.to_string())?;
    let dialogue = serde_json::to_string(&analysis.dialogue).map_err(|e| e.to_string())?;
    let explicit = serde_json::to_string(&analysis.explicit).map_err(|e| e.to_string())?;
    let structure = serde_json::to_string(&analysis.structure).map_err(|e| e.to_string())?;
    let timeline = serde_json::to_string(&analysis.timeline).map_err(|e| e.to_string())?;
    let voice = serde_json::to_string(&analysis.voice).map_err(|e| e.to_string())?;
    let checklist = serde_json::to_string(&analysis.checklist).map_err(|e| e.to_string())?;
    let assessment = serde_json::to_string(&analysis.assessment).map_err(|e| e.to_string())?;

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
    // The one table that spells out each dimension's tier and score. The
    // names come from the same `D_*` constants the cache read looks up, so
    // the two halves of the cache cannot drift apart.
    let dimensions = [
        DimensionRow {
            dimension: D_T0,
            tier: "t0",
            score: None,
            data: &t0,
        },
        DimensionRow {
            dimension: D_T1,
            tier: "t1",
            score: None,
            data: &t1,
        },
        DimensionRow {
            dimension: D_PLOT,
            tier: "t1",
            // NULL when the document carries nothing a thread can be traced
            // through — the text-only import case. An empty thread map is a
            // fact about the import, not a verdict on the story.
            score: analysis
                .plot
                .readable
                .then_some(analysis.plot.coherence_score as f32),
            data: &plot,
        },
        DimensionRow {
            dimension: D_PACING,
            tier: "t1",
            score: Some(analysis.pacing.pace_score as f32),
            data: &pacing,
        },
        DimensionRow {
            dimension: D_CONFLICT,
            tier: "t1",
            score: analysis.conflict.conflict_score.map(|s| s as f32),
            data: &conflict,
        },
        DimensionRow {
            dimension: D_THEME,
            tier: "t1",
            score: analysis.theme.theme_score.map(|s| s as f32),
            data: &theme,
        },
        DimensionRow {
            dimension: D_SETTING,
            tier: "t1",
            score: analysis.setting.setting_score.map(|s| s as f32),
            data: &setting,
        },
        DimensionRow {
            dimension: D_CLICHE,
            tier: "t1",
            score: analysis.cliche.cliche_score.map(|s| s as f32),
            data: &cliche,
        },
        DimensionRow {
            dimension: D_DIALOGUE,
            tier: "t1",
            score: None,
            data: &dialogue,
        },
        DimensionRow {
            dimension: D_EXPLICIT,
            tier: "t1",
            score: None,
            data: &explicit,
        },
        DimensionRow {
            dimension: D_STRUCTURE,
            tier: "t1",
            score: None,
            data: &structure,
        },
        DimensionRow {
            dimension: D_TIMELINE,
            tier: "t1",
            score: None,
            data: &timeline,
        },
        DimensionRow {
            dimension: D_VOICE,
            tier: "t1",
            score: None,
            data: &voice,
        },
        DimensionRow {
            dimension: D_CHECKLIST,
            tier: "t1",
            score: None,
            data: &checklist,
        },
        DimensionRow {
            dimension: D_ASSESSMENT,
            tier: "t1",
            score: Some(analysis.assessment.overall as f32),
            data: &assessment,
        },
    ];

    save_analysis_bundle(conn, project_id, input_hash, &dimensions, &concerns)
        .map_err(|e| e.to_string())
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
    analyze_project_inner(&state, &project_id, force.unwrap_or(false)).map(|(analysis, _)| analysis)
}

/// One project's outcome of a batch comparison: its analysis, or the reason
/// it failed.
///
/// A batch used to fail as a whole on the first broken project — one stale
/// row hid every column the comparison was about. Carrying the outcome per
/// project lets the page show the columns that can answer and name the one
/// that cannot.
#[derive(Debug, Clone, Serialize)]
pub struct ProjectAnalysisOutcome {
    pub project_id: String,
    pub analysis: Option<ProjectAnalysis>,
    pub error: Option<String>,
}

/// Run T0/T1 analysis on multiple projects for cross-project comparison.
///
/// `async` for the same reason as [`analyze_project`]: keep the heavy work off
/// the main/UI thread so the frontend stays responsive. Comparison always reads
/// the cache when it is valid — there is no UI affordance to force it here.
/// Each project answers for itself; one failure does not void the batch.
#[tauri::command]
pub(crate) async fn analyze_projects(
    state: State<'_, AppState>,
    project_ids: Vec<String>,
) -> Result<Vec<ProjectAnalysisOutcome>, String> {
    Ok(project_ids
        .iter()
        .map(|id| match analyze_project_inner(&state, id, false) {
            Ok((analysis, _)) => ProjectAnalysisOutcome {
                project_id: id.clone(),
                analysis: Some(analysis),
                error: None,
            },
            Err(e) => ProjectAnalysisOutcome {
                project_id: id.clone(),
                analysis: None,
                error: Some(e),
            },
        })
        .collect())
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
