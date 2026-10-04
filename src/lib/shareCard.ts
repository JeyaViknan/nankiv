/**
 * The "I'm in" card.
 *
 * Drawn in nankiv's own visual language rather than a template's: the indigo
 * field, the frosted sheet of rows and the green "you" row from the app icon,
 * with the company written inside that row. It says what is worth saying —
 * the company, the round, how many made it, which of the season's shortlists
 * this is for you — and nothing that is not the student's to share: no Neo
 * ID, no CGPA, no one else's name.
 *
 * Drawn on a canvas, locally, at 1080 × 1350 — a portrait that sits well in a
 * chat and in a story.
 */

import type { DriveListItem, ImportOutcome } from "./api";
import { symbolImage } from "./symbols";

export const CARD = { width: 1080, height: 1350 } as const;

const SANS =
  'system-ui, -apple-system, "SF Pro Display", "Segoe UI Variable Display", "Segoe UI", sans-serif';

export interface CardFacts {
  company: string;
  /** How many were shortlisted. */
  total: number;
  /** The drive's rounds, earliest first, this one marked; empty for one. */
  rounds: { label: string; current: boolean }[];
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
  return {
    company: outcome.company,
    total: outcome.total_students,
    rounds:
      outcome.progression?.steps.map((s) => ({
        label: s.label,
        current: s.drive_id === outcome.drive_id,
      })) ?? [],
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

/** Fits text to a width, shrinking to a floor, then shortening with "…". */
function fit(
  ctx: CanvasRenderingContext2D,
  text: string,
  weight: number,
  from: number,
  to: number,
  width: number,
): { text: string; size: number } {
  for (let size = from; size >= to; size -= 2) {
    ctx.font = `${weight} ${size}px ${SANS}`;
    if (ctx.measureText(text).width <= width) return { text, size };
  }
  let t = text;
  while (t.length > 1 && ctx.measureText(`${t}…`).width > width) {
    t = t.slice(0, -1);
  }
  return { text: `${t.trimEnd()}…`, size: to };
}

function pill(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  w: number,
  h: number,
) {
  ctx.beginPath();
  ctx.roundRect(x, y, w, h, h / 2);
}

/** A symbol drawn in one colour, from the black-on-clear image macOS gives. */
function tinted(image: HTMLImageElement, colour: string): HTMLCanvasElement {
  const c = document.createElement("canvas");
  c.width = image.naturalWidth;
  c.height = image.naturalHeight;
  const g = c.getContext("2d")!;
  g.drawImage(image, 0, 0);
  g.globalCompositeOperation = "source-in";
  g.fillStyle = colour;
  g.fillRect(0, 0, c.width, c.height);
  return c;
}

function loadImage(url: string): Promise<HTMLImageElement> {
  return new Promise((resolve, reject) => {
    const img = new Image();
    img.onload = () => resolve(img);
    img.onerror = reject;
    img.src = url;
  });
}

export async function drawCard(
  canvas: HTMLCanvasElement,
  facts: CardFacts,
): Promise<void> {
  const { width: W, height: H } = CARD;
  canvas.width = W;
  canvas.height = H;
  const ctx = canvas.getContext("2d")!;

  // The field: the icon's indigo, lit from the top left.
  const field = ctx.createLinearGradient(0, 0, 0, H);
  field.addColorStop(0, "#5B52F2");
  field.addColorStop(1, "#2A2594");
  ctx.fillStyle = field;
  ctx.fillRect(0, 0, W, H);
  const glow = ctx.createRadialGradient(160, 110, 0, 160, 110, 980);
  glow.addColorStop(0, "rgba(255,255,255,0.18)");
  glow.addColorStop(1, "rgba(255,255,255,0)");
  ctx.fillStyle = glow;
  ctx.fillRect(0, 0, W, H);

  // The frosted sheet of rows, as on the icon, behind everything.
  ctx.fillStyle = "rgba(255,255,255,0.08)";
  ctx.strokeStyle = "rgba(255,255,255,0.16)";
  ctx.lineWidth = 2;
  ctx.beginPath();
  ctx.roundRect(660, 168, 380, 492, 56);
  ctx.fill();
  ctx.stroke();
  ctx.fillStyle = "rgba(255,255,255,0.13)";
  pill(ctx, 716, 262, 268, 44);
  ctx.fill();
  pill(ctx, 716, 484, 212, 44);
  ctx.fill();

  // The wordmark, and the news.
  ctx.textBaseline = "alphabetic";
  ctx.fillStyle = "rgba(255,255,255,0.9)";
  ctx.font = `650 44px ${SANS}`;
  ctx.fillText("nankiv", 96, 140);
  ctx.fillStyle = "#FFFFFF";
  ctx.font = `800 172px ${SANS}`;
  ctx.fillText("I\u2019m in.", 90, 470);

  // The green row — you, on the list — with the company in it.
  const row = { x: 96, y: 540, w: 888, h: 150 };
  ctx.save();
  ctx.shadowColor = "rgba(10, 8, 60, 0.35)";
  ctx.shadowBlur = 48;
  ctx.shadowOffsetY = 18;
  ctx.fillStyle = "#30D158";
  pill(ctx, row.x, row.y, row.w, row.h);
  ctx.fill();
  ctx.restore();

  const mid = row.y + row.h / 2;
  const symbol = await symbolImage({
    name: "checkmark",
    pointSize: 58,
    weight: "bold",
  });
  if (symbol) {
    const img = await loadImage(symbol.url);
    ctx.drawImage(
      tinted(img, "#FFFFFF"),
      row.x + 70 - symbol.width / 2,
      mid - symbol.height / 2,
      symbol.width,
      symbol.height,
    );
  } else {
    ctx.strokeStyle = "#FFFFFF";
    ctx.lineWidth = 15;
    ctx.lineCap = "round";
    ctx.lineJoin = "round";
    ctx.beginPath();
    ctx.moveTo(row.x + 44, mid + 2);
    ctx.lineTo(row.x + 64, mid + 22);
    ctx.lineTo(row.x + 100, mid - 22);
    ctx.stroke();
  }
  const name = fit(ctx, facts.company, 760, 72, 40, row.w - 150 - 56);
  ctx.fillStyle = "#FFFFFF";
  ctx.font = `760 ${name.size}px ${SANS}`;
  ctx.textBaseline = "middle";
  ctx.fillText(name.text, row.x + 140, mid + 3);
  ctx.textBaseline = "alphabetic";

  // The rounds, when there are several: where this one sits.
  let y = 800;
  if (facts.rounds.length > 1) {
    let x = 96;
    ctx.font = `650 34px ${SANS}`;
    facts.rounds.forEach((r, i) => {
      if (i > 0) {
        ctx.fillStyle = "rgba(255,255,255,0.6)";
        ctx.fillText("→", x, y + 42);
        x += 52;
      }
      const w = ctx.measureText(r.label).width + 52;
      pill(ctx, x, y, w, 60);
      if (r.current) {
        ctx.fillStyle = "#FFFFFF";
        ctx.fill();
        ctx.fillStyle = "#2E2A9E";
      } else {
        ctx.strokeStyle = "rgba(255,255,255,0.55)";
        ctx.lineWidth = 2;
        ctx.stroke();
        ctx.fillStyle = "rgba(255,255,255,0.88)";
      }
      ctx.fillText(r.label, x + 26, y + 42);
      x += w + 18;
    });
    y += 132;
  } else {
    y += 40;
  }

  // The facts.
  ctx.fillStyle = "rgba(255,255,255,0.95)";
  ctx.font = `600 48px ${SANS}`;
  ctx.fillText(`One of ${facts.total.toLocaleString()} shortlisted`, 96, y);
  if (facts.when) {
    ctx.fillStyle = "rgba(255,255,255,0.62)";
    ctx.font = `500 38px ${SANS}`;
    ctx.fillText(
      facts.when.toLocaleDateString(undefined, {
        day: "numeric",
        month: "long",
        year: "numeric",
      }),
      96,
      y + 64,
    );
  }

  // Which shortlist of the season, once there is more than one to count.
  if (facts.ordinal && facts.ordinal > 1) {
    const text = `${ordinalLabel(facts.ordinal)} shortlist this season`;
    ctx.font = `600 34px ${SANS}`;
    const w = ctx.measureText(text).width + 56;
    pill(ctx, 96, y + 120, w, 64);
    ctx.strokeStyle = "rgba(255,255,255,0.45)";
    ctx.lineWidth = 2;
    ctx.stroke();
    ctx.fillStyle = "rgba(255,255,255,0.9)";
    ctx.fillText(text, 124, y + 164);
  }

  ctx.fillStyle = "rgba(255,255,255,0.5)";
  ctx.font = `500 30px ${SANS}`;
  ctx.fillText("Checked offline with nankiv", 96, 1262);
}

/** The card as a PNG. */
export async function renderCard(facts: CardFacts): Promise<Blob> {
  const canvas = document.createElement("canvas");
  await drawCard(canvas, facts);
  return new Promise((resolve, reject) =>
    canvas.toBlob(
      (b) => (b ? resolve(b) : reject(new Error("no image"))),
      "image/png",
    ),
  );
}
