/**
 * The icon system's invariants.
 *
 * On a Mac every icon is an SF Symbol drawn by macOS; everywhere else it is
 * nankiv's own drawing. Before the drawn set had rules it ran to seven stroke
 * weights across four viewBox grids, because each icon was drawn when it was
 * needed. Both sets are only sets if nothing can drift back out of them.
 */

import { render, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { api } from "../lib/api";
import { resetSymbols } from "../lib/symbols";
import { Icon, pointSizeFor, strokeFor, type IconName } from "./Icon";
import SYMBOLS from "./symbols.json";

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

describe("on a Mac, SF Symbols", () => {
  const image = {
    width: 21,
    height: 19,
    url: "data:image/png;base64,iVBORw0KGgo=",
  };

  beforeEach(() => {
    resetSymbols();
    Object.defineProperty(window, "__TAURI_INTERNALS__", {
      value: {},
      configurable: true,
    });
    vi.spyOn(navigator, "userAgent", "get").mockReturnValue(
      "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15",
    );
  });

  afterEach(() => {
    delete (window as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
    vi.restoreAllMocks();
  });

  it("asks macOS for every icon on screen in one call, by SF Symbols name", async () => {
    const symbols = vi
      .spyOn(api, "symbols")
      .mockImplementation(async (requests) => requests.map(() => image));
    render(
      <>
        <Icon name="gear" size={17} />
        <Icon name="people" size={17} />
        <Icon name="check" size={12} weight="semibold" />
      </>,
    );
    await waitFor(() =>
      expect(document.querySelectorAll(".sym-glyph")).toHaveLength(3),
    );
    expect(symbols).toHaveBeenCalledTimes(1);
    const requests = symbols.mock.calls[0]?.[0] ?? [];
    expect(requests.map((r) => r.name)).toEqual([
      "gearshape",
      "person.2",
      "checkmark",
    ]);
    expect(requests[2]).toMatchObject({
      pointSize: pointSizeFor(12),
      weight: "semibold",
    });
  });

  it("keeps the layout square, and centres the symbol in it", async () => {
    vi.spyOn(api, "symbols").mockResolvedValue([image]);
    const { container } = render(<Icon name="people" size={17} />);
    const box = container.querySelector<HTMLElement>(".sym")!;
    expect(box.style.width).toBe("17px");
    expect(box.style.height).toBe("17px");
    expect(box.getAttribute("aria-hidden")).toBe("true");

    const glyph = await waitFor(() => {
      const g = container.querySelector<HTMLElement>(".sym-glyph");
      expect(g).not.toBeNull();
      return g!;
    });
    expect(glyph.style.width).toBe("21px");
    expect(glyph.style.left).toBe("-2px");
    expect(glyph.style.top).toBe("-1px");
  });

  it("falls back to the drawn icon for a symbol this Mac does not have", async () => {
    vi.spyOn(api, "symbols").mockResolvedValue([null]);
    const { container } = render(<Icon name="sheet" size={21} />);
    await waitFor(() => expect(container.querySelector("svg")).not.toBeNull());
    expect(container.querySelector(".sym")).toBeNull();
  });

  it("falls back to the drawn set if the bridge fails", async () => {
    vi.spyOn(api, "symbols").mockRejectedValue(new Error("no bridge"));
    const { container } = render(<Icon name="trash" size={15} />);
    await waitFor(() => expect(container.querySelector("svg")).not.toBeNull());
  });

  it("asks once for an icon shown many times", async () => {
    const symbols = vi.spyOn(api, "symbols").mockResolvedValue([image]);
    const { rerender } = render(<Icon name="plus" size={14} />);
    await waitFor(() =>
      expect(document.querySelector(".sym-glyph")).not.toBeNull(),
    );
    rerender(
      <>
        <Icon name="plus" size={14} />
        <Icon name="plus" size={14} />
      </>,
    );
    expect(document.querySelectorAll(".sym-glyph")).toHaveLength(2);
    expect(symbols).toHaveBeenCalledTimes(1);
  });
});

describe("where SF Symbols are not licensed", () => {
  it("draws its own on Windows, without asking", () => {
    Object.defineProperty(window, "__TAURI_INTERNALS__", {
      value: {},
      configurable: true,
    });
    const ua = vi
      .spyOn(navigator, "userAgent", "get")
      .mockReturnValue("Mozilla/5.0 (Windows NT 10.0; Win64; x64) Edg/140.0");
    const symbols = vi.spyOn(api, "symbols");
    expect(svgOf("gear")).not.toBeNull();
    expect(symbols).not.toHaveBeenCalled();
    ua.mockRestore();
    delete (window as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
  });
});

describe("the symbol names", () => {
  it("names a symbol for every icon, as the SF Symbols app writes them", () => {
    for (const n of ALL) {
      expect(SYMBOLS[n], n).toMatch(/^[a-z0-9]+(\.[a-z0-9]+)*$/);
    }
    expect(Object.keys(SYMBOLS).sort()).toEqual([...ALL].sort());
  });

  it("keeps the largest symbols inside their square", () => {
    for (const size of [12, 14, 17, 21, 32, 38]) {
      expect(pointSizeFor(size)).toBeLessThanOrEqual(size * 0.8 + 0.25);
    }
  });
});
