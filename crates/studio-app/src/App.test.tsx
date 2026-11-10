// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from "vitest";
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import App from "./App";
import ErrorBoundary from "./components/ErrorBoundary";
import type { Project } from "./types";

/**
 * The app, clicked.
 *
 * The rest of the suite is pure logic, which is why a page whose click handlers
 * had stopped firing could still pass it: nothing here had ever rendered a
 * component or dispatched an event. These two cases are the floor — that a
 * control still reaches the code behind it, and that a render failure says so
 * instead of leaving a window that looks intact and answers nothing.
 */

const analysis = {
  invoke: vi.fn<(cmd: string, args?: unknown) => Promise<unknown>>(),
};

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (cmd: string, args?: unknown) => analysis.invoke(cmd, args),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: async () => () => {},
}));

vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: async () => null,
  save: async () => null,
}));

function project(id: string, name: string): Project {
  return {
    project_id: id,
    name,
    genre: null,
    word_count: 1200,
    chapter_count: 3,
    source_path: null,
    created_at: new Date("2026-01-04").toISOString(),
    updated_at: new Date("2026-01-04").toISOString(),
  };
}

let root: Root | undefined;
const host = document.createElement("div");
document.body.append(host);

/** React 19 only warns — and only flushes effects synchronously — when told. */
function render(node: React.ReactNode) {
  if (!root) {
    act(() => {
      root = createRoot(host);
    });
  }
  act(() => {
    root!.render(node);
  });
}

function click(el: Element) {
  act(() => {
    el.dispatchEvent(new MouseEvent("click", { bubbles: true }));
  });
}

beforeEach(() => {
  // Each case gets its own App. A reused root keeps the previous case's view and
  // selection, which is state these tests are not asserting on and would rather
  // not have decide whether a row is on screen.
  if (root) {
    act(() => {
      root!.unmount();
    });
    root = undefined;
  }
  (globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT =
    true;
  localStorage.clear();
  host.replaceChildren();
  vi.stubGlobal(
    "matchMedia",
    vi.fn((query: string) => ({
      matches: false,
      media: query,
      onchange: null,
      addEventListener: () => {},
      removeEventListener: () => {},
      addListener: () => {},
      removeListener: () => {},
      dispatchEvent: () => false,
    })),
  );
  analysis.invoke = vi.fn(async (cmd: string) =>
    cmd === "list_projects" ? [project("p1", "测试稿")] : {},
  );
});

describe("the running app", () => {
  it("opens a project when its card is clicked", async () => {
    render(<App />);
    // The list arrives through an effect, so let it land before reaching for a card.
    await act(async () => {});

    const card = [...host.querySelectorAll("button")].find((b) =>
      b.textContent?.includes("测试稿"),
    );
    expect(card, "the project card should be on screen").toBeTruthy();

    await act(async () => {
      card!.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });

    // The analysis page is the only thing that asks for a project's numbers, so
    // this is the click being wired to the code behind it.
    expect(analysis.invoke).toHaveBeenCalledWith("analyze_project", {
      projectId: "p1",
      force: false,
    });
  });

  it("keeps the sidebar row working as a second way in", async () => {
    render(<App />);
    await act(async () => {});

    const row = [...host.querySelectorAll("nav button")].find((b) =>
      b.textContent?.includes("测试稿"),
    );
    expect(row, "the sidebar should list the project").toBeTruthy();

    await act(async () => {
      click(row!);
    });

    expect(analysis.invoke).toHaveBeenCalledWith("analyze_project", {
      projectId: "p1",
      force: false,
    });
  });

  it("says what is missing instead of going dead when the cache is stale", async () => {
    // This is the bug the page had: `invoke<ProjectAnalysis>` is a cast, so an
    // analysis stored by an older engine reached the render intact-looking, and
    // the first `t1.sections` threw — which does not blank one panel, it takes
    // the whole window's buttons with it and leaves no message behind.
    analysis.invoke = vi.fn(async (cmd: string) =>
      cmd === "list_projects" ? [project("p1", "测试稿")] : { project_id: "p1" },
    );
    render(<App />);
    await act(async () => {});

    const card = [...host.querySelectorAll("button")].find((b) =>
      b.textContent?.includes("测试稿"),
    );
    await act(async () => {
      card!.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });

    expect(host.textContent).toContain("缓存的分析结果不完整");
    expect(host.textContent).toContain("t1 缺失或不是对象");
    // And the page still answers: the way out is on it.
    const retry = [...host.querySelectorAll("button")].find(
      (b) => b.textContent === "重新分析",
    );
    expect(retry, "the incomplete panel offers the fix").toBeTruthy();
  });
});

describe("ErrorBoundary", () => {
  it("replaces a throwing page with something readable, not a dead window", () => {
    const spy = vi.spyOn(console, "error").mockImplementation(() => {});
    function Broken(): React.ReactElement {
      throw new Error("cannot read 节奏曲线");
    }

    render(
      <ErrorBoundary>
        <Broken />
      </ErrorBoundary>,
    );

    expect(host.textContent).toContain("这一页在渲染时出错了");
    // The message is the whole point: without it the reader has a window that
    // looks painted and responds to nothing, and no way to say what happened.
    expect(host.textContent).toContain("cannot read 节奏曲线");
    expect(host.querySelector("button")).toBeTruthy();
    spy.mockRestore();
  });
});
