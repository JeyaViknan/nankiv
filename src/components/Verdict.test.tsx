/**
 * The reliability hierarchy, enforced at the rendering layer.
 *
 * The Rust side guarantees the *data* can't conflate "unknown" with "no". These
 * tests guarantee the *interface* doesn't undo that by rendering them alike.
 */

import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { VerdictBanner, VerdictPill, undeterminedText } from "./Verdict";
import {
  isDefiniteNo,
  isShortlisted,
  isUndetermined,
  type Verdict,
} from "../lib/api";

const IN: Verdict = { status: "shortlisted" };
const OUT: Verdict = { status: "not_shortlisted" };
const NO_IDENTITY: Verdict = {
  status: "undetermined",
  reason: "no_identity_configured",
};
const WRONG_KEY: Verdict = {
  status: "undetermined",
  reason: "key_kind_not_configured",
  file_key: "reg_no",
};
const UNREADABLE: Verdict = {
  status: "undetermined",
  reason: "file_not_understood",
};

describe("verdict predicates", () => {
  it("separates a definite no from an unknown", () => {
    expect(isDefiniteNo(OUT)).toBe(true);
    for (const v of [NO_IDENTITY, WRONG_KEY, UNREADABLE]) {
      expect(isDefiniteNo(v)).toBe(false);
      expect(isUndetermined(v)).toBe(true);
      expect(isShortlisted(v)).toBe(false);
    }
  });

  it("does not treat the absence of a yes as a no", () => {
    // The bug this whole application exists to avoid, expressed directly.
    const notYes = [OUT, NO_IDENTITY, WRONG_KEY, UNREADABLE].filter(
      (v) => !isShortlisted(v),
    );
    expect(notYes).toHaveLength(4);
    expect(notYes.filter(isDefiniteNo)).toHaveLength(1);
  });
});

describe("VerdictBanner", () => {
  it("announces a positive result unmistakably", () => {
    render(
      <VerdictBanner verdict={IN} company="Siemens" totalStudents={166} />,
    );
    expect(screen.getByText("You're in")).toBeInTheDocument();
    expect(screen.getByText(/Siemens/)).toBeInTheDocument();
    expect(screen.getByText(/166/)).toBeInTheDocument();
  });

  it("states a genuine rejection plainly", () => {
    render(
      <VerdictBanner verdict={OUT} company="Siemens" totalStudents={166} />,
    );
    expect(screen.getByText("Not this time")).toBeInTheDocument();
  });

  it("never says 'not shortlisted' when the file could not be read", () => {
    render(
      <VerdictBanner verdict={UNREADABLE} company="TCS" totalStudents={0} />,
    );
    expect(screen.queryByText("Not this time")).not.toBeInTheDocument();
    expect(
      screen.getByText(/Couldn't read this shortlist/),
    ).toBeInTheDocument();
    // And it must say so explicitly, because silence reads as rejection.
    expect(
      screen.getByText(/says nothing about whether you were shortlisted/),
    ).toBeInTheDocument();
  });

  it("never says 'not shortlisted' when the file uses a key we don't have", () => {
    render(
      <VerdictBanner verdict={WRONG_KEY} company="HPE" totalStudents={874} />,
    );
    expect(screen.queryByText("Not this time")).not.toBeInTheDocument();
    expect(screen.getByText(/Can't tell from this file/)).toBeInTheDocument();
    expect(screen.getByText(/registration number/)).toBeInTheDocument();
  });

  it("prompts for identity when none is configured", () => {
    render(
      <VerdictBanner
        verdict={NO_IDENTITY}
        company="Zluri"
        totalStudents={400}
      />,
    );
    expect(screen.getByText(/don't know who you are/)).toBeInTheDocument();
    expect(screen.queryByText("Not this time")).not.toBeInTheDocument();
  });

  it("gives the four states visually distinct treatments", () => {
    const classFor = (v: Verdict) => {
      const { container, unmount } = render(
        <VerdictBanner verdict={v} company="X" totalStudents={1} />,
      );
      const cls = container.querySelector(".verdict")!.className;
      unmount();
      return cls;
    };
    const yes = classFor(IN);
    const no = classFor(OUT);
    const unknown = classFor(UNREADABLE);

    expect(yes).toContain("yes");
    expect(no).toContain("no");
    expect(unknown).toContain("unknown");
    expect(new Set([yes, no, unknown]).size).toBe(3);
  });
});

describe("VerdictPill", () => {
  it("labels each state in words, not colour alone", () => {
    const { unmount: u1 } = render(<VerdictPill verdict={IN} />);
    expect(screen.getByText("In")).toBeInTheDocument();
    u1();

    const { unmount: u2 } = render(<VerdictPill verdict={OUT} />);
    expect(screen.getByText("Not in")).toBeInTheDocument();
    u2();

    render(<VerdictPill verdict={UNREADABLE} />);
    expect(screen.getByText("Unknown")).toBeInTheDocument();
  });

  it("shows 'Unknown' rather than 'Not in' for every undetermined reason", () => {
    for (const v of [NO_IDENTITY, WRONG_KEY, UNREADABLE]) {
      const { unmount } = render(<VerdictPill verdict={v} />);
      expect(screen.getByText("Unknown")).toBeInTheDocument();
      expect(screen.queryByText("Not in")).not.toBeInTheDocument();
      unmount();
    }
  });
});

describe("undeterminedText", () => {
  it("explains each reason distinctly and actionably", () => {
    const seen = new Set<string>();
    for (const r of [
      { reason: "no_identity_configured" },
      { reason: "key_kind_not_configured", file_key: "reg_no" },
      { reason: "key_kind_not_configured", file_key: "neo_id" },
      { reason: "file_not_understood" },
    ] as const) {
      const { title, detail } = undeterminedText(r);
      expect(title.length).toBeGreaterThan(0);
      expect(detail.length).toBeGreaterThan(0);
      seen.add(detail);
    }
    expect(seen.size).toBe(4);
  });
});
