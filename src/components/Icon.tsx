/**
 * The icon set.
 *
 * Every icon in the application was previously drawn ad hoc, which produced
 * seven different stroke weights across four different viewBox grids. That
 * incoherence is what reads as unprofessional — far more than the absence of
 * any particular icon library.
 *
 * So: one system, applied without exception.
 *
 *   grid      20 × 20, matching the SF Symbols small optical size
 *   stroke    1.5 at 20px, scaled proportionally by `size`
 *   caps      round, joins round
 *   alignment shapes sit on the half-pixel grid so 1.5px strokes stay crisp
 *
 * The geometry follows the conventions SF Symbols uses — consistent optical
 * weight, generous interior counters, terminals that stop short of the bounding
 * box — but every path here is original. Apple's own artwork is licensed for
 * Apple platforms only and may not be embedded in a distributed product, and
 * nankiv ships for Windows too.
 */

import type { SVGProps } from "react";

export type IconName =
  | "back"
  | "check"
  | "chevronRight"
  | "close"
  | "compare"
  | "dash"
  | "gear"
  | "people"
  | "plus"
  | "question"
  | "search"
  | "sheet"
  | "trash"
  | "tray"
  | "warning";

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

interface Props extends Omit<SVGProps<SVGSVGElement>, "name"> {
  name: IconName;
  /** Rendered size in px. Stroke weight scales with it to hold optical weight. */
  size?: number;
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

export function Icon({ name, size = 17, ...rest }: Props) {
  const circle = CIRCLES[name];
  const dot = DOTS[name];

  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 20 20"
      fill="none"
      stroke="currentColor"
      strokeWidth={strokeFor(size)}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      focusable="false"
      {...rest}
    >
      {circle && <circle cx={circle[0]} cy={circle[1]} r={circle[2]} />}
      <path d={PATHS[name]} />
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
