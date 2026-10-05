/**
 * The order of the result page.
 *
 * Your answer, then your circle, then everything else — whatever the answer
 * was. An earlier build moved the analysis above the circle when the answer
 * was no, so the page changed shape depending on the result, and the people
 * you came to check on were pushed down exactly when you most wanted them.
 */

import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { api } from "../lib/api";
import type {
  ImportOutcome,
  PersonResult,
  Progression,
  Verdict,
} from "../lib/api";
import { useStore } from "../lib/store";
import { DriveScreen } from "./DriveScreen";

function person(label: string, verdict: Verdict): PersonResult {
  return {
    label,
    neo_id: null,
    reg_no: null,
    verdict,
    confidence: "verified",
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
    your_cgpa: null,
    evidence: {
      key: "neo_id",
      yours: "V9H0G6C4",
      listed: 96,
      found_at:
        you.status === "shortlisted"
          ? {
              kind: "neo_id",
              value: "V9H0G6C4",
              sheet: "Round 2",
              row: 42,
              column: "C",
              header: "Neo ID",
            }
          : null,
      excerpt:
        you.status === "shortlisted"
          ? [
              { row: 40, value: "H4J3V6N2" },
              { row: 41, value: "J2T0S6Y7" },
              { row: 42, value: "V9H0G6C4" },
              { row: 43, value: "D7J9E5J8" },
            ]
          : [],
    },
    progression: null,
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

describe("the evidence behind the answer", () => {
  it("shows the file around your line, once asked", async () => {
    const user = userEvent.setup();
    render(
      <DriveScreen
        outcome={outcome({ status: "shortlisted" })}
        onFix={() => {}}
      />,
    );
    const why = screen.getByText("Why does it think I'm in?");
    expect(why.closest("details")).not.toHaveAttribute("open");
    await user.click(why);
    expect(why.closest("details")).toHaveAttribute("open");

    const sheet = screen.getByRole("table", { name: /Round 2/ });
    const rows = within(sheet).getAllByRole("row");
    // The heading as the file wrote it, then the lines in file order.
    expect(rows.map((r) => r.textContent)).toEqual([
      "RowNeo ID",
      "40H4J3V6N2",
      "41J2T0S6Y7",
      "42V9H0G6C4You",
      "43D7J9E5J8",
    ]);
    // Yours, and only yours, is the one lit.
    expect(rows[3]).toHaveAttribute("aria-current", "true");
    expect(rows.filter((r) => r.hasAttribute("aria-current"))).toHaveLength(1);
    expect(screen.getByText("Round 2 · Column C")).toBeInTheDocument();
    expect(screen.getByText(/Matched by Neo ID/)).toBeInTheDocument();
  });

  it("finds the file again for a yes from before lines were kept", async () => {
    const user = userEvent.setup();
    const o = outcome({ status: "shortlisted" });
    const old = { ...o.evidence, found_at: null, excerpt: [] };
    const find = vi.spyOn(api, "findEvidence").mockResolvedValue(o.evidence);
    render(<DriveScreen outcome={{ ...o, evidence: old }} onFix={() => {}} />);

    await user.click(screen.getByText("Why does it think I'm in?"));
    expect(find).toHaveBeenCalledWith(1);
    expect(
      await screen.findByRole("table", { name: /Round 2/ }),
    ).toBeInTheDocument();

    // Asked once: closing and opening again does not look again.
    await user.click(screen.getByText("Why does it think I'm in?"));
    await user.click(screen.getByText("Why does it think I'm in?"));
    expect(find).toHaveBeenCalledTimes(1);
    find.mockRestore();
  });

  it("says it in a sentence when the file is gone", async () => {
    const user = userEvent.setup();
    const o = outcome({ status: "shortlisted" });
    const old = { ...o.evidence, found_at: null, excerpt: [] };
    const find = vi.spyOn(api, "findEvidence").mockResolvedValue(old);
    render(<DriveScreen outcome={{ ...o, evidence: old }} onFix={() => {}} />);

    await user.click(screen.getByText("Why does it think I'm in?"));
    expect(
      await screen.findByText(/V9H0G6C4, is one of 96 in this file/),
    ).toBeInTheDocument();
    expect(screen.queryByRole("table")).not.toBeInTheDocument();
    find.mockRestore();
  });

  it("asks the other question when you're not in", () => {
    render(
      <DriveScreen
        outcome={outcome({ status: "not_shortlisted" })}
        onFix={() => {}}
      />,
    );
    expect(screen.getByText("Why not?")).toBeInTheDocument();
  });
});

describe("rounds", () => {
  const progression: Progression = {
    steps: [
      { drive_id: 7, label: "R1", verdict: { status: "shortlisted" } },
      { drive_id: 1, label: "R2", verdict: { status: "shortlisted" } },
    ],
    next_pending: true,
  };

  it("shows each round with your answer, and one still to come", () => {
    render(
      <DriveScreen
        outcome={{ ...outcome({ status: "shortlisted" }), progression }}
        onFix={() => {}}
      />,
    );
    const trail = screen.getByRole("list", { name: "Rounds" });
    expect(trail).toHaveTextContent("R1");
    expect(trail).toHaveTextContent("R2");
    expect(trail).toHaveTextContent("Next");
    expect(screen.getByLabelText("R2, you're in")).toHaveAttribute(
      "aria-current",
      "step",
    );
    // Still second: the circle, then the rounds, then the analysis.
    expect(sections()).toEqual([
      "Your circle",
      "Rounds",
      "What this shortlist suggests",
    ]);
  });

  it("opens an earlier round from the trail", async () => {
    const openDrive = vi.fn().mockResolvedValue(undefined);
    useStore.setState({ openDrive });
    const user = userEvent.setup();
    render(
      <DriveScreen
        outcome={{ ...outcome({ status: "shortlisted" }), progression }}
        onFix={() => {}}
      />,
    );
    await user.click(screen.getByLabelText("R1, you're in — open"));
    expect(openDrive).toHaveBeenCalledWith(7);
  });

  it("offers to compare with the round before, and to separate a wrong link", () => {
    render(
      <DriveScreen
        outcome={{ ...outcome({ status: "shortlisted" }), progression }}
        onFix={() => {}}
      />,
    );
    expect(
      screen.getByRole("button", { name: "Compare with R1" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: /Separate it/ }),
    ).toBeInTheDocument();
  });
});

describe("a drive nothing could name", () => {
  it("asks to be named instead of showing a placeholder", async () => {
    const user = userEvent.setup();
    render(
      <DriveScreen
        outcome={{
          ...outcome({ status: "shortlisted" }),
          company: "Unnamed drive",
        }}
        onFix={() => {}}
      />,
    );
    await user.click(screen.getByRole("button", { name: "Name this drive" }));
    expect(screen.getByLabelText("Drive name")).toHaveValue("Unnamed drive");
  });
});
