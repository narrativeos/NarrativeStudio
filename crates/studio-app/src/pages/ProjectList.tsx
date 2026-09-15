import { useEffect, useState, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";

interface Project {
  project_id: string;
  name: string;
  genre: string | null;
  word_count: number | null;
  chapter_count: number | null;
  source_path: string | null;
  created_at: string;
  updated_at: string;
}

interface ImportProgress {
  type: "status" | "progress" | "done" | "error";
  message?: string;
  current?: number;
  total?: number;
  project_id?: string;
}

function ProjectList() {
  const [projects, setProjects] = useState<Project[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [importing, setImporting] = useState(false);
  const [progress, setProgress] = useState<ImportProgress | null>(null);
  const unlistenRef = useRef<UnlistenFn | null>(null);

  useEffect(() => {
    loadProjects();
    return () => {
      if (unlistenRef.current) unlistenRef.current();
    };
  }, []);

  async function loadProjects() {
    try {
      const result = await invoke<Project[]>("list_projects");
      setProjects(result);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }

  async function handleImport() {
    // 选择 TraceView 项目文件夹（包含 project.json）
    const folderPath = await open({
      title: "选择 TraceView 项目文件夹",
      directory: true,
      multiple: false,
    });
    if (!folderPath || typeof folderPath !== "string") return;

    setImporting(true);
    setError(null);
    setProgress(null);

    // 监听进度事件
    unlistenRef.current = await listen<ImportProgress>("import-progress", (event) => {
      setProgress(event.payload);
      if (event.payload.type === "done") {
        setImporting(false);
        setProgress(null);
        loadProjects();
      }
    });

    try {
      await invoke("import_project", { projectPath: folderPath });
      // 如果没收到 done 事件（兜底）
      setImporting(false);
      setProgress(null);
      await loadProjects();
    } catch (e) {
      setError(String(e));
      setImporting(false);
      setProgress(null);
    } finally {
      if (unlistenRef.current) {
        unlistenRef.current();
        unlistenRef.current = null;
      }
    }
  }

  if (loading) {
    return <div className="text-text-muted">Loading projects...</div>;
  }

  if (error) {
    return (
      <div className="text-red-400">
        <p>Failed to load projects:</p>
        <pre className="text-xs mt-2">{error}</pre>
      </div>
    );
  }

  return (
    <div>
      <div className="flex items-center justify-between mb-4">
        <h2 className="text-xl font-semibold">Projects</h2>
        <button
          onClick={handleImport}
          disabled={importing}
          className="px-4 py-2 bg-accent/20 text-accent rounded-md hover:bg-accent/30 transition-colors disabled:opacity-50"
        >
          {importing ? "Importing..." : "+ Import Project"}
        </button>
      </div>

      {/* Import progress panel */}
      {importing && (
        <div className="mb-4 p-4 bg-surface-alt rounded-lg border border-accent/30">
          <div className="flex items-center gap-2 mb-2">
            <div className="w-4 h-4 border-2 border-accent border-t-transparent rounded-full animate-spin" />
            <span className="text-sm text-accent font-medium">Importing...</span>
          </div>
          {progress && (
            <>
              <p className="text-sm text-text-muted">{progress.message}</p>
              {progress.type === "progress" && progress.total != null && progress.total > 0 && (
                <div className="mt-2">
                  <div className="w-full h-2 bg-gray-700 rounded-full overflow-hidden">
                    <div
                      className="h-full bg-accent transition-all duration-300"
                      style={{
                        width: `${Math.round(((progress.current ?? 0) / progress.total) * 100)}%`,
                      }}
                    />
                  </div>
                  <p className="text-xs text-text-muted mt-1">
                    {(progress.current ?? 0).toLocaleString()} / {progress.total.toLocaleString()}
                  </p>
                </div>
              )}
            </>
          )}
        </div>
      )}

      {projects.length === 0 ? (
        <div className="text-center py-12 text-text-muted">
          <p className="text-4xl mb-4">📂</p>
          <p>No projects yet.</p>
          <p className="text-sm mt-2">
            Select a TraceView project folder (containing project.json) to import.
          </p>
        </div>
      ) : (
        <div className="grid grid-cols-1 gap-3">
          {projects.map((p) => (
            <div
              key={p.project_id}
              className="p-4 bg-surface-alt rounded-lg border border-gray-700 hover:border-accent/50 transition-colors cursor-pointer"
            >
              <h3 className="font-medium text-text">{p.name}</h3>
              <div className="flex gap-4 mt-2 text-xs text-text-muted">
                {p.genre && <span>Genre: {p.genre}</span>}
                {p.word_count && <span>{p.word_count.toLocaleString()} words</span>}
                {p.chapter_count && <span>{p.chapter_count} documents</span>}
                <span>{new Date(p.created_at).toLocaleDateString()}</span>
              </div>
              {p.source_path && (
                <p className="text-xs text-text-muted/60 mt-1 truncate" title={p.source_path}>
                  📁 {p.source_path}
                </p>
              )}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

export default ProjectList;