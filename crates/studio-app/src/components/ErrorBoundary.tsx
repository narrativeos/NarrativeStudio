import { Component, type ErrorInfo, type ReactNode } from "react";

interface Props {
  children: ReactNode;
}

interface State {
  error: Error | null;
}

/**
 * The last thing standing between a broken number and a dead window.
 *
 * Without a boundary, React 19 tears down the *entire* tree when any render
 * throws, and a Tauri window has no console open to say so: the page keeps the
 * pixels it last painted, so it looks intact while every button in it is inert.
 * That is what "the items stopped responding" looks like from the outside, and
 * it is why this belongs above the routes rather than around one panel.
 *
 * The message is the point. A stack in a devtools panel nobody can reach is not
 * a diagnosis, so the error text and — in a dev build — the frames go on screen.
 */
export default class ErrorBoundary extends Component<Props, State> {
  state: State = { error: null };

  static getDerivedStateFromError(error: Error): Partial<State> {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo): void {
    // Both surfaces: the banner for the reader, the real console for anyone with
    // devtools open. `componentDidCatch` is not a render, so this is safe.
    console.error(error, info.componentStack);
  }

  reload = () => {
    // A reload is the only recovery that does not depend on the state that
    // broke: the analysis cache is on disk, so the page comes back with it.
    window.location.reload();
  };

  render(): ReactNode {
    const { error } = this.state;
    if (!error) return this.props.children;

    return (
      <div className="p-6">
        <div
          role="alert"
          className="p-4 rounded-lg border border-bad/40 bg-bad/15 text-sm text-bad"
        >
          <h2 className="font-semibold mb-1">这一页在渲染时出错了</h2>
          <p className="mb-2">
            界面已经停止更新，所以这里的按钮不会再有反应。数据本身在磁盘上，重新载入即可回到项目。
          </p>
          <pre className="font-mono text-xs whitespace-pre-wrap break-words mb-3">
            {error.message}
            {import.meta.env.DEV && error.stack ? `\n${error.stack}` : ""}
          </pre>
          <button
            onClick={this.reload}
            className="px-3 py-1.5 rounded bg-accent-soft text-accent hover:bg-accent hover:text-surface"
          >
            重新载入
          </button>
        </div>
      </div>
    );
  }
}
