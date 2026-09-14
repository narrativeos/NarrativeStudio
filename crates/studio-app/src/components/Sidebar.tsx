type Page = "projects" | "analysis" | "settings";

interface SidebarProps {
  currentPage: Page;
  onNavigate: (page: Page) => void;
}

const navItems: { id: Page; label: string; icon: string }[] = [
  { id: "projects", label: "Projects", icon: "📁" },
  { id: "analysis", label: "Analysis", icon: "📊" },
  { id: "settings", label: "Settings", icon: "⚙️" },
];

function Sidebar({ currentPage, onNavigate }: SidebarProps) {
  return (
    <aside className="w-56 bg-surface-alt border-r border-gray-700 flex flex-col">
      <div className="p-4 border-b border-gray-700">
        <h1 className="text-lg font-bold text-accent">NarrativeStudio</h1>
        <p className="text-xs text-text-muted mt-1">Multi-dimensional Analysis</p>
      </div>
      <nav className="flex-1 p-2">
        {navItems.map((item) => (
          <button
            key={item.id}
            onClick={() => onNavigate(item.id)}
            className={`w-full text-left px-3 py-2 rounded-md mb-1 transition-colors ${
              currentPage === item.id
                ? "bg-accent/20 text-accent"
                : "text-text-muted hover:text-text hover:bg-gray-700/50"
            }`}
          >
            <span className="mr-2">{item.icon}</span>
            {item.label}
          </button>
        ))}
      </nav>
      <div className="p-3 border-t border-gray-700 text-xs text-text-muted">
        v0.1.0
      </div>
    </aside>
  );
}

export default Sidebar;