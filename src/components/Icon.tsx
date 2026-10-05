/**
 * The icon set.
 *
 * On a Mac every icon is an SF Symbol — named in symbols.json exactly as
 * Apple's SF Symbols app lists it, and drawn by macOS at the size and weight it
 * is shown at. nankiv ships none of Apple's artwork: the symbols are licensed
 * for Apple platforms only, the same code builds for Windows, and so they come
 * from the system at run time the way a native app's do. See src/lib/symbols.ts.
 *
 * Everywhere else — Windows, macOS before 11, the browser preview — the icons
 * are nankiv's own drawings, held to one system so the fallback is still a set:
 *
 *   grid      20 × 20, matching the SF Symbols small optical size
 *   stroke    1.5 at 20px, scaled proportionally by `size`
 *   caps      round, joins round
 *   alignment shapes sit on the half-pixel grid so 1.5px strokes stay crisp
 *
 * Every path here is original.
 */

import type { CSSProperties } from "react";
import type { SymbolImage, SymbolWeight } from "../lib/api";
import { symbolScale, useSymbol } from "../lib/symbols";
import SYMBOLS from "./symbols.json";

export type IconName = keyof typeof SYMBOLS;

/**
 * Path data on a 20×20 grid.
 *
 * Coordinates land on halves so that a 1.5-wide stroke straddles a pixel
 * boundary cleanly at 1× rather than blurring across two.
 */
const PATHS: Record<IconName, string> = {
  back: "M12.25 4.5 6.75 10l5.5 5.5",
  check: "M4.75 10.5 8.5 14.25l6.75-8",
  chevronRight: "M7.75 4.5 13.25 10l-5.5 5.5",
  close: "M5.25 5.25l9.5 9.5M14.75 5.25l-9.5 9.5",
  compare: "M7.5 4.5h-3v11h3M12.5 4.5h3v11h-3M10 3v14",
  // Two sheets, the front one whole and the one behind showing its corner.
  copy:
    "M6.25 6.75h5.5a1.5 1.5 0 0 1 1.5 1.5v6.5a1.5 1.5 0 0 1-1.5 1.5h-5.5a1.5 1.5 0 0 1-1.5-1.5v-6.5a1.5 1.5 0 0 1 1.5-1.5Z" +
    "M7.75 4.25h6a1.5 1.5 0 0 1 1.5 1.5v7",
  dash: "M5.5 10h9",
  // Eight teeth around a hub, rather than a toothed outline: a drawn gear
  // silhouette turns to mush below about 24px. The teeth start *inside* the hub
  // radius so they read as attached — spokes with a gap around the hub read as
  // a sun, which is the difference between "settings" and "brightness".
  gear:
    "M12.9 10 16.4 10M12.05 12.05 14.53 14.53M10 12.9 10 16.4" +
    "M7.95 12.05 5.47 14.53M7.1 10 3.6 10M7.95 7.95 5.47 5.47" +
    "M10 7.1 10 3.6M12.05 7.95 14.53 5.47",
  people:
    "M7.75 9.5a2.5 2.5 0 1 0 0-5 2.5 2.5 0 0 0 0 5ZM2.75 16.25c0-2.55 2.24-4.25 5-4.25s5 1.7 5 4.25" +
    "M13.5 5.4a2.05 2.05 0 0 1 0 4.1M14.4 12.15c2.2.35 2.85 1.85 2.85 4.1",
  plus: "M10 4.75v10.5M4.75 10h10.5",
  question: "M7.6 7.6a2.45 2.45 0 1 1 3.65 2.13c-.72.42-1.25.98-1.25 1.82v.3",
  // An arrow into an open box.
  save:
    "M10 3.25v8.5M6.75 8.5 10 11.75l3.25-3.25" +
    "M6.75 7.25h-1.5a1.5 1.5 0 0 0-1.5 1.5v6a1.5 1.5 0 0 0 1.5 1.5h9.5a1.5 1.5 0 0 0 1.5-1.5v-6a1.5 1.5 0 0 0-1.5-1.5h-1.5",
  search: "M9.25 15.25a6 6 0 1 0 0-12 6 6 0 0 0 0 12ZM13.6 13.6 17 17",
  sheet:
    "M4.25 3.75h11.5a.5.5 0 0 1 .5.5v11.5a.5.5 0 0 1-.5.5H4.25a.5.5 0 0 1-.5-.5V4.25a.5.5 0 0 1 .5-.5Z" +
    "M3.75 8.25h12.5M8 8.25v8M3.75 12.25h12.5",
  trash:
    "M3.75 5.5h12.5M7.75 5.5V3.75h4.5V5.5M5.5 5.5l.6 10.25a.75.75 0 0 0 .75.7h6.3a.75.75 0 0 0 .75-.7L14.5 5.5",
  tray: "M10 3.25v9.5M6.5 9.25 10 12.75l3.5-3.5M3.75 13.5v2.25a.75.75 0 0 0 .75.75h11a.75.75 0 0 0 .75-.75V13.5",
  warning: "M10 6.25v4.5",
};

