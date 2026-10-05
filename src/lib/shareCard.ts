/**
 * The "I'm in" card, set as a pass.
 *
 * Restraint over decoration: a near-black field, one typeface, a strict grid,
 * and a single colour — the green of the status. The company is the hero,
 * set large and tight; beneath a tear line, the details sit in labelled
 * fields the way a boarding pass sets them. No gradient, no glow, no badge.
 *
 * It carries only what is the student's to share: the company, how many made
 * the list, the round, the date, and which of the season's shortlists this is
 * for them. No Neo ID, no CGPA, no one else's name.
 *
 * 1080 × 1350 — a portrait that sits well in a chat and in a story. Drawn
 * locally on a canvas; the preview is that same canvas.
 */

import type { DriveListItem, ImportOutcome } from "./api";

export const CARD = { width: 1080, height: 1350 } as const;

const SANS =
  'system-ui, -apple-system, "SF Pro Display", "Segoe UI Variable Display", "Segoe UI", sans-serif';

const INK = {
  field: "#0E0E10",
  text: "rgba(255,255,255,0.94)",
  quiet: "rgba(255,255,255,0.56)",
  label: "rgba(255,255,255,0.42)",
  rule: "rgba(255,255,255,0.12)",
  green: "#34C759",
};

const MARGIN = 96;

export interface CardFacts {
  company: string;
  /** How many were shortlisted. */
  total: number;
  /** This list's round or stage — "Interview", "R2" — when known. */
  round: string | null;
  /** Which of the season's shortlists you made this is — 1st, 2nd… */
  ordinal: number | null;
  /** When the list was imported. */
  when: Date | null;
}

/** Only facts that are the student's to share. */
export function cardFacts(
  outcome: ImportOutcome,
  drives: DriveListItem[],
): CardFacts {
  const yours = drives
    .filter((d) => d.verdict.status === "shortlisted")
    .sort((a, b) => a.imported_at.localeCompare(b.imported_at));
  const position = yours.findIndex((d) => d.id === outcome.drive_id);
  const record = drives.find((d) => d.id === outcome.drive_id);
  const when = record
    ? new Date(record.imported_at.replace(" ", "T") + "Z")
    : null;
  const step = outcome.progression?.steps.find(
    (s) => s.drive_id === outcome.drive_id,
  );
  return {
    company: outcome.company,
    total: outcome.total_students,
    round: step?.label ?? record?.round ?? null,
    ordinal: position >= 0 ? position + 1 : null,
    when: when && !Number.isNaN(when.getTime()) ? when : null,
  };
}

export function ordinalLabel(n: number): string {
  const tens = n % 100;
  const suffix =
    tens >= 11 && tens <= 13
      ? "th"
      : (({ 1: "st", 2: "nd", 3: "rd" } as Record<number, string>)[n % 10] ??
        "th");
  return `${n}${suffix}`;
}

/** The labelled fields under the tear line, in order, skipping the unknown. */
export function cardFields(facts: CardFacts): [string, string][] {
  const fields: [string, string][] = [];
  if (facts.round) fields.push(["Round", facts.round]);
  if (facts.when) {
    fields.push([
      "Date",
      facts.when.toLocaleDateString(undefined, {
        day: "numeric",
        month: "short",
        year: "numeric",
      }),
    ]);
  }
  if (facts.ordinal) {
    fields.push(["Season", `${ordinalLabel(facts.ordinal)} shortlist`]);
  }
  return fields;
}

/** Small capitals, tracked out — drawn letter by letter, since canvas
 *  letter-spacing is not everywhere yet. Returns the width drawn. */
function caps(
  ctx: CanvasRenderingContext2D,
  text: string,
  x: number,
  y: number,
  size: number,
  colour: string,
  align: "left" | "right" = "left",
): number {
  ctx.font = `620 ${size}px ${SANS}`;
  ctx.fillStyle = colour;
  const tracking = size * 0.16;
  const chars = [...text.toUpperCase()];
  const width =
    chars.reduce((w, c) => w + ctx.measureText(c).width + tracking, 0) -
    tracking;
  let at = align === "right" ? x - width : x;
  for (const c of chars) {
    ctx.fillText(c, at, y);
    at += ctx.measureText(c).width + tracking;
  }
  return width;
}

