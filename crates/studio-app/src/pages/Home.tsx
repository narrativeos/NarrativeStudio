import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import ProjectCard from "../components/ProjectCard";
import { FolderIcon } from "../components/Icon";
import { useShortcuts } from "../lib/shortcuts";
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

  // The two import paths are the first thing anyone needs a key for, and the
  // page is only this page while it is showing the list.
  useShortcuts({ import: handleImportText, "import-folder": handleImport });

  // Keep the log box scrolled to the newest line as it grows.
  useEffect(() => {
    if (logBoxRef.current) {
      logBoxRef.current.scrollTop = logBoxRef.current.scrollHeight;
    }
  }, [logs]);

  /**
   * Run one import: progress events in, one list refresh out.
   *
   * The "done" event used to refresh the list and so did the resolved invoke,
   * which fetched the project list twice per import. The event now only ends
   * the spinner; the refresh happens once, when the command resolves — the
   * one path that also knows the import truly finished.
   */
  async function runImport(command: string, args: Record<string, unknown>) {
    setImporting(true);
    setError(null);
    setLogs([]);
    setProgress(null);

    const unlisten = await listen<ImportProgress>("import-progress", (event) => {
      const p = event.payload;
      if ((p.type === "status" || p.type === "progress") && p.message) {
        setLogs((prev) => [...prev, p.message as string]);
      }
      if (p.type === "progress" && p.total != null && p.total > 0) {
        setProgress({ current: p.current ?? 0, total: p.total });
      }
      if (p.type === "done") {
        setImporting(false);
      }
      if (p.type === "error") {
        setError(p.message ?? "Import failed");
        setImporting(false);
      }
    });

    try {
      await invoke(command, args);
      onImported();
    } catch (e) {
      setError(String(e));
    } finally {
      setImporting(false);
      unlisten();
    }
  }

  async function handleImport() {
    const folderPath = await open({
      title: "选择 TraceView 项目文件夹",
      directory: true,
      multiple: false,
    });
    if (!folderPath || typeof folderPath !== "string") return;
    await runImport("import_project", { projectPath: folderPath });
  }

  async function handleImportText() {
    const filePath = await open({
      title: "选择 Markdown / TXT 文件",
      directory: false,
      multiple: false,
      filters: [{ name: "文本", extensions: ["md", "markdown", "mdown", "mkd", "txt"] }],
    });
    if (!filePath || typeof filePath !== "string") return;
    await runImport("import_text_file", { filePath });
  }

  return (
    <div>
      <div className="mb-4">
        <h2 className="text-xl font-semibold mb-3">项目</h2>
        <div className="flex items-center gap-2">
          <button
            onClick={handleImportText}
            disabled={importing}
            className="px-4 py-2 bg-surface-alt text-text rounded-md border border-line hover:bg-fill transition-colors disabled:opacity-50"
          >
            {importing ? "导入中…" : "导入 Markdown / TXT"}
          </button>
          <button
            onClick={handleImport}
            disabled={importing}
            className="px-4 py-2 bg-accent-soft text-accent rounded-md hover:bg-accent hover:text-surface transition-colors disabled:opacity-50"
          >
            {importing ? "导入中…" : "导入 TraceView 项目"}
          </button>
        </div>
        {/* What each entry brings the engine, said where it can be read before
            choosing rather than behind a hover on one button. A plain-text file
            carries no segmentation and no entities, and that gap is decided
            here — by the time the analysis page says so, the import is done. */}
        <p className="text-xs text-text-muted mt-2">
          Markdown / TXT 是一个文件一个项目，不带分词、词性与实体，因此词频、情节线、地点清单这几节会显示为读不到，其余维度照常。TraceView 项目目录带有全部输入。
        </p>
        {/* Named where they are used: a shortcut nobody can discover is a
            shortcut nobody has. */}
        <p className="text-xs text-text-muted mt-1">
          快捷键：<kbd className="font-sans">⌘O</kbd> 导入文件 ·{" "}
          <kbd className="font-sans">⌘⇧O</kbd> 导入项目目录
        </p>
      </div>

      {error && (
        <div className="text-bad mb-4">
          <p>导入失败：</p>
          <pre className="text-xs mt-2 whitespace-pre-wrap">{error}</pre>
        </div>
      )}

      {importing && (
        <div className="mb-4 p-4 bg-surface-alt rounded-lg border border-accent/30">
          <div className="flex items-center gap-2 mb-2">
            <div className="w-4 h-4 border-2 border-accent border-t-transparent rounded-full animate-spin" />
            <span className="text-sm text-accent font-medium">导入中…</span>
          </div>
          {progress && progress.total > 0 && (
            <div className="mt-1">
              <div className="w-full h-2 bg-fill rounded-full overflow-hidden">
                <div
                  className="h-full bg-accent transition-all duration-200"
                  style={{
                    width: `${Math.min(100, Math.round((progress.current / progress.total) * 100))}%`,
                  }}
                />
              </div>
              <p className="text-xs text-text-muted mt-1">
                {progress.current.toLocaleString()} / {progress.total.toLocaleString()} 个文本块
              </p>
            </div>
          )}
          <div
            ref={logBoxRef}
            className="mt-3 max-h-48 overflow-y-auto bg-fill-soft rounded-md p-2 font-mono text-xs text-text-muted space-y-1"
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
          <p className="mb-4 flex justify-center">
            <FolderIcon className="h-12 w-12" />
          </p>
          <p>还没有项目。</p>
          <p className="text-sm mt-2 max-w-md mx-auto">
            导入一个 Markdown 或 TXT 文件，或一个包含 <code>project.json</code> 的 TraceView
            项目目录。导入完成后这里会出现项目卡片，点进去就是分析。
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