/** Icons whose meaning needs a filled dot the stroke cannot express. */
const DOTS: Partial<Record<IconName, [number, number]>> = {
  question: [10, 14.6],
  warning: [10, 14.1],
};

/** Icons that carry a stroked circle alongside their path. */
const CIRCLES: Partial<Record<IconName, [number, number, number]>> = {
  question: [10, 10, 7.25],
  warning: [10, 10, 7.25],
  gear: [10, 10, 3.15],
};

/** How much heavier than regular each weight draws the fallback stroke. */
const STROKE_WEIGHT: Record<SymbolWeight, number> = {
  ultralight: 0.5,
  thin: 0.65,
  light: 0.8,
  regular: 1,
  medium: 1.2,
  semibold: 1.6,
  bold: 1.8,
  heavy: 2,
  black: 2.2,
};

interface Props {
  name: IconName;
  /** The square the icon occupies in the layout, in px. */
  size?: number;
  /** As in San Francisco: the symbol's weight should match its label's. */
  weight?: SymbolWeight;
  /** Draw the icon in on first paint. For arrivals only. */
  draw?: boolean;
  className?: string;
}

/**
 * The SF Symbols point size for an icon occupying a `size` square.
 *
 * A symbol's point size is a font size, not a box: its drawing runs taller
 * than the size and, for wide symbols, wider. Four-fifths keeps the set's
 * largest symbols inside their square at every size nankiv uses.
 */
export function pointSizeFor(size: number): number {
  return Math.round(size * 0.8 * 2) / 2;
}

/**
 * Stroke weight in viewBox units for a given rendered size.
 *
 * Stroke scales with the icon, so a 34px icon drawn at a flat 1.5 renders twice
 * as heavy as a 17px one and the set stops looking like a set. Easing the value
 * back as size grows holds the *optical* weight even, which is the entire point
 * of having a system rather than a folder of drawings.
 */
export function strokeFor(size: number): number {
  const w = 1.5 * Math.pow(20 / Math.max(size, 14), 0.45);
  return Number(w.toFixed(3));
}

export function Icon({
  name,
  size = 17,
  weight = "regular",
  draw = false,
  className,
}: Props) {
  const symbol = useSymbol({
    name: SYMBOLS[name],
    pointSize: pointSizeFor(size),
    weight,
  });

  if (symbol === null) {
    return (
      <Drawn
        name={name}
        size={size}
        weight={weight}
        draw={draw}
        className={className}
      />
    );
  }

  // The square holds the layout; the symbol sits centred in it at its own
  // size, and is only there once macOS has drawn it — a frame at most.
  const classes = ["sym", draw && "sym-draw", className].filter(Boolean);
  return (
    <span
      className={classes.join(" ")}
      style={{ width: size, height: size }}
      aria-hidden="true"
    >
      {symbol && <span className="sym-glyph" style={glyph(symbol, size)} />}
    </span>
  );
}

/**
 * Centres the symbol in its square, snapped to device pixels so the drawing
 * macOS made for this density is not resampled across two.
 */
function glyph(symbol: SymbolImage, size: number): CSSProperties {
  const scale = symbolScale();
  const snap = (v: number) => Math.round(v * scale) / scale;
  const mask = `url("${symbol.url}")`;
  return {
    left: snap((size - symbol.width) / 2),
    top: snap((size - symbol.height) / 2),
    width: symbol.width,
    height: symbol.height,
    WebkitMaskImage: mask,
    maskImage: mask,
  };
}

/** nankiv's own drawing, where SF Symbols are not available. */
function Drawn({
  name,
  size,
  weight,
  draw,
  className,
}: Required<Omit<Props, "className">> & { className?: string }) {
  const circle = CIRCLES[name];
  const dot = DOTS[name];

  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 20 20"
      fill="none"
      stroke="currentColor"
      strokeWidth={strokeFor(size) * STROKE_WEIGHT[weight]}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      focusable="false"
      className={className}
    >
      {circle && <circle cx={circle[0]} cy={circle[1]} r={circle[2]} />}
      {/* `draw` lets the stroke draw itself once, for the single moment in the
          application that earns it: a result arriving. The CSS owns the timing
          and stands the icon down for anyone who has asked for less motion. */}
      <path d={PATHS[name]} className={draw ? "icon-draw" : undefined} />
      {dot && (
        <circle
          cx={dot[0]}
          cy={dot[1]}
          r={0.95}
          fill="currentColor"
          stroke="none"
        />
      )}
    </svg>
  );
}