/** Breaks a name into at most two lines at `size`, or says it won't fit. */
function lines(
  ctx: CanvasRenderingContext2D,
  text: string,
  size: number,
  width: number,
): string[] | null {
  ctx.font = `640 ${size}px ${SANS}`;
  const words = text.split(/\s+/);
  const out: string[] = [];
  let line = "";
  for (const word of words) {
    const next = line ? `${line} ${word}` : word;
    if (ctx.measureText(next).width <= width) {
      line = next;
    } else {
      if (!line || out.length === 1) return null;
      out.push(line);
      line = word;
    }
  }
  out.push(line);
  return out.every((l) => ctx.measureText(l).width <= width) ? out : null;
}

export async function drawCard(
  canvas: HTMLCanvasElement,
  facts: CardFacts,
): Promise<void> {
  const { width: W, height: H } = CARD;
  canvas.width = W;
  canvas.height = H;
  const ctx = canvas.getContext("2d")!;
  const inner = W - MARGIN * 2;
  ctx.textBaseline = "alphabetic";

  ctx.fillStyle = INK.field;
  ctx.fillRect(0, 0, W, H);

  // Header: the name on the left, the status on the right — the only colour.
  ctx.font = `600 34px ${SANS}`;
  ctx.fillStyle = INK.text;
  ctx.fillText("nankiv", MARGIN, 130);
  const status = caps(
    ctx,
    "Shortlisted",
    W - MARGIN,
    128,
    23,
    INK.green,
    "right",
  );
  ctx.fillStyle = INK.green;
  ctx.beginPath();
  ctx.arc(W - MARGIN - status - 22, 120, 7, 0, Math.PI * 2);
  ctx.fill();

  ctx.fillStyle = INK.rule;
  ctx.fillRect(MARGIN, 178, inner, 2);

  // The company, as large as it will go in two lines.
  let size = 128;
  let set = lines(ctx, facts.company, size, inner);
  while (!set && size > 64) {
    size -= 4;
    set = lines(ctx, facts.company, size, inner);
  }
  if (!set) {
    // Even small, too long: one line, shortened.
    let t = facts.company;
    while (t.length > 1 && ctx.measureText(`${t}…`).width > inner) {
      t = t.slice(0, -1);
    }
    set = [`${t.trimEnd()}…`];
  }
  // The name and its one line, set at the optical centre of the space
  // between the header rule and the tear line — a touch above the true
  // middle, where a centred block looks centred.
  const tear = 1060;
  const lead = size * 1.04;
  const capHeight = size * 0.72;
  const below = 84; // the line under the name, from the name's last baseline
  const block = capHeight + lead * (set.length - 1) + below;
  const top = 180 + (tear - 180 - block) * 0.44;

  ctx.font = `640 ${size}px ${SANS}`;
  ctx.fillStyle = INK.text;
  let y = top + capHeight;
  for (const line of set) {
    ctx.fillText(line, MARGIN - size * 0.04, y);
    y += lead;
  }

  ctx.font = `400 38px ${SANS}`;
  ctx.fillStyle = INK.quiet;
  ctx.fillText(
    `One of ${facts.total.toLocaleString()} students on the list`,
    MARGIN,
    y - lead + below,
  );

  // The tear line, and the details below it.
  ctx.strokeStyle = INK.rule;
  ctx.lineWidth = 2;
  ctx.setLineDash([10, 10]);
  ctx.beginPath();
  ctx.moveTo(MARGIN, tear);
  ctx.lineTo(W - MARGIN, tear);
  ctx.stroke();
  ctx.setLineDash([]);

  const fields = cardFields(facts);
  const column = inner / 3;
  fields.forEach(([label, value], i) => {
    const x = MARGIN + column * i;
    caps(ctx, label, x, tear + 82, 21, INK.label);
    ctx.font = `560 40px ${SANS}`;
    ctx.fillStyle = INK.text;
    let v = value;
    while (v.length > 1 && ctx.measureText(v).width > column - 24) {
      v = v.slice(0, -1);
    }
    ctx.fillText(v === value ? v : `${v.trimEnd()}…`, x, tear + 140);
  });
}

/** The card as a PNG, from the canvas it was drawn on. */
export function cardPng(canvas: HTMLCanvasElement): Promise<Blob> {
  return new Promise((resolve, reject) =>
    canvas.toBlob(
      (b) => (b ? resolve(b) : reject(new Error("no image"))),
      "image/png",
    ),
  );
}
