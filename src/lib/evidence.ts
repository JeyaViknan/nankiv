/**
 * "Why does it think I'm in?" — the evidence behind a verdict, in words.
 *
 * A wrong match is the worst thing this app can do, so the answer shows its
 * working: the row, the sheet and the column a student can open the file and
 * find for themselves. When it was not recorded — drives imported before it
 * was — it says what it can check instead, and never invents a location.
 */

import type { Evidence, KeyKind, Verdict } from "./api";

const KEY_NAME: Record<KeyKind, [string, string]> = {
  neo_id: ["Neo ID", "Neo IDs"],
  reg_no: ["registration number", "registration numbers"],
};

/** The sheet name a pasted list is recorded under. */
export const PASTED_SHEET = "Pasted list";

export function explainVerdict(
  verdict: Verdict,
  evidence: Evidence,
): string | null {
  if (!evidence.key || !evidence.yours) return null;
  const [one, many] = KEY_NAME[evidence.key];
  const count = `${evidence.listed.toLocaleString()} ${evidence.listed === 1 ? one : many}`;

  if (verdict.status === "shortlisted") {
    const at = evidence.found_at;
    if (!at) {
      return `Your ${one}, ${evidence.yours}, is one of the ${count} in this file.`;
    }
    if (at.sheet === PASTED_SHEET) {
      return `Line ${at.row} of the list you pasted is your ${one}, ${evidence.yours}.`;
    }
    const column = at.column
      ? `, in column ${at.column}${at.header ? ` (“${at.header}”)` : ""}`
      : "";
    return `Row ${at.row} of the sheet “${at.sheet}” has your ${one}, ${evidence.yours}${column}.`;
  }

  if (verdict.status === "not_shortlisted") {
    return `This file lists students by ${one}. Yours, ${evidence.yours}, isn't among its ${count} — if that's not your ${one}, correct it in Settings.`;
  }

  // "Can't tell" already explains itself, with the fix beside it.
  return null;
}
