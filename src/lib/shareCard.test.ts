import { describe, expect, it } from "vitest";
import type { DriveListItem, ImportOutcome, Verdict } from "./api";
import { cardFacts, ordinalLabel } from "./shareCard";

function drive(id: number, at: string, verdict: Verdict): DriveListItem {
  return {
    id,
    company: `Co ${id}`,
    drive_date: null,
    imported_at: at,
    source_filename: "f.xlsx",
    content_hash: `H${id}`,
    shape: "neo_id_only",
    primary_key: "neo_id",
    total_students: 10,
    round_label: null,
    parent_drive_id: null,
    verdict,
    round: null,
  };
}

const outcome = {
  drive_id: 3,
  company: "Siemens SISW",
  total_students: 149,
  you: {
    label: "You",
    neo_id: "V9H0G6C4",
    reg_no: "23BAI0002",
    verdict: { status: "shortlisted" },
    confidence: "verified",
  },
  your_cgpa: 8.42,
  progression: {
    steps: [
      { drive_id: 2, label: "Test", verdict: { status: "shortlisted" } },
      { drive_id: 3, label: "Interview", verdict: { status: "shortlisted" } },
    ],
    next_pending: true,
  },
} as unknown as ImportOutcome;

const drives = [
  drive(1, "2026-09-01 10:00:00", { status: "shortlisted" }),
  drive(4, "2026-09-02 10:00:00", { status: "not_shortlisted" }),
  drive(2, "2026-09-03 10:00:00", { status: "shortlisted" }),
  drive(3, "2026-10-05 10:00:00", { status: "shortlisted" }),
];

describe("what goes on the card", () => {
  it("is the company, the count, the round and the season", () => {
    const f = cardFacts(outcome, drives);
    expect(f.company).toBe("Siemens SISW");
    expect(f.total).toBe(149);
    expect(f.rounds).toEqual([
      { label: "Test", current: false },
      { label: "Interview", current: true },
    ]);
    expect(f.ordinal).toBe(3);
    expect(f.when?.toISOString()).toBe("2026-10-05T10:00:00.000Z");
  });

  it("carries nothing that is not the student's to share", () => {
    const text = JSON.stringify(cardFacts(outcome, drives));
    for (const leak of ["V9H0G6C4", "23BAI0002", "8.42"]) {
      expect(text).not.toContain(leak);
    }
  });
});

describe("ordinals", () => {
  it("are written as people say them", () => {
    expect([1, 2, 3, 4, 11, 12, 13, 21, 22, 103].map(ordinalLabel)).toEqual([
      "1st",
      "2nd",
      "3rd",
      "4th",
      "11th",
      "12th",
      "13th",
      "21st",
      "22nd",
      "103rd",
    ]);
  });
});
