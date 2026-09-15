import { useRef, useState } from "react";
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
  const [progress, setProgress] = useState<ImportProgress | null>(null);
  const unlistenRef = useRef<UnlistenFn | null>(null);

  async function handleImport() {
    const folderPath = await open({
      title: "选择 TraceView 项目文件夹",
      directory: true,
      multiple: false,
    });
    if (!folderPath || typeof folderPath !== "string") return;

    setImporting(true);
    setError(null);
    setProgress(null);

    unlistenRef.current = await listen<ImportProgress>("import-progress", (event) => {
      setProgress(event.payload);
      if (event.payload.type === "done") {
        setImporting(false);
        setProgress(null);
        onImported();
      }
    });

    try {
      await invoke("import_project", { projectPath: folderPath });
      setImporting(false);
      setProgress(null);
      onImported();
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
          {progress && (
            <>
              <p className="text-sm text-text-muted">{progress.message}</p>
              {progress.type === "progress" &&
                progress.total != null &&
                progress.total > 0 && (
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
            <ProjectCard key={p.project_id} project={p} onOpen={onOpenProject} />
          ))}
        </div>
      )}
    </div>
  );
}

export default Home;
