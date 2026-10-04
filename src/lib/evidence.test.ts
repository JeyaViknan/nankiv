import { describe, expect, it } from "vitest";
import type { Evidence } from "./api";
import { PASTED_SHEET, explainVerdict } from "./evidence";

const base: Evidence = {
  key: "neo_id",
  yours: "V9H0G6C4",
  listed: 149,
  found_at: {
    kind: "neo_id",
    value: "V9H0G6C4",
    sheet: "Round 2",
    row: 42,
    column: "C",
    header: "Neo ID",
  },
};

describe("why it thinks you're in", () => {
  it("points at the row, sheet and column to check", () => {
    expect(explainVerdict({ status: "shortlisted" }, base)).toBe(
      "Row 42 of the sheet “Round 2” has your Neo ID, V9H0G6C4, in column C (“Neo ID”).",
    );
  });

  it("leaves out a heading the file never gave", () => {
    const e = { ...base, found_at: { ...base.found_at!, header: null } };
    expect(explainVerdict({ status: "shortlisted" }, e)).toBe(
      "Row 42 of the sheet “Round 2” has your Neo ID, V9H0G6C4, in column C.",
    );
  });

  it("speaks of lines for a pasted list", () => {
    const e = {
      ...base,
      found_at: {
        ...base.found_at!,
        sheet: PASTED_SHEET,
        row: 7,
        column: null,
      },
    };
    expect(explainVerdict({ status: "shortlisted" }, e)).toBe(
      "Line 7 of the list you pasted is your Neo ID, V9H0G6C4.",
    );
  });

  it("says what it can check when the row was never recorded", () => {
    const e = { ...base, found_at: null };
    expect(explainVerdict({ status: "shortlisted" }, e)).toBe(
      "Your Neo ID, V9H0G6C4, is one of the 149 Neo IDs in this file.",
    );
  });
});

describe("why it thinks you're not", () => {
  it("names what the file is keyed by, and how to fix a typo", () => {
    const e = {
      ...base,
      key: "reg_no" as const,
      yours: "23BAI0002",
      found_at: null,
    };
    expect(explainVerdict({ status: "not_shortlisted" }, e)).toBe(
      "This file lists students by registration number. Yours, 23BAI0002, isn't among its 149 registration numbers — if that's not your registration number, correct it in Settings.",
    );
  });
});

describe("when there is nothing to explain", () => {
  it("leaves 'can't tell' to the banner, which already says why", () => {
    expect(
      explainVerdict(
        { status: "undetermined", reason: "no_identity_configured" },
        { ...base, yours: null },
      ),
    ).toBeNull();
  });
});
