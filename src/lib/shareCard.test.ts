import { describe, expect, it } from "vitest";
import type { DriveListItem, ImportOutcome, Verdict } from "./api";
import { LINES, cardFacts, cardFields, lineFor } from "./shareCard";

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
  it("is the company, the round and the day", () => {
    const f = cardFacts(outcome, drives);
    expect(f.company).toBe("Siemens SISW");
    expect(f.round).toBe("Interview");
    expect(f.when?.toISOString()).toBe("2026-10-05T10:00:00.000Z");
  });

  it("carries nothing that is not the student's to share", () => {
    const text = JSON.stringify(cardFacts(outcome, drives));
    // Not who you are, not your CGPA, and not how many made the list.
    for (const leak of ["V9H0G6C4", "23BAI0002", "8.42", "149"]) {
      expect(text).not.toContain(leak);
    }
  });
});

describe("the line under the company", () => {
  it("is the same every time the same drive is shared", () => {
    const a = cardFacts(outcome, drives);
    const b = cardFacts(outcome, [...drives].reverse());
    expect(lineFor(a)).toBe(lineFor(b));
  });

  it("moves on to another line with each turn, and comes back round", () => {
    const f = cardFacts(outcome, drives);
    const seen = new Set(LINES.map((_, i) => lineFor(f, i)));
    expect(seen.size).toBe(LINES.length);
    expect(lineFor(f, LINES.length)).toBe(lineFor(f));
  });

  it("is short, and never a number", () => {
    for (const line of LINES) {
      expect(line.length).toBeLessThanOrEqual(36);
      // "Sheet1" is a name; a count on its own is not allowed.
      expect(line).not.toMatch(/\b\d+\b/);
    }
  });
});

describe("the fields on the stub", () => {
  it("are the round and the date, in that order", () => {
    const fields = cardFields(cardFacts(outcome, drives));
    expect(fields.map(([label]) => label)).toEqual(["Round", "Date"]);
  });

  it("leave out what isn't known, rather than show a blank", () => {
    expect(
      cardFields({ company: "Zluri", round: null, when: null, seed: 1 }),
    ).toEqual([]);
  });

  it("fall back to the stage the file named when there are no rounds", () => {
    const single = { ...outcome, progression: null } as ImportOutcome;
    const named = drives.map((d) => (d.id === 3 ? { ...d, round: "Test" } : d));
    expect(cardFacts(single, named).round).toBe("Test");
  });
});
