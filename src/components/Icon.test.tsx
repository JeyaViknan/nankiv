/**
 * The icon system's invariant.
 *
 * Before this existed the app had seven stroke weights across four viewBox
 * grids, because each icon was drawn when it was needed. The set is only a set
 * if nothing can drift back out of it.
 */

import { render } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { Icon, strokeFor, type IconName } from "./Icon";

const ALL: IconName[] = [
  "back",
  "check",
  "chevronRight",
  "close",
  "compare",
  "dash",
  "gear",
  "people",
  "plus",
  "question",
  "search",
  "sheet",
  "trash",
  "tray",
  "warning",
];

function svgOf(name: IconName, size?: number) {
  const { container } = render(<Icon name={name} size={size} />);
  return container.querySelector("svg")!;
}

describe("one grid", () => {
  it("draws every icon on the same 20x20 viewBox", () => {
    for (const n of ALL) {
      expect(svgOf(n).getAttribute("viewBox")).toBe("0 0 20 20");
    }
  });

  it("uses round caps and joins throughout", () => {
    for (const n of ALL) {
      const svg = svgOf(n);
      expect(svg.getAttribute("stroke-linecap")).toBe("round");
      expect(svg.getAttribute("stroke-linejoin")).toBe("round");
    }
  });

  it("renders a path for every declared name", () => {
    for (const n of ALL) {
      const d = svgOf(n).querySelector("path")?.getAttribute("d") ?? "";
      expect(d.length, `${n} has no path data`).toBeGreaterThan(8);
    }
  });
});

describe("optical weight", () => {
  it("eases stroke back as the icon grows", () => {
    // A flat weight would make a 44px icon render nearly three times heavier
    // than a 16px one, which is exactly how a set stops looking like a set.
    expect(strokeFor(44)).toBeLessThan(strokeFor(16));
    expect(strokeFor(34)).toBeLessThan(strokeFor(20));
  });

  it("keeps the rendered weight within a narrow band across the range", () => {
    // Rendered px = strokeWidth × (size / 20). Across the sizes actually used,
    // that should stay visually even rather than tracking size linearly.
    const rendered = [14, 17, 20, 26, 34, 44].map(
      (s) => strokeFor(s) * (s / 20),
    );
    const ratio = Math.max(...rendered) / Math.min(...rendered);
    expect(ratio).toBeLessThan(2.2);
  });

  it("never goes below a hairline", () => {
    for (const s of [12, 14, 17, 20, 26, 34, 44, 64]) {
      expect(strokeFor(s)).toBeGreaterThan(0.8);
    }
  });
});

describe("accessibility", () => {
  it("hides icons from assistive technology, since labels live on controls", () => {
    for (const n of ALL) {
      const svg = svgOf(n);
      expect(svg.getAttribute("aria-hidden")).toBe("true");
      expect(svg.getAttribute("focusable")).toBe("false");
    }
  });

  it("inherits colour so semantic state comes from the parent", () => {
    for (const n of ALL) {
      expect(svgOf(n).getAttribute("stroke")).toBe("currentColor");
    }
  });
});
