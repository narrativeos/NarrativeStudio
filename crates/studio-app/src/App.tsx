import { useState } from "react";
import Sidebar from "./components/Sidebar";
import ProjectList from "./pages/ProjectList";
import Analysis from "./pages/Analysis";

type Page = "projects" | "analysis" | "settings";

function App() {
  const [currentPage, setCurrentPage] = useState<Page>("projects");

  return (
    <div className="flex h-screen w-screen">
      <Sidebar currentPage={currentPage} onNavigate={setCurrentPage} />
      <main className="flex-1 overflow-auto p-6">
        {currentPage === "projects" && <ProjectList />}
        {currentPage === "analysis" && <Analysis />}
        {currentPage === "settings" && (
          <div className="text-text-muted">Settings (coming soon)</div>
        )}
      </main>
    </div>
  );
}

export default App;