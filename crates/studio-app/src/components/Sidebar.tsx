import { useState } from "react";
import type { ThemeChoice } from "../lib/theme";
import type { Project } from "../types";

type View = "home" | "project" | "compare" | "settings";

interface SidebarProps {
  projects: Project[];
  view: View;
  selectedProjectId: string | null;
  compareSelection: Set<string>;
  onSelectProject: (id: string) => void;
  onEnterCompare: () => void;
  onExitCompare: () => void;
  onToggleCompare: (id: string) => void;
  onOpenSettings: () => void;
  onDeleteProject: (id: string) => void;
  themeChoice: ThemeChoice;
  onThemeChoice: (choice: ThemeChoice) => void;
}

const THEME_CHOICES: { value: ThemeChoice; label: string }[] = [
  { value: "system", label: "跟随系统" },
  { value: "light", label: "浅色" },
  { value: "dark", label: "暗色" },
];

function Sidebar({
  projects,
  view,
  selectedProjectId,
  compareSelection,
  onSelectProject,
  onEnterCompare,
  onExitCompare,
  onToggleCompare,
  onOpenSettings,
  onDeleteProject,
  themeChoice,
  onThemeChoice,
}: SidebarProps) {
  const isCompare = view === "compare";
  const [confirmId, setConfirmId] = useState<string | null>(null);

  return (
    <aside className="w-64 bg-surface-alt border-r border-line flex flex-col">
      <div className="p-4 border-b border-line">
        <h1 className="text-lg font-bold text-accent">NarrativeStudio</h1>
        <p className="text-xs text-text-muted mt-1">Multi-dimensional Analysis</p>
      </div>

      <div className="px-3 pt-3 pb-1 text-xs font-semibold uppercase tracking-wide text-text-muted">
        {isCompare ? "Compare" : "Projects"}
      </div>

      <nav className="flex-1 overflow-y-auto px-2 pb-2">
        {projects.length === 0 && (
          <p className="text-xs text-text-muted px-3 py-2">No projects yet.</p>
        )}
        {projects.map((p) => {
          const active = isCompare
            ? compareSelection.has(p.project_id)
            : selectedProjectId === p.project_id;
          return (
            <div key={p.project_id} className="group relative mb-1">
              <button
                onClick={() =>
                  isCompare ? onToggleCompare(p.project_id) : onSelectProject(p.project_id)
                }
                className={`w-full text-left px-3 py-2 rounded-md transition-colors flex items-center gap-2 ${
                  !isCompare ? "pr-8" : ""
                } ${
                  active
                    ? "bg-accent/20 text-accent"
                    : "text-text-muted hover:text-text hover:bg-fill/50"
                }`}
              >
                {isCompare && (
                  <span
                    className={`inline-flex w-4 h-4 shrink-0 items-center justify-center rounded border text-[10px] ${
                      active ? "border-accent bg-accent text-surface" : "border-line-strong"
                    }`}
                  >
                    {active ? "✓" : ""}
                  </span>
                )}
                <span className="truncate">{p.name}</span>
              </button>
              {!isCompare && (
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    setConfirmId(p.project_id);
                  }}
                  className="absolute right-1 top-1/2 -translate-y-1/2 p-1 rounded text-text-muted hover:text-bad hover:bg-fill/50 opacity-0 group-hover:opacity-100 transition-opacity"
                  title="删除项目"
                  aria-label="删除项目"
                >
                  🗑️
                </button>
              )}
            </div>
          );
        })}

        {confirmId && (
          <div className="mx-1 mt-2 p-2 rounded-md bg-bad/15 border border-bad/40 text-xs">
            <p className="text-bad mb-2">
              删除「{projects.find((p) => p.project_id === confirmId)?.name}」及其全部导入数据？此操作不可撤销。
            </p>
            <div className="flex gap-2">
              <button
                onClick={() => {
                  const id = confirmId;
                  setConfirmId(null);
                  onDeleteProject(id);
                }}
                className="flex-1 px-2 py-1 rounded bg-bad-solid hover:bg-bad/80 text-white"
              >
                删除
              </button>
              <button
                onClick={() => setConfirmId(null)}
                className="flex-1 px-2 py-1 rounded bg-fill hover:bg-fill-strong text-text-muted"
              >
                取消
              </button>
            </div>
          </div>
        )}
      </nav>

      <div className="p-2 border-t border-line space-y-1">
        <div className="px-3 pb-1">
          <div className="text-xs text-text-muted mb-1">主题</div>
          <div className="grid grid-cols-3 gap-1 rounded-md bg-fill-soft p-1" role="group">
            {THEME_CHOICES.map((c) => (
              <button
                key={c.value}
                onClick={() => onThemeChoice(c.value)}
                aria-pressed={themeChoice === c.value}
                className={`rounded px-1 py-1 text-xs transition-colors ${
                  themeChoice === c.value
                    ? "bg-accent/20 text-accent"
                    : "text-text-muted hover:text-text"
                }`}
              >
                {c.label}
              </button>
            ))}
          </div>
        </div>
        <button
          onClick={isCompare ? onExitCompare : onEnterCompare}
          className={`w-full text-left px-3 py-2 rounded-md transition-colors ${
            isCompare
              ? "bg-accent/20 text-accent"
              : "text-text-muted hover:text-text hover:bg-fill/50"
          }`}
        >
          <span className="mr-2">⚖️</span>
          {isCompare ? "Exit Compare" : "Compare Projects"}
        </button>
        <button
          onClick={onOpenSettings}
          className={`w-full text-left px-3 py-2 rounded-md transition-colors ${
            view === "settings"
              ? "bg-accent/20 text-accent"
              : "text-text-muted hover:text-text hover:bg-fill/50"
          }`}
        >
          <span className="mr-2">⚙️</span>Settings
        </button>
      </div>

      <div className="p-3 border-t border-line text-xs text-text-muted">v0.1.0</div>
    </aside>
  );
}

export default Sidebar;
