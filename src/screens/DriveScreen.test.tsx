/**
 * The order of the result page.
 *
 * Your answer, then your circle, then everything else — whatever the answer
 * was. An earlier build moved the analysis above the circle when the answer
 * was no, so the page changed shape depending on the result, and the people
 * you came to check on were pushed down exactly when you most wanted them.
 */

import { render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it } from "vitest";
import type { ImportOutcome, PersonResult, Verdict } from "../lib/api";
import { useStore } from "../lib/store";
import { DriveScreen } from "./DriveScreen";

function person(label: string, verdict: Verdict): PersonResult {
  return {
    label,
    neo_id: null,
    reg_no: null,
    verdict,
    confidence: "verified",
    cgpa: null,
  };
}

function outcome(you: Verdict): ImportOutcome {
  return {
    drive_id: 1,
    company: "Aurora Systems",
    drive_date: null,
    total_students: 96,
    shape: "neo_id_only",
    primary_key: "neo_id",
    you: person("You", you),
    friends: [
      person("Arjun", { status: "shortlisted" }),
      person("Meera", { status: "not_shortlisted" }),
    ],
    analysis: {
      total_students: 96,
      matched_students: 0,
      coverage: 0,
      sufficient: false,
      cgpa: null,
      cutoff: null,
      branches: null,
      your_percentile: null,
    },
    learned_verified: 0,
    learned_named: 0,
    unreadable_headers: null,
  };
}

/** The section headings, top to bottom. */
function sections(): string[] {
  return [...document.querySelectorAll(".block .eyebrow")].map(
    (e) => e.textContent ?? "",
  );
}

beforeEach(() => {
  useStore.setState({ drives: [], profile: null, stats: null });
});

describe("the result page", () => {
  for (const [answer, verdict] of [
    ["in", { status: "shortlisted" }],
    ["not in", { status: "not_shortlisted" }],
  ] as const) {
    it(`shows your circle straight after the answer when you're ${answer}`, () => {
      render(<DriveScreen outcome={outcome(verdict)} onFix={() => {}} />);
      expect(sections()).toEqual([
        "Your circle",
        "What this shortlist suggests",
      ]);
      expect(screen.getByText("Arjun")).toBeInTheDocument();
      expect(screen.getByText("Meera")).toBeInTheDocument();
    });
  }
});
