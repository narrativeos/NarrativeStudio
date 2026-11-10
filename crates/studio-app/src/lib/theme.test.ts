/// <reference types="vite/client" />
import { describe, expect, it } from "vitest";
import { readChoice, resolveTheme, THEME_STORAGE_KEY, writeChoice } from "./theme";

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

const html = import.meta.glob("../../index.html", {
  query: "?raw",
  import: "default",
  eager: true,
})["../../index.html"] as string;

/**
 * Each `[data-theme="…"]` block as the compiler wrote it — quotes and whitespace
 * there are Tailwind's choice, not ours.
 */
function themeBlocks() {
  return [...css.matchAll(/\[data-theme=["']?(\w+)["']?\]\s*\{([^}]*)\}/g)];
}

function token(theme: string, name: string): string | null {
  const body = themeBlocks().find((b) => b[1] === theme)?.[2] ?? "";
  const value = new RegExp(`--${name}:\\s*([^;]+);`).exec(body)?.[1]?.trim() ?? "";
  // A theme may borrow a Tailwind palette colour (`var(--color-gray-700)`); those
  // ship with the framework and are legible by construction, so only our own hex
  // values are measured below.
  return /^#[0-9a-fA-F]{6}$/.test(value) ? value : null;
}

const channel = (hex: string, i: number) =>
  parseInt(hex.slice(1 + i * 2, 3 + i * 2), 16) / 255;
const linear = (c: number) => (c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4);
const luminance = (hex: string) =>
  0.2126 * linear(channel(hex, 0)) +
  0.7152 * linear(channel(hex, 1)) +
  0.0722 * linear(channel(hex, 2));

function contrast(a: string, b: string): number {
  const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return (hi + 0.05) / (lo + 0.05);
}

/** A 15%-on-surface badge fill: blended in light, which is close enough to the
 *  oklab mix the browser does to tell a passing pair from a failing one. */
function tint(fg: string, bg: string, alpha: number): string {
  return (
    "#" +
    [0, 1, 2]
      .map((i) => {
        const v = linear(channel(fg, i)) * alpha + linear(channel(bg, i)) * (1 - alpha);
        const c = v <= 0.0031308 ? v * 12.92 : 1.055 * v ** (1 / 2.4) - 0.055;
        return Math.round(Math.max(0, Math.min(255, c * 255)))
          .toString(16)
          .padStart(2, "0");
      })
      .join("")
  );
}

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
    const blocks = themeBlocks();
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
      "bad-on-solid",
      "accent-soft",
    ];
    for (const [, theme, body] of blocks) {
      for (const name of semantic) {
        expect(body, `${theme} theme is missing --${name}`).toContain(`--${name}:`);
      }
    }
  });
});

/**
 * A theme is only finished when its text can be read off its surfaces. These
 * numbers are the WCAG 2.1 thresholds; the badge floor is lower because a badge
 * tints its own colour into the background, which the shipped dark theme has
 * always done at around 3.4:1.
 */
describe("theme legibility", () => {
  const surfaces = ["surface", "surface-alt"];
  const inks = ["text", "text-muted", "accent"];
  const states = ["good", "warn", "bad", "firm", "info"];

  for (const theme of ["dark", "light"]) {
    it(`keeps ${theme} body text readable on both surfaces`, () => {
      for (const surface of surfaces) {
        const bg = token(theme, surface);
        expect(bg, `${theme} --${surface} should be a plain colour to measure`).not.toBeNull();
        for (const ink of inks) {
          const fg = token(theme, ink);
          if (!fg) continue;
          expect(contrast(fg, bg!), `${ink} on ${surface} in ${theme}`).toBeGreaterThanOrEqual(4.5);
        }
      }
    });

    it(`keeps ${theme} state colours readable on both surfaces`, () => {
      for (const surface of surfaces) {
        const bg = token(theme, surface)!;
        for (const state of states) {
          const fg = token(theme, state);
          if (!fg) continue;
          expect(contrast(fg, bg), `${state} on ${surface} in ${theme}`).toBeGreaterThanOrEqual(4.5);
          expect(
            contrast(fg, tint(fg, bg, 0.15)),
            `${state} badge in ${theme}`,
          ).toBeGreaterThanOrEqual(3);
        }
      }
    });

    it(`keeps ${theme} destructive buttons readable`, () => {
      // Measured between the two tokens, not against a literal white: the point
      // is that the pair the button actually uses holds in both themes.
      const solid = token(theme, "bad-solid");
      const ink = token(theme, "bad-on-solid");
      if (!solid || !ink) return;
      expect(contrast(ink, solid)).toBeGreaterThanOrEqual(4.5);
    });

    it(`keeps ${theme} accent text readable on its own tint`, () => {
      // The active nav row and the secondary primary buttons. These are the two
      // places that used `bg-accent/20`, where the fill is the accent itself at
      // 20% — which reads fine on the dark theme and at 3.81:1 on a light one,
      // because a tint's contrast depends on the surface under it. Measuring the
      // pair is what stops an alpha value coming back to do that again.
      const soft = token(theme, "accent-soft");
      const fg = token(theme, "accent");
      if (!soft || !fg) return;
      expect(contrast(fg, soft), `accent on accent-soft in ${theme}`).toBeGreaterThanOrEqual(4.5);
    });
  }
});

describe("theme choice", () => {
  const store = (value: string | null) => {
    const data = new Map<string, string>(value === null ? [] : [[THEME_STORAGE_KEY, value]]);
    return {
      getItem: (k: string) => data.get(k) ?? null,
      setItem: (k: string, v: string) => void data.set(k, v),
      data,
    };
  };

  it("follows the system only when asked to", () => {
    expect(resolveTheme("system", true)).toBe("dark");
    expect(resolveTheme("system", false)).toBe("light");
    expect(resolveTheme("light", true)).toBe("light");
    expect(resolveTheme("dark", false)).toBe("dark");
  });

  it("falls back to following the system when nothing usable is stored", () => {
    expect(readChoice(store(null))).toBe("system");
    expect(readChoice(store("sepia"))).toBe("system");
    expect(readChoice(store("light"))).toBe("light");
  });

  it("writes a choice that reads back", () => {
    const s = store(null);
    writeChoice(s, "dark");
    expect(readChoice(s)).toBe("dark");
    expect(s.data.get(THEME_STORAGE_KEY)).toBe("dark");
  });

  it("is repeated faithfully in the pre-paint script", () => {
    // index.html has to make the same decision before any module loads, so it
    // carries a second copy of the rule. This keeps the copies from drifting.
    expect(html).toContain(THEME_STORAGE_KEY);
    expect(html).toContain("dataset.theme");
    expect(html).toContain("prefers-color-scheme: dark");
  });
});
