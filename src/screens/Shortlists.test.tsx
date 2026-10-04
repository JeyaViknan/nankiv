/**
 * The list of shortlists reads as a season: each row says whether you made it,
 * and which round it was, without opening anything.
 */

import { render, screen, within } from "@testing-library/react";
import { beforeEach, describe, expect, it } from "vitest";
import type { DriveListItem, Verdict } from "../lib/api";
import { useStore } from "../lib/store";
import { Shortlists } from "./Shortlists";

function drive(
  id: number,
  company: string,
  verdict: Verdict,
  round: string | null = null,
): DriveListItem {
  return {
    id,
    company,
    drive_date: null,
    imported_at: "2026-09-01 10:00:00",
    source_filename: `${company}.xlsx`,
    content_hash: `H${id}`,
    shape: "neo_id_only",
    primary_key: "neo_id",
    total_students: 120,
    round_label: null,
    parent_drive_id: null,
    verdict,
    round,
  };
}

beforeEach(() => {
  useStore.setState({
    drives: [
      drive(1, "Siemens SISW", { status: "shortlisted" }, "R2"),
      drive(2, "Zluri", { status: "not_shortlisted" }),
      drive(3, "Deloitte", {
        status: "undetermined",
        reason: "key_kind_not_configured",
        file_key: "reg_no",
      }),
    ],
    importStage: { phase: "idle" },
    profile: null,
    stats: null,
  });
});

describe("each shortlist row", () => {
  it("says whether you made it, in words", () => {
    render(<Shortlists onBrowse={() => {}} />);
    const row = (name: string) =>
      screen.getByText(name).closest("button") as HTMLElement;
    expect(within(row("Siemens SISW")).getByText("In")).toBeInTheDocument();
    expect(within(row("Zluri")).getByText("Not in")).toBeInTheDocument();
    expect(within(row("Deloitte")).getByText("Unknown")).toBeInTheDocument();
  });

  it("names the round when it is one of several", () => {
    render(<Shortlists onBrowse={() => {}} />);
    const siemens = screen.getByText("Siemens SISW").closest("button")!;
    expect(siemens).toHaveTextContent("R2");
    const zluri = screen.getByText("Zluri").closest("button")!;
    expect(zluri).not.toHaveTextContent(/R\d/);
  });
});
