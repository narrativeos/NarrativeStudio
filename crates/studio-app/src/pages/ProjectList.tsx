import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface Project {
  project_id: string;
  name: string;
  genre: string | null;
  word_count: number | null;
  chapter_count: number | null;
  created_at: string;
  updated_at: string;
}

function ProjectList() {
  const [projects, setProjects] = useState<Project[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    loadProjects();
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
          onClick={() => alert("Import coming in PR-4")}
          className="px-4 py-2 bg-accent/20 text-accent rounded-md hover:bg-accent/30 transition-colors"
        >
          + Import
        </button>
      </div>

      {projects.length === 0 ? (
        <div className="text-center py-12 text-text-muted">
          <p className="text-4xl mb-4">📂</p>
          <p>No projects yet.</p>
          <p className="text-sm mt-2">
            Import a TraceView project to get started.
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
                {p.chapter_count && <span>{p.chapter_count} chapters</span>}
                <span>{new Date(p.created_at).toLocaleDateString()}</span>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

export default ProjectList;