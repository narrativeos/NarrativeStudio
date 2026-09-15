import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import ProjectCard from "../components/ProjectCard";
import type { Project } from "../types";

interface ImportProgress {
  type: "status" | "progress" | "done" | "error";
  message?: string;
  current?: number;
  total?: number;
  project_id?: string;
}

interface HomeProps {
  projects: Project[];
  onOpenProject: (id: string) => void;
  onImported: () => void;
}

function Home({ projects, onOpenProject, onImported }: HomeProps) {
  const [importing, setImporting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [logs, setLogs] = useState<string[]>([]);
  const [progress, setProgress] = useState<{ current: number; total: number } | null>(null);
  const logBoxRef = useRef<HTMLDivElement | null>(null);
  const unlistenRef = useRef<UnlistenFn | null>(null);

  // Keep the log box scrolled to the newest line as it grows.
  useEffect(() => {
    if (logBoxRef.current) {
      logBoxRef.current.scrollTop = logBoxRef.current.scrollHeight;
    }
  }, [logs]);

  async function handleImport() {
    const folderPath = await open({
      title: "选择 TraceView 项目文件夹",
      directory: true,
      multiple: false,
    });
    if (!folderPath || typeof folderPath !== "string") return;

    setImporting(true);
    setError(null);
    setLogs([]);
    setProgress(null);

    unlistenRef.current = await listen<ImportProgress>("import-progress", (event) => {
      const p = event.payload;
      if ((p.type === "status" || p.type === "progress") && p.message) {
        setLogs((prev) => [...prev, p.message as string]);
      }
      if (p.type === "progress" && p.total != null && p.total > 0) {
        setProgress({ current: p.current ?? 0, total: p.total });
      }
      if (p.type === "done") {
        setImporting(false);
        onImported();
      }
      if (p.type === "error") {
        setError(p.message ?? "Import failed");
        setImporting(false);
      }
    });

    try {
      await invoke("import_project", { projectPath: folderPath });
      setImporting(false);
      onImported();
    } catch (e) {
      setError(String(e));
      setImporting(false);
    } finally {
      if (unlistenRef.current) {
        unlistenRef.current();
        unlistenRef.current = null;
      }
    }
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

      {error && (
        <div className="text-red-400 mb-4">
          <p>Import failed:</p>
          <pre className="text-xs mt-2">{error}</pre>
        </div>
      )}

      {importing && (
        <div className="mb-4 p-4 bg-surface-alt rounded-lg border border-accent/30">
          <div className="flex items-center gap-2 mb-2">
            <div className="w-4 h-4 border-2 border-accent border-t-transparent rounded-full animate-spin" />
            <span className="text-sm text-accent font-medium">Importing...</span>
          </div>
          {progress && progress.total > 0 && (
            <div className="mt-1">
              <div className="w-full h-2 bg-gray-700 rounded-full overflow-hidden">
                <div
                  className="h-full bg-accent transition-all duration-200"
                  style={{
                    width: `${Math.min(100, Math.round((progress.current / progress.total) * 100))}%`,
                  }}
                />
              </div>
              <p className="text-xs text-text-muted mt-1">
                {progress.current.toLocaleString()} / {progress.total.toLocaleString()} blocks
              </p>
            </div>
          )}
          <div
            ref={logBoxRef}
            className="mt-3 max-h-48 overflow-y-auto bg-black/40 rounded-md p-2 font-mono text-xs text-text-muted space-y-1"
          >
            {logs.map((line, i) => (
              <div key={i} className="whitespace-pre-wrap break-words">
                {line}
              </div>
            ))}
          </div>
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
            <ProjectCard key={p.project_id} project={p} onOpen={onOpenProject} />
          ))}
        </div>
      )}
    </div>
  );
}

export default Home;
