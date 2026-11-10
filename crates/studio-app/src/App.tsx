import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import Sidebar from "./components/Sidebar";
import Home from "./pages/Home";
import Analysis from "./pages/Analysis";
import Compare from "./pages/Compare";
import { readBridge } from "./lib/bridge";
import { applyTheme, readChoice, writeChoice, type ThemeChoice } from "./lib/theme";
import type { Project } from "./types";

type View = "home" | "project" | "compare";

function App() {
  const [view, setView] = useState<View>("home");
  const [projects, setProjects] = useState<Project[]>([]);
  const [selectedProjectId, setSelectedProjectId] = useState<string | null>(null);
  const [compareSelection, setCompareSelection] = useState<Set<string>>(new Set());
  const [themeChoice, setThemeChoice] = useState<ThemeChoice>(() =>
    readChoice(localStorage),
  );
  const [problem, setProblem] = useState<string | null>(null);
  // Read once per render rather than kept in state: the bridge is decided by
  // which window this is and cannot change while the window is open.
  const bridge = readBridge();

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
    // Nothing here can succeed without the bridge, and trying produces a banner
    // per command that looks like damaged data. See `lib/bridge.ts`.
    if (bridge === "none") return;
    loadProjects();
    // `bridge` cannot change while the window is open, so this still runs once;
    // naming it keeps the effect honest rather than widening an existing warning.
  }, [bridge]);

  /**
   * Say something went wrong, without stopping anyone.
   *
   * This was `window.alert`. A modal native dialog freezes the whole window,
   * cannot be dismissed with Escape, takes focus away from whatever the reader
   * was in the middle of, and repeats itself for every failure in a row — which
   * for a failed batch of imports means clicking through the same box many
   * times. A banner states it once, stays until dismissed, and leaves the page
   * usable underneath.
   */
  function reportProblem(message: string) {
    console.error(message);
    setProblem(message);
  }

  async function loadProjects() {
    try {
      const result = await invoke<Project[]>("list_projects");
      setProjects(result);
    } catch (e) {
      // A list that comes back empty because the command failed is not an empty
      // library, and the empty state would cheerfully invite a first import.
      reportProblem(`读取项目列表失败：${String(e)}`);
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
      reportProblem(`删除项目失败：${String(e)}`);
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

  if (bridge === "none") {
    // Its own page rather than a banner, because a banner leaves an empty project
    // list underneath that reads as the library being empty. Nothing in this
    // window can be made to work, and the reason is which window it is.
    return (
      <div className="p-6">
        <div className="max-w-2xl p-4 rounded-lg border border-warn/40 bg-warn/15 text-sm">
          <h2 className="font-semibold mb-2">这个窗口没有连到 Tauri 后端</h2>
          <p className="mb-2">
            项目列表、导入和分析都要经过 Tauri 注入的 IPC 桥，而这个窗口里没有它。所以这里不会列出项目，也不是数据读坏了。
          </p>
          <p className="mb-2">
            若你是在浏览器里打开 <code className="font-mono text-xs">http://localhost:1422</code>
            ：那只能调样式和组件。要跑真实数据，请起完整的开发实例：
          </p>
          <pre className="font-mono text-xs mb-3 whitespace-pre-wrap">
            cd crates/studio-app && pnpm tauri dev
          </pre>
          <p>
            或在 VS Code 里用 launch.json 的「调试 Rust（Tauri 主进程）」。若这条提示出现在应用自己的窗口里，说明前后端不是同一次构建，重跑上面那条命令即可。
          </p>
        </div>
      </div>
    );
  }

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
          {problem && (
            <div
              role="alert"
              className="mb-4 flex items-start gap-3 p-3 rounded-lg border border-bad/40 bg-bad/15 text-sm text-bad"
            >
              <p className="flex-1">{problem}</p>
              <button
                onClick={() => setProblem(null)}
                aria-label="关闭提示"
                className="shrink-0 px-1.5 py-0.5 rounded hover:bg-fill/50"
              >
                ✕
              </button>
            </div>
          )}
          {view === "home" && (
            <Home projects={projects} onOpenProject={selectProject} onImported={loadProjects} />
          )}
          {view === "project" && selectedProjectId && (
            <Analysis projectId={selectedProjectId} onBack={goHome} />
          )}
          {view === "compare" && <Compare selection={compareSelection} />}
        </div>
      </main>
    </div>
  );
}

export default App;
