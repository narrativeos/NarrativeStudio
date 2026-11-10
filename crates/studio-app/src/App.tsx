import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import Sidebar from "./components/Sidebar";
import Home from "./pages/Home";
import Analysis from "./pages/Analysis";
import Compare from "./pages/Compare";
import { applyTheme, readChoice, writeChoice, type ThemeChoice } from "./lib/theme";
import type { Project } from "./types";

type View = "home" | "project" | "compare" | "settings";

function App() {
  const [view, setView] = useState<View>("home");
  const [projects, setProjects] = useState<Project[]>([]);
  const [selectedProjectId, setSelectedProjectId] = useState<string | null>(null);
  const [compareSelection, setCompareSelection] = useState<Set<string>>(new Set());
  const [themeChoice, setThemeChoice] = useState<ThemeChoice>(() =>
    readChoice(localStorage),
  );

  // The pick lives in React; the OS preference lives in a media query that can
  // change while the window is open, so both are re-applied together.
  useEffect(() => {
    const media = window.matchMedia("(prefers-color-scheme: dark)");
    const paint = () => applyTheme(themeChoice, media.matches);
    paint();
    media.addEventListener("change", paint);
    return () => media.removeEventListener("change", paint);
  }, [themeChoice]);

  const chooseTheme = useCallback((choice: ThemeChoice) => {
    writeChoice(localStorage, choice);
    setThemeChoice(choice);
  }, []);

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

  async function deleteProject(id: string) {
    try {
      await invoke("delete_project", { projectId: id });
      if (selectedProjectId === id) {
        setSelectedProjectId(null);
        setView("home");
      }
      await loadProjects();
    } catch (e) {
      console.error("Failed to delete project:", e);
      window.alert(`删除项目失败：${e}`);
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
        onDeleteProject={deleteProject}
        themeChoice={themeChoice}
        onThemeChoice={chooseTheme}
      />
      <main className="flex-1 overflow-auto">
        {/* The padding lives inside the scroller rather than on it: a sticky
            element offsets from the scrollport's padding edge, so padding on
            `main` itself would leave a band above the dimension bar that the
            page scrolls through. */}
        <div className="p-6">
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
        </div>
      </main>
    </div>
  );
}

export default App;
