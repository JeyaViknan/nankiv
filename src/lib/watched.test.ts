import { describe, expect, it } from "vitest";
import { arrivalMessage } from "./watched";

describe("what arrived from Downloads", () => {
  it("says the company and the answer, kindly", () => {
    expect(arrivalMessage("Siemens", { status: "shortlisted" })).toBe(
      "Siemens came in from Downloads — you're in",
    );
    expect(arrivalMessage("Zluri", { status: "not_shortlisted" })).toBe(
      "Zluri came in from Downloads — not this time",
    );
    expect(
      arrivalMessage("Deloitte", {
        status: "undetermined",
        reason: "no_identity_configured",
      }),
    ).toBe("Deloitte came in from Downloads");
  });
});
