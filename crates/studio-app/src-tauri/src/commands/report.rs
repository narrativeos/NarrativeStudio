//! Report generation, Markdown and print-HTML export, and the Knowsphere
//! export — the machine-readable product consumed by NarratSphere.

use serde::Serialize;
use studio_analysis::document_hash;
use studio_report::{
    render_html, render_knowsphere, render_markdown, KnowsphereInput, ReportInput,
};
use studio_storage::{
    entity_stats_for_project, get_report, mark_exported, save_report, StoredReport,
};
use tauri::State;
use uuid::Uuid;

use crate::commands::analyze::{analyze_project_inner, load_project_document};
use crate::AppState;

/// A generated report: metadata plus the rendered Markdown, so the UI can name
/// the file sensibly and show what it is about to save.
#[derive(Debug, Clone, Serialize)]
pub struct ReportMeta {
    pub report_id: String,
    pub project_id: String,
    pub title: String,
    pub created_at: String,
    pub markdown: String,
    pub exported_path: Option<String>,
}

/// Build the Markdown report for a project and store it.
///
/// The numbers come from [`analyze_project_inner`], i.e. from the cache when it is
/// valid: a report always matches what the analysis page shows, and generating one
/// never pays for a recompute the UI already triggered.
#[tauri::command]
pub(crate) async fn report_generate(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<ReportMeta, String> {
    let pid = Uuid::parse_str(&project_id).map_err(|e| e.to_string())?;
    let analysis = analyze_project_inner(&state, &project_id, false)?;

    let title = format!("{} 分析报告", analysis.name);
    let created_at = chrono::Utc::now();
    let markdown = render_markdown(&ReportInput {
        project_name: &analysis.name,
        generated_at: created_at,
        t0: &analysis.t0,
        t1: &analysis.t1,
        plot: &analysis.plot,
        pacing: &analysis.pacing,
        conflict: &analysis.conflict,
        theme: &analysis.theme,
        setting: &analysis.setting,
        cliche: &analysis.cliche,
        dialogue: &analysis.dialogue,
        explicit: &analysis.explicit,
        structure: &analysis.structure,
        timeline: &analysis.timeline,
        voice: &analysis.voice,
        checklist: &analysis.checklist,
        assessment: &analysis.assessment,
    });

    let report = StoredReport {
        report_id: Uuid::new_v4(),
        project_id: pid,
        title: Some(title.clone()),
        markdown: markdown.clone(),
        exported_path: None,
        created_at,
    };
    // The connection is only taken after the analysis above released it, so
    // generating a report cannot deadlock against the analysis path.
    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    save_report(&storage.conn, &report).map_err(|e| e.to_string())?;

    Ok(ReportMeta {
        report_id: report.report_id.to_string(),
        project_id: pid.to_string(),
        title,
        created_at: created_at.to_rfc3339(),
        markdown,
        exported_path: None,
    })
}

/// Write a stored report to the path the user picked, then remember that path.
///
/// The file is written here rather than through the frontend FS plugin on purpose:
/// the dialog only needs to hand back a path, and no filesystem scope has to be
/// granted to the webview.
#[tauri::command]
pub(crate) async fn report_export(
    state: State<'_, AppState>,
    report_id: String,
    path: String,
) -> Result<String, String> {
    let rid = Uuid::parse_str(&report_id).map_err(|e| e.to_string())?;
    let markdown = {
        let storage = state.storage.lock().map_err(|e| e.to_string())?;
        get_report(&storage.conn, rid)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("报告不存在：{report_id}"))?
            .markdown
    };

    std::fs::write(&path, markdown).map_err(|e| format!("写入报告失败：{e}"))?;

    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    mark_exported(&storage.conn, rid, &path).map_err(|e| e.to_string())?;
    Ok(path)
}

/// Write the print-ready HTML of a stored report — the file the user prints to
/// PDF.
///
/// The HTML is rendered from the **stored Markdown** rather than stored beside
/// it: `render_html` is a pure function of that text, so a second column would
/// only be a second copy that can rot. The numbers in the printed file are
/// therefore still the ones persisted when the report was generated — printing
/// after a re-analysis cannot quietly pick up new ones.
#[tauri::command]
pub(crate) async fn report_export_html(
    state: State<'_, AppState>,
    report_id: String,
    path: String,
) -> Result<String, String> {
    let rid = Uuid::parse_str(&report_id).map_err(|e| e.to_string())?;
    let markdown = {
        let storage = state.storage.lock().map_err(|e| e.to_string())?;
        get_report(&storage.conn, rid)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("报告不存在：{report_id}"))?
            .markdown
    };

    std::fs::write(&path, render_html(&markdown)).map_err(|e| format!("写入报告失败：{e}"))?;

    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    mark_exported(&storage.conn, rid, &path).map_err(|e| e.to_string())?;
    Ok(path)
}

/// Write a `.knowsphere.json` file for a project — the export consumed by
/// NarratSphere.
///
/// The analysis comes from the same cache the analysis page reads, so the
/// file matches what the user saw; entity nodes carry registry-issued stable
/// ids so a re-export merges into hand-edited knowledge spheres by identity.
/// The document is validated against the schema contract *before* anything is
/// written: NarratSphere never receives a file this side already knows is
/// broken.
#[tauri::command]
pub(crate) async fn export_knowsphere(
    state: State<'_, AppState>,
    project_id: String,
    path: String,
) -> Result<String, String> {
    let pid = Uuid::parse_str(&project_id).map_err(|e| e.to_string())?;
    let analysis = analyze_project_inner(&state, &project_id, false)?;

    // Entity identities and the source hash come straight from the database:
    // the hash is the same content key the analysis cache is keyed on.
    let (entities, source_hash) = {
        let storage = state.storage.lock().map_err(|e| e.to_string())?;
        let entities = entity_stats_for_project(&storage.conn, pid).map_err(|e| e.to_string())?;
        let doc = load_project_document(&storage.conn, pid)?;
        let hash = document_hash(&doc).map_err(|e| e.to_string())?;
        (entities, hash)
    };

    let doc = render_knowsphere(&KnowsphereInput {
        project_name: &analysis.name,
        project_id: &project_id,
        generated_at: chrono::Utc::now(),
        source_hash: &source_hash,
        t1: &analysis.t1,
        plot: &analysis.plot,
        theme: &analysis.theme,
        setting: &analysis.setting,
        timeline: &analysis.timeline,
        assessment: &analysis.assessment,
        entities: &entities,
    })
    .map_err(|e| e.to_string())?;

    let issues = doc.validate();
    if !issues.is_empty() {
        return Err(format!(
            "知识球导出未通过 schema 自检，已停止写入：\n{}",
            issues.join("\n")
        ));
    }

    std::fs::write(&path, doc.serialize()).map_err(|e| format!("写入知识球文件失败：{e}"))?;
    Ok(path)
}
