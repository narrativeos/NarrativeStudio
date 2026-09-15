import type { Project } from "../types";

interface ProjectCardProps {
  project: Project;
  onOpen: (id: string) => void;
}

function ProjectCard({ project, onOpen }: ProjectCardProps) {
  return (
    <div
      onClick={() => onOpen(project.project_id)}
      className="p-4 bg-surface-alt rounded-lg border border-gray-700 hover:border-accent/50 transition-colors cursor-pointer"
    >
      <h3 className="font-medium text-text">{project.name}</h3>
      <div className="flex gap-4 mt-2 text-xs text-text-muted">
        {project.genre && <span>Genre: {project.genre}</span>}
        {project.word_count != null && (
          <span>{project.word_count.toLocaleString()} words</span>
        )}
        {project.chapter_count != null && (
          <span>{project.chapter_count} documents</span>
        )}
        <span>{new Date(project.created_at).toLocaleDateString()}</span>
      </div>
      {project.source_path && (
        <p className="text-xs text-text-muted/60 mt-1 truncate" title={project.source_path}>
          📁 {project.source_path}
        </p>
      )}
    </div>
  );
}

export default ProjectCard;
