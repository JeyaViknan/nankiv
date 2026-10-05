/**
 * The "I'm in" card, made as a pass.
 *
 * A matte black card with a stub. The corners are rounded and two notches are
 * punched where the stub tears, cut out of the image itself, so wherever it is
 * posted it reads as an object rather than a screenshot. Three faces, each with
 * one job: a serif for the company, set large the way an invitation sets a
 * name; its italic for one line of season humour; a monospace for the small
 * print, the way a ticket prints it. One colour, the green of the status.
 *
 * It carries only what is the student's to share: the company, the round, the
 * day. No count, no Neo ID, no CGPA, no one else's name.
 *
 * The faces are the system's (New York and SF Mono on a Mac, Sitka and
 * Cascadia on Windows), the way the symbols are: nothing is bundled, nothing
 * is fetched. Drawn locally on a canvas; the preview is that same canvas.
 */

import type { DriveListItem, ImportOutcome } from "./api";

export const CARD = { width: 1080, height: 1350 } as const;

const SERIF =
  'ui-serif, "New York", "Sitka Display", "Iowan Old Style", Georgia, serif';
const SANS =
  'system-ui, -apple-system, "SF Pro Display", "Segoe UI Variable Display", "Segoe UI", sans-serif';
const MONO =
  'ui-monospace, "SF Mono", "Cascadia Mono", Menlo, Consolas, monospace';

const INK = {
  card: "#0B0B0D",
  text: "rgba(255,255,255,0.95)",
  line: "rgba(255,255,255,0.64)",
  label: "rgba(255,255,255,0.46)",
  perforation: "rgba(255,255,255,0.20)",
  edge: "rgba(255,255,255,0.10)",
  green: "#34C759",
};

const MARGIN = 96;
const RADIUS = 56;
/** Where the stub tears off, and the notches punched there. */
const TEAR = 1004;
const NOTCH = 30;

/**
 * One line of season humour, under the company. Short enough for one line at
 * any size the card sets it; never a number, never a name.
 */
export const LINES = [
  "Mom, I'm in the Excel.",
  "Ctrl+F found me.",
  "Somewhere in Sheet1, it's me.",
  "The spreadsheet said yes.",
  "Plot twist: it's me.",
  "Excel finally said something nice.",
  "My Neo ID did the talking.",
  "Shortlisted. Act normal.",
  "Main character of Sheet1.",
  "Next round, same energy.",
] as const;

export interface CardFacts {
  company: string;
  /** This list's round or stage — "Interview", "R2" — when known. */
  round: string | null;
  /** When the list was imported. */
  when: Date | null;
  /** Steady per drive: picks its first line and draws its barcode. */
  seed: number;
}

/** A small, steady hash: the same drive always gets the same card. */
function hash(text: string): number {
  let h = 2166136261;
  for (const c of text) {
    h ^= c.codePointAt(0)!;
    h = Math.imul(h, 16777619);
  }
  return h >>> 0;
}

/** Only facts that are the student's to share. */
export function cardFacts(
  outcome: ImportOutcome,
  drives: DriveListItem[],
): CardFacts {
  const record = drives.find((d) => d.id === outcome.drive_id);
  const when = record
    ? new Date(record.imported_at.replace(" ", "T") + "Z")
    : null;
  const step = outcome.progression?.steps.find(
    (s) => s.drive_id === outcome.drive_id,
  );
  return {
    company: outcome.company,
    round: step?.label ?? record?.round ?? null,
    when: when && !Number.isNaN(when.getTime()) ? when : null,
    seed: hash(`${outcome.drive_id}:${outcome.company}`),
  };
}

/** The line for a card, `turn` lines on from its own. */
export function lineFor(facts: CardFacts, turn = 0): string {
  return LINES[(facts.seed + turn) % LINES.length]!;
}

/** The labelled fields on the stub, in order, skipping the unknown. */
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
  return fields;
}

/** Tracked capitals, drawn letter by letter, since canvas letter-spacing is
 *  not everywhere yet. Returns the width drawn. */
