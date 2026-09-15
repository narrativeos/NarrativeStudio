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
}

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
}: SidebarProps) {
  const isCompare = view === "compare";

  return (
    <aside className="w-64 bg-surface-alt border-r border-gray-700 flex flex-col">
      <div className="p-4 border-b border-gray-700">
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
            <button
              key={p.project_id}
              onClick={() =>
                isCompare ? onToggleCompare(p.project_id) : onSelectProject(p.project_id)
              }
              className={`w-full text-left px-3 py-2 rounded-md mb-1 transition-colors flex items-center gap-2 ${
                active
                  ? "bg-accent/20 text-accent"
                  : "text-text-muted hover:text-text hover:bg-gray-700/50"
              }`}
            >
              {isCompare && (
                <span
                  className={`inline-flex w-4 h-4 shrink-0 items-center justify-center rounded border text-[10px] ${
                    active ? "border-accent bg-accent text-surface" : "border-gray-500"
                  }`}
                >
                  {active ? "✓" : ""}
                </span>
              )}
              <span className="truncate">{p.name}</span>
            </button>
          );
        })}
      </nav>

      <div className="p-2 border-t border-gray-700 space-y-1">
        <button
          onClick={isCompare ? onExitCompare : onEnterCompare}
          className={`w-full text-left px-3 py-2 rounded-md transition-colors ${
            isCompare
              ? "bg-accent/20 text-accent"
              : "text-text-muted hover:text-text hover:bg-gray-700/50"
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
              : "text-text-muted hover:text-text hover:bg-gray-700/50"
          }`}
        >
          <span className="mr-2">⚙️</span>Settings
        </button>
      </div>

      <div className="p-3 border-t border-gray-700 text-xs text-text-muted">v0.1.0</div>
    </aside>
  );
}

export default Sidebar;
