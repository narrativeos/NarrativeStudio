import type { Project } from "../types";

interface ProjectCardProps {
  project: Project;
  onOpen: (id: string) => void;
}

/**
 * A project in the list.
 *
 * The card is the primary action of this page, so it is a `button` rather than a
 * `div` with an `onClick`: focusable, activated by Space and Enter, and announced
 * with its name. A reader who never touches the trackpad can still open a project.
 *
 * Its contents are spans because a button's content model is phrasing content —
 * the layout comes from classes, not from the element.
 */
function ProjectCard({ project, onOpen }: ProjectCardProps) {
  return (
    <button
      onClick={() => onOpen(project.project_id)}
      className="w-full text-left p-4 bg-surface-alt rounded-lg border border-line hover:border-accent/50 transition-colors"
    >
      <span className="block font-medium text-text">{project.name}</span>
      <span className="flex gap-4 mt-2 text-xs text-text-muted">
        {project.genre && <span>类型：{project.genre}</span>}
        {project.word_count != null && (
          <span className="tabular-nums">{project.word_count.toLocaleString()} 字</span>
        )}
        {project.chapter_count != null && (
          <span className="tabular-nums">{project.chapter_count} 篇文档</span>
        )}
        <span>{new Date(project.created_at).toLocaleDateString()}</span>
      </span>
      {project.source_path && (
        <span
          className="block text-xs text-text-muted/60 mt-1 truncate"
          title={project.source_path}
        >
          📁 {project.source_path}
        </span>
      )}
    </button>
  );
}

export default ProjectCard;