function caps(
  ctx: CanvasRenderingContext2D,
  text: string,
  x: number,
  y: number,
  font: string,
  size: number,
  colour: string,
  align: "left" | "right" = "left",
): number {
  ctx.font = font;
  ctx.fillStyle = colour;
  const tracking = size * 0.2;
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

/** Tightens display type where the canvas can; elsewhere sets it as it is. */
function track(ctx: CanvasRenderingContext2D, px: number) {
  if ("letterSpacing" in ctx) {
    (
      ctx as CanvasRenderingContext2D & { letterSpacing: string }
    ).letterSpacing = `${px}px`;
  }
}

/** Breaks a name into at most two lines in `font`, or says it won't fit. */
function lines(
  ctx: CanvasRenderingContext2D,
  text: string,
  width: number,
): string[] | null {
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

/** The card's outline: rounded, with a notch punched in each side. */
function outline(ctx: CanvasRenderingContext2D) {
  const { width: W, height: H } = CARD;
  ctx.beginPath();
  ctx.moveTo(RADIUS, 0);
  ctx.lineTo(W - RADIUS, 0);
  ctx.arcTo(W, 0, W, RADIUS, RADIUS);
  ctx.lineTo(W, TEAR - NOTCH);
  ctx.arc(W, TEAR, NOTCH, -Math.PI / 2, Math.PI / 2, true);
  ctx.lineTo(W, H - RADIUS);
  ctx.arcTo(W, H, W - RADIUS, H, RADIUS);
  ctx.lineTo(RADIUS, H);
  ctx.arcTo(0, H, 0, H - RADIUS, RADIUS);
  ctx.lineTo(0, TEAR + NOTCH);
  ctx.arc(0, TEAR, NOTCH, Math.PI / 2, -Math.PI / 2, true);
  ctx.lineTo(0, RADIUS);
  ctx.arcTo(0, 0, RADIUS, 0, RADIUS);
  ctx.closePath();
}

/** A steady sequence from a seed, for the grain and the barcode. */
function random(seed: number): () => number {
  let s = seed || 1;
  return () => {
    s ^= s << 13;
    s ^= s >>> 17;
    s ^= s << 5;
    return (s >>> 0) / 4294967296;
  };
}

/** Matte stock: a fine, even grain, the same for the same card. */
function grain(ctx: CanvasRenderingContext2D, seed: number) {
  const tile = document.createElement("canvas");
  tile.width = tile.height = 192;
  const t = tile.getContext("2d");
  if (!t) return;
  const img = t.createImageData(192, 192);
  const next = random(seed);
  for (let i = 0; i < img.data.length; i += 4) {
    const v = next() < 0.5 ? 0 : 255;
    img.data[i] = img.data[i + 1] = img.data[i + 2] = v;
    img.data[i + 3] = Math.round(next() * 9);
  }
  t.putImageData(img, 0, 0);
  const pattern = ctx.createPattern(tile, "repeat");
  if (!pattern) return;
  ctx.fillStyle = pattern;
  ctx.fillRect(0, 0, CARD.width, CARD.height);
}

/** Decoration, not data: bars drawn from the seed, as a pass would carry. */
function barcode(
  ctx: CanvasRenderingContext2D,
  seed: number,
  right: number,
  top: number,
  width: number,
  height: number,
) {
  const next = random(seed ^ 0x9e3779b9);
  const bars: [number, number][] = [];
  let x = 0;
  while (x < width) {
    const bar = 3 + Math.floor(next() * 3) * 3;
    const gap = 4 + Math.floor(next() * 3) * 3;
    if (x + bar > width) break;
    bars.push([x, bar]);
    x += bar + gap;
  }
  const used = bars.length
    ? bars[bars.length - 1]![0] + bars[bars.length - 1]![1]
    : 0;
  const left = right - used;
  ctx.fillStyle = "rgba(255,255,255,0.88)";
  for (const [bx, bw] of bars) ctx.fillRect(left + bx, top, bw, height);
}

export async function drawCard(
  canvas: HTMLCanvasElement,
  facts: CardFacts,
  line: string = lineFor(facts),
): Promise<void> {
  const { width: W, height: H } = CARD;
  canvas.width = W;
  canvas.height = H;
  const ctx = canvas.getContext("2d")!;
  const inner = W - MARGIN * 2;
  ctx.clearRect(0, 0, W, H);
  ctx.textBaseline = "alphabetic";

  // The stock: black, lit faintly from above, with a grain, and an edge that
  // catches the light. Everything after is cut to the same outline.
  outline(ctx);
  ctx.save();
  ctx.clip();
  ctx.fillStyle = INK.card;
  ctx.fillRect(0, 0, W, H);
  const light = ctx.createLinearGradient(0, 0, 0, H * 0.6);
  light.addColorStop(0, "rgba(255,255,255,0.055)");
  light.addColorStop(1, "rgba(255,255,255,0)");
  ctx.fillStyle = light;
  ctx.fillRect(0, 0, W, H);
  grain(ctx, facts.seed);
  outline(ctx);
  ctx.strokeStyle = INK.edge;
  ctx.lineWidth = 3;
  ctx.stroke();

  // The masthead: the name, and the status in the one colour.
  ctx.font = `600 34px ${SANS}`;
  track(ctx, -0.3);
  ctx.fillStyle = INK.text;
  ctx.fillText("nankiv", MARGIN, 146);
  track(ctx, 0);
  const statusFont = `500 21px ${MONO}`;
  const status = caps(
    ctx,
    "Shortlisted",
    W - MARGIN,
    144,
    statusFont,
    21,
    INK.green,
    "right",
  );
  ctx.fillStyle = INK.green;
  ctx.beginPath();
  ctx.arc(W - MARGIN - status - 22, 137, 7, 0, Math.PI * 2);
  ctx.fill();

  // The company, as large as two lines allow, in the serif.
  let size = 172;
  const fit = () => {
    ctx.font = `600 ${size}px ${SERIF}`;
    track(ctx, -size * 0.016);
    return lines(ctx, facts.company, inner);
  };
  let set = fit();
  while (!set && size > 72) {
    size -= 4;
    set = fit();
  }
  if (!set) {
    // Even small, too long: one line, shortened.
    let t = facts.company;
    while (t.length > 1 && ctx.measureText(`${t}…`).width > inner) {
      t = t.slice(0, -1);
    }
    set = [`${t.trimEnd()}…`];
  }

  // The line under it, in the italic, one line however long the name.
  let lineSize = 54;
  const lineFont = () => `italic 400 ${lineSize}px ${SERIF}`;
  ctx.font = lineFont();
  track(ctx, 0);
  while (ctx.measureText(line).width > inner && lineSize > 30) {
    lineSize -= 2;
    ctx.font = lineFont();
  }

  // Name and line sit low, on the tear, the way a ticket sets its headline:
  // the black above them is the room the card is made of.
  const lead = size * 0.98;
  // Clear of a descender in the last line, whatever the size.
  const gap = Math.max(100, size * 0.58);
  const lineBaseline = TEAR - 118;
  const lastBaseline = lineBaseline - gap;

  ctx.font = `600 ${size}px ${SERIF}`;
  track(ctx, -size * 0.016);
  ctx.fillStyle = INK.text;
  set.forEach((l, i) => {
    const y = lastBaseline - lead * (set!.length - 1 - i);
    ctx.fillText(l, MARGIN - size * 0.03, y);
  });
  track(ctx, 0);
  ctx.font = lineFont();
  ctx.fillStyle = INK.line;
  ctx.fillText(line, MARGIN, lineBaseline);

  // The perforation, dot by dot between the notches.
  ctx.strokeStyle = INK.perforation;
  ctx.lineWidth = 4;
  ctx.lineCap = "round";
  ctx.setLineDash([0.1, 15]);
  ctx.beginPath();
  ctx.moveTo(NOTCH + 22, TEAR);
  ctx.lineTo(W - NOTCH - 22, TEAR);
  ctx.stroke();
  ctx.setLineDash([]);
  ctx.lineCap = "butt";

  // The stub: the small print in mono, its values in the sans, and a barcode.
  const stubMid = TEAR + (H - TEAR) / 2;
  const fields = cardFields(facts);
  const column = 300;
  fields.forEach(([label, value], i) => {
    const x = MARGIN + column * i;
    caps(ctx, label, x, stubMid - 28, `500 21px ${MONO}`, 21, INK.label);
    ctx.font = `600 40px ${SANS}`;
    track(ctx, -0.4);
    ctx.fillStyle = INK.text;
    let v = value;
    while (v.length > 1 && ctx.measureText(v).width > column - 30) {
      v = v.slice(0, -1);
    }
    ctx.fillText(v === value ? v : `${v.trimEnd()}…`, x, stubMid + 30);
    track(ctx, 0);
  });
  barcode(ctx, facts.seed, W - MARGIN, stubMid - 52, 236, 92);

  ctx.restore();
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
