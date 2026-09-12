/**
 * Guards on the stylesheet itself.
 *
 * These exist because each one describes a bug that actually shipped. CSS has
 * no type checker, so the failures are silent: the page still renders, it just
 * renders wrong.
 */

import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";

const css = readFileSync(join(__dirname, "app.css"), "utf8");

/** Rough CSS specificity for the simple selectors used in this file. */
function specificity(selector: string): [number, number, number] {
  // `:where(...)` contributes nothing, by definition — strip it whole before
  // counting, or its contents get tallied and the result is meaningless.
  const sel = selector.replace(/:where\([^)]*\)/g, "");
  const ids = (sel.match(/#[\w-]+/g) ?? []).length;
  const classes = (sel.match(/\.[\w-]+/g) ?? []).length;
  const attrs = (sel.match(/\[[^\]]+\]/g) ?? []).length;
  const pseudoClasses = (sel.match(/:(?!:)[\w-]+/g) ?? []).length;
  const elements = (sel.match(/(^|[\s>+~])[a-z][\w-]*/g) ?? []).length;
  const pseudoElements = (sel.match(/::[\w-]+/g) ?? []).length;
  return [ids, classes + attrs + pseudoClasses, elements + pseudoElements];
}

function beats(a: string, b: string): boolean {
  const [x, y] = [specificity(a), specificity(b)];
  for (let i = 0; i < 3; i++) {
    if (x[i]! !== y[i]!) return x[i]! > y[i]!;
  }
  return false;
}

describe("cascade", () => {
  it("gives the base input rule zero specificity", () => {
    // The bug: `.search-input` (0,1,0) was declared before `input[type="text"]`
    // (0,1,1), so the generic padding won and slid the placeholder underneath
    // the search glyph. Wrapping the base rule in :where() drops it to (0,0,0)
    // so a component class always wins, whatever the declaration order.
    expect(css).toContain(':where(input[type="text"]) {');
    expect(css).not.toMatch(/\ninput\[type="text"\] \{/);
    expect(specificity(':where(input[type="text"])')).toEqual([0, 0, 0]);
  });

  it("lets every component input class out-rank the base rule", () => {
    // Catch the whole class of bug rather than the instances of it. `.name-input`
    // was a latent second case: it worked only because that element omitted a
    // type attribute, so adding one would have broken it silently.
    const classes = [...css.matchAll(/\n(\.[\w-]*input[\w-]*)[\s:{]/g)].map(
      (m) => m[1]!,
    );
    expect(classes.length).toBeGreaterThan(0);
    for (const sel of classes) {
      expect(beats(sel, ':where(input[type="text"])')).toBe(true);
    }
  });
});

describe("theming", () => {
  it("defines every colour token in the bare :root block", () => {
    // A token defined only inside a media or [data-theme] block is undefined in
    // the un-stamped "system" state, which is what most viewers actually get.
    const root = css.slice(css.indexOf(":root {"), css.indexOf("@media"));
    const used = new Set(
      [...css.matchAll(/var\((--[\w-]+)\)/g)].map((m) => m[1]!),
    );
    const missing = [...used].filter((t) => !root.includes(`${t}:`));
    expect(missing).toEqual([]);
  });

  it("redefines the dark palette for both the media query and the toggle", () => {
    // Three states, not two: an explicit choice must beat the OS in either
    // direction, and "system" must fall through to the media query.
    expect(css).toContain("@media (prefers-color-scheme: dark)");
    expect(css).toContain(':root:not([data-theme="light"])');
    expect(css).toContain(':root[data-theme="dark"]');
  });

  it("paints the body from a token rather than inheriting the host ground", () => {
    const body = css.slice(css.indexOf("\nbody {"));
    expect(body.slice(0, 400)).toMatch(/background: var\(--bg\)/);
  });
});

describe("verdict states", () => {
  it("gives each of the three states its own rule", () => {
    for (const sel of [".verdict.yes", ".verdict.no", ".verdict.unknown"]) {
      expect(css).toContain(sel);
    }
  });

  it("does not put white text on system green", () => {
    // White on any green bright enough to read as positive fails contrast. The
    // colour belongs in the glyph; the text stays at full strength on the
    // normal surface.
    const yes = css.slice(
      css.indexOf(".verdict.yes"),
      css.indexOf(".verdict.no"),
    );
    expect(yes).not.toMatch(/color:\s*#fff/i);
  });
});
