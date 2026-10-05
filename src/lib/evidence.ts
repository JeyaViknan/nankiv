/**
 * "Why does it think I'm in?" — the evidence behind a verdict, in words.
 *
 * A wrong match is the worst thing this app can do, so the answer shows its
 * working: the row, the sheet and the column a student can open the file and
 * find for themselves. When it was not recorded — drives imported before it
 * was — it says what it can check instead, and never invents a location.
 */

import type { Evidence, KeyKind, Verdict } from "./api";

const KEY_NAME: Record<KeyKind, string> = {
  neo_id: "Neo ID",
  reg_no: "registration number",
};

/** Whether the file's own lines around yours can be shown. Not for drives
 *  imported before lines were kept. */
export function hasExcerpt(evidence: Evidence): boolean {
  return (
    evidence.key !== null &&
    evidence.found_at !== null &&
    evidence.excerpt.length > 0
  );
}

/** "Neo ID", "registration number": what a file lists students by. */
export function keyName(kind: KeyKind): string {
  return KEY_NAME[kind];
}

/** The sheet name a pasted list is recorded under. */
export const PASTED_SHEET = "Pasted list";

export function explainVerdict(
  verdict: Verdict,
  evidence: Evidence,
): string | null {
  if (!evidence.key || !evidence.yours) return null;
  const one = KEY_NAME[evidence.key];
  const count = evidence.listed.toLocaleString();

  if (verdict.status === "shortlisted") {
    const at = evidence.found_at;
    if (!at) {
      return `Your ${one}, ${evidence.yours}, is one of ${count} in this file.`;
    }
    if (at.sheet === PASTED_SHEET) {
      return `Your ${one}, ${evidence.yours}, is on line ${at.row} of what you pasted.`;
    }
    const column = at.column ? `, column ${at.column}` : "";
    return `Your ${one}, ${evidence.yours}, is in row ${at.row}${column} of “${at.sheet}”.`;
  }

  if (verdict.status === "not_shortlisted") {
    return `Your ${one}, ${evidence.yours}, isn't among the ${count} in this file. Wrong ${one}? Fix it in Settings.`;
  }

  // "Can't tell" already explains itself, with the fix beside it.
  return null;
}
