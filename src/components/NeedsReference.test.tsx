/**
 * "Not set up" is not "not enough data".
 *
 * The bug: a fresh install reported "we matched 0 of 1815 students (0%) — too
 * few to say anything honest", which describes a thin sample. The real
 * situation was that nankiv held no academic records at all. A bigger shortlist
 * would never have fixed it, and the message pointed nowhere.
 */

import { render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { NeedsReference } from "./NeedsReference";
import { InsufficientSample } from "./Analysis";
import { useStore } from "../lib/store";
import type { DriveAnalysis } from "../lib/api";

vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn() }));

const thinSample: DriveAnalysis = {
  total_students: 125,
  matched_students: 1,
  coverage: 0.008,
  sufficient: false,
  cgpa: null,
  cutoff: null,
  branches: null,
  your_percentile: null,
};

beforeEach(() => {
  useStore.setState({ stats: null, current: null });
});

describe("the not-set-up surface", () => {
  it("says analysis isn't set up, not that data is insufficient", () => {
    render(<NeedsReference />);
    expect(screen.getByText(/Analysis isn't set up yet/i)).toBeInTheDocument();
    expect(screen.queryByText(/too few/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/bigger sample/i)).not.toBeInTheDocument();
  });

  it("offers the action that actually resolves it", () => {
    render(<NeedsReference />);
    expect(
      screen.getByRole("button", { name: /Add a reference sheet/i }),
    ).toBeInTheDocument();
  });

  it("reassures that membership is unaffected", () => {
    // The shortlist itself is fine. Nothing here casts doubt on the verdict.
    render(<NeedsReference />);
    expect(screen.getByText(/Membership above is exact/i)).toBeInTheDocument();
  });

  it("states the data minimisation up front, where consent is given", () => {
    render(<NeedsReference />);
    expect(
      screen.getByText(/discarded as the file is read/i),
    ).toBeInTheDocument();
  });

  it("has a compact form that still carries the action", () => {
    render(<NeedsReference compact />);
    expect(screen.getByText(/Analysis isn't set up yet/i)).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: /Add sheet/i }),
    ).toBeInTheDocument();
  });
});

describe("the thin-sample surface", () => {
  it("remains distinct, for when reference data does exist", () => {
    render(<InsufficientSample analysis={thinSample} />);
    expect(screen.getByText(/Not enough data to analyse/i)).toBeInTheDocument();
    expect(screen.getByText(/125/)).toBeInTheDocument();
  });

  it("does not claim setup is missing", () => {
    const { container } = render(<InsufficientSample analysis={thinSample} />);
    expect(container.textContent ?? "").not.toMatch(/isn't set up/i);
  });
});

describe("the two are never confused", () => {
  it("produces different text for the two situations", () => {
    const a = render(<NeedsReference />).container.textContent ?? "";
    const b =
      render(<InsufficientSample analysis={thinSample} />).container
        .textContent ?? "";
    expect(a).not.toEqual(b);
    // Only one of them asks for a bigger sample; only one offers a fix.
    expect(b).toMatch(/bigger sample/i);
    expect(a).not.toMatch(/bigger sample/i);
  });
});
