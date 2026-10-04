import { describe, expect, it } from "vitest";
import { cgpaUpdatedLabel, readCgpa } from "./cgpa";

describe("reading a typed CGPA", () => {
  it("takes a figure out of ten", () => {
    expect(readCgpa("8.42")).toEqual({ kind: "valid", value: 8.42 });
    expect(readCgpa(" 9 ")).toEqual({ kind: "valid", value: 9 });
    expect(readCgpa("10")).toEqual({ kind: "valid", value: 10 });
  });

  it("treats a blank field as no CGPA, not as zero", () => {
    expect(readCgpa("")).toEqual({ kind: "empty" });
    expect(readCgpa("   ")).toEqual({ kind: "empty" });
  });

  it("refuses a percentage rather than guessing at it", () => {
    expect(readCgpa("84.2")).toEqual({ kind: "invalid" });
    expect(readCgpa("84")).toEqual({ kind: "invalid" });
  });

  it("refuses anything that is not a plain number", () => {
    for (const t of ["0", "10.5", "8.421", "8,42", "eight", "-8", ".5"]) {
      expect(readCgpa(t), t).toEqual({ kind: "invalid" });
    }
  });
});

describe("saying how current it is", () => {
  it("names the day it changed, in the reader's own date order", () => {
    expect(cgpaUpdatedLabel("2026-10-04 12:00:00")).toMatch(
      /^Updated (4 Oct|Oct 4)/,
    );
  });

  it("says nothing when there is nothing to date", () => {
    expect(cgpaUpdatedLabel(null)).toBeNull();
    expect(cgpaUpdatedLabel("not a date")).toBeNull();
  });
});
