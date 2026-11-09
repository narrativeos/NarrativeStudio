/// <reference types="vite/client" />
import { describe, expect, it } from "vitest";

// Read every source as text — no Node APIs, so this runs in whatever environment
// Vitest is configured with. Tests are skipped: they may quote a palette colour to
// prove a component does not use one.
const raw = import.meta.glob("../**/*.{ts,tsx}", {
  query: "?raw",
  import: "default",
  eager: true,
}) as Record<string, string>;

const files = Object.entries(raw).filter(([path]) => !path.endsWith(".test.ts"));
const css = import.meta.glob("../index.css", {
  query: "?raw",
  import: "default",
  eager: true,
})["../index.css"] as string;

/**
 * Palette colours belong in `index.css`, where a theme switch can reach them.
 * A component that names `bg-red-400` directly is invisible to both themes, so
 * it must name a token (`bg-bad`) or an identity hue (`bg-cat-pos`) instead.
 */
describe("colour tokens", () => {
  it("scans a non-trivial number of sources", () => {
    expect(files.length).toBeGreaterThan(20);
  });

  it("reads a stylesheet that went through the compiler", () => {
    // Guards the other four: a stubbed-out CSS import would pass them vacuously.
    expect(css).toContain("--color-text");
  });

  it("names no palette colour from a component", () => {
    const palette =
      /\b(?:bg|text|border|fill|stroke|ring|from|to|via)-(?:gray|slate|zinc|neutral|stone|red|rose|orange|amber|yellow|lime|green|emerald|teal|cyan|sky|blue|indigo|violet|purple|fuchsia|pink)-\d{2,3}/g;
    const offenders = files.flatMap(([path, text]) =>
      [...text.matchAll(palette)].map((m) => `${path}: ${m[0]}`),
    );
    expect(offenders).toEqual([]);
  });

  it("names no raw hex or rgba colour in a component", () => {
    const rawColor = /#[0-9a-fA-F]{6}\b|rgba?\(/g;
    const offenders = files.flatMap(([path, text]) =>
      [...text.matchAll(rawColor)].map((m) => `${path}: ${m[0]}`),
    );
    expect(offenders).toEqual([]);
  });

  it("only references tokens the stylesheet defines", () => {
    const defined = new Set([...css.matchAll(/(--color-[a-z0-9-]+)/g)].map((m) => m[1]));
    const missing = files.flatMap(([path, text]) =>
      [...text.matchAll(/var\(--color-[a-z0-9-]+\)/g)]
        .map((m) => m[0])
        .filter((v) => !defined.has(v.slice(4, -1)))
        .map((v) => `${path}: ${v}`),
    );
    expect(missing).toEqual([]);
  });

  it("defines every semantic token in both themes", () => {
    // Read from the compiled stylesheet, where Tailwind has written out the theme
    // — quotes and whitespace there are the compiler's choice, not ours.
    const blocks = [...css.matchAll(/\[data-theme=["']?(\w+)["']?\]\s*\{([^}]*)\}/g)];
    expect(blocks.map((b) => b[1]).sort()).toEqual(["dark", "light"]);
    const semantic = [
      "surface",
      "surface-alt",
      "line",
      "line-strong",
      "fill",
      "fill-soft",
      "fill-strong",
      "text",
      "text-muted",
      "accent",
      "good",
      "warn",
      "bad",
      "firm",
      "info",
      "bad-solid",
    ];
    for (const [, theme, body] of blocks) {
      for (const name of semantic) {
        expect(body, `${theme} theme is missing --${name}`).toContain(`--${name}:`);
      }
    }
  });
});