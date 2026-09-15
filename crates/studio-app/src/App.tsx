import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import Sidebar from "./components/Sidebar";
import Home from "./pages/Home";
import Analysis from "./pages/Analysis";
import Compare from "./pages/Compare";
import type { Project } from "./types";

type View = "home" | "project" | "compare" | "settings";

function App() {
  const [view, setView] = useState<View>("home");
  const [projects, setProjects] = useState<Project[]>([]);
  const [selectedProjectId, setSelectedProjectId] = useState<string | null>(null);
  const [compareSelection, setCompareSelection] = useState<Set<string>>(new Set());

  useEffect(() => {
    loadProjects();
  }, []);

  async function loadProjects() {
    try {
      const result = await invoke<Project[]>("list_projects");
      setProjects(result);
    } catch (e) {
      console.error("Failed to load projects:", e);
    }
  }

  const selectProject = useCallback((id: string) => {
    setSelectedProjectId(id);
    setView("project");
  }, []);

  const goHome = useCallback(() => {
    setView("home");
    setSelectedProjectId(null);
  }, []);

  const enterCompare = useCallback(() => setView("compare"), []);

  const exitCompare = useCallback(() => {
    setCompareSelection(new Set());
    setView("home");
  }, []);

  const toggleCompare = useCallback((id: string) => {
    setCompareSelection((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  }, []);

  const openSettings = useCallback(() => setView("settings"), []);

  return (
    <div className="flex h-screen w-screen">
      <Sidebar
        projects={projects}
        view={view}
        selectedProjectId={selectedProjectId}
        compareSelection={compareSelection}
        onSelectProject={selectProject}
        onEnterCompare={enterCompare}
        onExitCompare={exitCompare}
        onToggleCompare={toggleCompare}
        onOpenSettings={openSettings}
      />
      <main className="flex-1 overflow-auto p-6">
        {view === "home" && (
          <Home projects={projects} onOpenProject={selectProject} onImported={loadProjects} />
        )}
        {view === "project" && selectedProjectId && (
          <Analysis projectId={selectedProjectId} onBack={goHome} />
        )}
        {view === "compare" && <Compare selection={compareSelection} />}
        {view === "settings" && (
          <div className="text-text-muted">Settings (coming soon)</div>
        )}
      </main>
    </div>
  );
}

export default App;
