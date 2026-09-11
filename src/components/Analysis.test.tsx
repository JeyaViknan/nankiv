/**
 * Analytics rendering rules.
 *
 * Two things must hold no matter what the engine returns: coverage is always
 * visible, and an estimate never reads as an official figure.
 */

import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import {
  BranchCard,
  CoverageNotice,
  CutoffCard,
  DistributionCard,
  InsufficientSample,
  StandingCard,
} from "./Analysis";
import type {
  BranchReport,
  CutoffReport,
  DriveAnalysis,
  Distribution,
} from "../lib/api";

const gated: DriveAnalysis = {
  total_students: 125,
  matched_students: 1,
  coverage: 0.008,
  sufficient: false,
  cgpa: null,
  cutoff: null,
  branches: null,
  your_percentile: null,
};

const cutoff: CutoffReport = {
  verdict: {
    value: { kind: "hard_cutoff", threshold: 9.0, observed_floor: 8.94 },
    matched: 25,
    total: 166,
  },
  comparison: [
    {
      threshold: 9.5,
      share_below_shortlist: 0.8,
      share_below_batch: 0.94,
      is_signal: false,
    },
    {
      threshold: 9.0,
      share_below_shortlist: 0.04,
      share_below_batch: 0.63,
      is_signal: true,
    },
    {
      threshold: 8.5,
      share_below_shortlist: 0.0,
      share_below_batch: 0.25,
      is_signal: false,
    },
  ],
  statement:
    "Looks like a CGPA cutoff around 9.0 — the lowest we found was 8.94 (based on 25 of 166 shortlisted students we could match). This is an estimate, not an official cutoff.",
};

const dist: Distribution = {
  n: 25,
  min: 8.94,
  max: 9.71,
  median: 9.28,
  mean: 9.3,
  std_dev: 0.21,
  p5: 9.01,
  p25: 9.1,
  p75: 9.5,
  buckets: [
    { lower: 8.75, upper: 9.0, count: 1 },
    { lower: 9.0, upper: 9.25, count: 10 },
    { lower: 9.25, upper: 9.5, count: 9 },
    { lower: 9.5, upper: 9.75, count: 5 },
  ],
};

const branches: BranchReport = {
  rows: {
    value: [
      {
        branch: "CSE",
        count: 17,
        share: 0.68,
        baseline_share: 0.6,
        lift: 1.13,
      },
      {
        branch: "ECE",
        count: 8,
        share: 0.32,
        baseline_share: 0.075,
        lift: 4.27,
      },
    ],
    matched: 25,
    total: 166,
  },
  over_represented: ["ECE"],
  statement:
    "ECE is noticeably over-represented compared to the batch (based on 25 of 166 students).",
};

describe("coverage disclosure", () => {
  it("always states matched over total", () => {
    render(
      <CoverageNotice
        analysis={{
          ...gated,
          matched_students: 25,
          total_students: 166,
          coverage: 0.15,
          sufficient: true,
        }}
      />,
    );
    expect(screen.getByText("25")).toBeInTheDocument();
    expect(screen.getByText("166")).toBeInTheDocument();
    expect(screen.getByText(/15%/)).toBeInTheDocument();
  });
});

describe("insufficient samples", () => {
  it("explains the refusal instead of showing a number", () => {
    // The Elgi case: one matched student out of 125.
    render(<InsufficientSample analysis={gated} />);
    expect(screen.getByText(/Not enough data to analyse/)).toBeInTheDocument();
    expect(screen.getByText(/isn't going to guess/)).toBeInTheDocument();
  });

  it("reassures that membership is unaffected", () => {
    render(<InsufficientSample analysis={gated} />);
    expect(screen.getByText(/Membership above is exact/)).toBeInTheDocument();
  });
});

describe("cutoff card", () => {
  it("labels the figure as an estimate", () => {
    render(<CutoffCard report={cutoff} />);
    expect(screen.getByText("estimate")).toBeInTheDocument();
  });

  it("shows the threshold and the honest observed floor", () => {
    render(<CutoffCard report={cutoff} />);
    // The phrase appears in both the heading and the statement; the heading is
    // the one that has to be right.
    expect(
      screen.getByRole("heading", { name: /CGPA cutoff around 9\.0/ }),
    ).toBeInTheDocument();
    expect(screen.getByText(/8\.94/)).toBeInTheDocument();
  });

  it("shows its working against the batch baseline", () => {
    render(<CutoffCard report={cutoff} />);
    expect(
      screen.getByText(/three quarters of the batch already clears 8.5/i),
    ).toBeInTheDocument();
    expect(screen.getByText("cutoff here")).toBeInTheDocument();
  });

  it("never claims to be the official cutoff", () => {
    const { container } = render(<CutoffCard report={cutoff} />);
    const text = container.textContent ?? "";
    expect(text).not.toMatch(/the official cutoff/i);
    expect(text).toMatch(/estimate/i);
  });
});

describe("distribution card", () => {
  it("renders the sample size and the range", () => {
    render(<DistributionCard dist={dist} yourCgpa={null} />);
    expect(screen.getByText("n=25")).toBeInTheDocument();
    expect(screen.getByText("8.94")).toBeInTheDocument();
    expect(screen.getByText("9.28")).toBeInTheDocument();
  });

  it("highlights the student's own CGPA when known", () => {
    render(<DistributionCard dist={dist} yourCgpa={9.41} />);
    expect(screen.getByText("9.41")).toBeInTheDocument();
    expect(screen.getByText("Yours")).toBeInTheDocument();
  });

  it("omits the personal figure when unknown", () => {
    render(<DistributionCard dist={dist} yourCgpa={null} />);
    expect(screen.queryByText("Yours")).not.toBeInTheDocument();
  });

  it("carries an accessible description of the chart", () => {
    render(<DistributionCard dist={dist} yourCgpa={null} />);
    expect(screen.getByRole("img")).toHaveAccessibleName(/median 9.28/);
  });
});

describe("branch card", () => {
  it("surfaces an over-represented branch with its lift", () => {
    render(<BranchCard report={branches} />);
    expect(
      screen.getByText(/ECE is noticeably over-represented/),
    ).toBeInTheDocument();
    expect(screen.getByText("4.3x")).toBeInTheDocument();
  });

  it("renders a dash where no baseline exists", () => {
    render(
      <BranchCard
        report={{
          ...branches,
          over_represented: [],
          rows: {
            ...branches.rows,
            value: [
              {
                branch: "CSE",
                count: 17,
                share: 0.68,
                baseline_share: null,
                lift: null,
              },
            ],
          },
        }}
      />,
    );
    expect(screen.getByText("—")).toBeInTheDocument();
  });
});

describe("standing card", () => {
  it("reports the percentile with its sample", () => {
    render(
      <StandingCard percentile={{ value: 0.68, matched: 25, total: 166 }} />,
    );
    expect(screen.getByText("68%")).toBeInTheDocument();
    expect(screen.getByText(/25 of 166/)).toBeInTheDocument();
  });
});
