import { useEffect, useState } from "react";
import { activeSectionId, ANALYSIS_SECTIONS } from "../lib/sections";

/** How far below the top edge of the window a heading has to clear to count as
 *  reached: the bar's own height plus the gap under it. Kept in step with the
 *  `scroll-mt-24` on `Section`. */
const BAR_OFFSET = 96;

/**
 * The jump bar for the analysis page.
 *
 * Seventeen dimensions in one column is a lot of scrolling to answer "how is the
 * pacing?", and the design asks for a dimension navigator. This is the version
 * that fits the page rather than the sidebar: a bar that sticks to the top of
 * the content column, names every dimension, and marks the one being read.
 *
 * The highlight follows the reader. `window`'s scroll listener is registered in
 * the capture phase on purpose — the page scrolls inside `<main>`, not inside
 * the window, and capture is what lets one listener see a scroll of any element
 * in the tree without hunting for the scroll container.
 */
function SectionNav() {
  const [active, setActive] = useState<string | null>(ANALYSIS_SECTIONS[0].id);

  useEffect(() => {
    const elements = ANALYSIS_SECTIONS.map((s) => document.getElementById(s.id)).filter(
      (el): el is HTMLElement => el !== null,
    );
    if (elements.length === 0) return;

    let frame = 0;
    const measure = () => {
      frame = 0;
      setActive(
        activeSectionId(
          elements.map((el) => el.id),
          elements.map((el) => el.getBoundingClientRect().top - BAR_OFFSET),
        ),
      );
    };
    const schedule = () => {
      if (!frame) frame = requestAnimationFrame(measure);
    };

    const events: Array<[string, EventListenerOptions | undefined]> = [
      ["scroll", { capture: true }],
      ["resize", undefined],
    ];
    for (const [type, options] of events) {
      window.addEventListener(type, schedule, options);
    }
    schedule();

    return () => {
      if (frame) cancelAnimationFrame(frame);
      for (const [type, options] of events) {
        window.removeEventListener(type, schedule, options);
      }
    };
  }, []);

  function jump(id: string) {
    // A jump is an answer to an action, so it moves — but a reader who asked the
    // system to reduce motion gets the destination instead of the trip.
    const reduce = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    document.getElementById(id)?.scrollIntoView({
      behavior: reduce ? "auto" : "smooth",
      block: "start",
    });
    setActive(id);
  }

  return (
    <nav
      aria-label="分析维度"
      className="sticky top-0 z-10 -mx-6 mb-6 border-b border-line bg-surface px-6 py-2"
    >
      <div className="flex flex-wrap gap-x-1 gap-y-1">
        {ANALYSIS_SECTIONS.map((s) => (
          <button
            key={s.id}
            onClick={() => jump(s.id)}
            aria-current={active === s.id ? "true" : undefined}
            className={`rounded px-2 py-1 text-xs transition-colors ${
              active === s.id
                ? "bg-fill text-text underline decoration-accent decoration-2 underline-offset-4"
                : "text-text-muted hover:bg-fill/50 hover:text-text"
            }`}
          >
            {s.label}
          </button>
        ))}
      </div>
    </nav>
  );
}

export default SectionNav;
