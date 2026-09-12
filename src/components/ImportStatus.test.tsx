/**
 * The moment between the drop and the answer.
 *
 * The rule under test: a file nankiv cannot read must never be presented in a
 * way that could be mistaken for "you were not shortlisted". Silence, or an
 * alarm, both read as a rejection — so the surface says so explicitly.
 */

import { render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it } from "vitest";
import { ImportStatus } from "./ImportStatus";
import { useStore } from "../lib/store";
import { isSpreadsheet, basename } from "./DropSurface";

beforeEach(() => {
  useStore.setState({ importStage: { phase: "idle" } });
});

describe("reading", () => {
  it("names the file so the student knows the drop landed", () => {
    useStore.setState({
      importStage: { phase: "reading", filename: "Siemens SISW 2027.xlsx" },
    });
    render(<ImportStatus />);
    expect(screen.getByText("Siemens SISW 2027.xlsx")).toBeInTheDocument();
    expect(screen.getByRole("status")).toBeInTheDocument();
  });

  it("shows nothing at all when idle", () => {
    const { container } = render(<ImportStatus />);
    expect(container).toBeEmptyDOMElement();
  });
});

describe("failure", () => {
  beforeEach(() => {
    useStore.setState({
      importStage: {
        phase: "failed",
        filename: "TCS Shortlist 27th (Cognizant).xlsx",
        error: {
          code: "file_not_understood",
          message: "This file doesn't use Neo IDs or registration numbers.",
          detail: "Columns found: REFERENCE_ID, Interview Date",
        },
      },
    });
  });

  it("states plainly that this is not a result", () => {
    render(<ImportStatus />);
    expect(
      screen.getByText(/says nothing about whether you were shortlisted/i),
    ).toBeInTheDocument();
  });

  it("never uses the words of a rejection", () => {
    const { container } = render(<ImportStatus />);
    const text = container.textContent ?? "";
    expect(text).not.toMatch(/not shortlisted/i);
    expect(text).not.toMatch(/not this time/i);
  });

  it("shows which file failed and what was found instead", () => {
    render(<ImportStatus />);
    expect(
      screen.getByText("TCS Shortlist 27th (Cognizant).xlsx"),
    ).toBeInTheDocument();
    expect(screen.getByText(/REFERENCE_ID/)).toBeInTheDocument();
  });
});

describe("drag validation", () => {
  it("accepts the spreadsheet formats the parser handles", () => {
    for (const f of ["a.xlsx", "b.XLS", "c.csv", "d.ods", "e.xlsm"]) {
      expect(isSpreadsheet(f)).toBe(true);
    }
  });

  it("rejects everything else, so it can be refused during the drag", () => {
    for (const f of ["a.pdf", "b.png", "notes.txt", "archive.zip", "noext"]) {
      expect(isSpreadsheet(f)).toBe(false);
    }
  });

  it("shows a filename, not a path, in the drag overlay", () => {
    expect(basename("/Users/x/Downloads/Siemens.xlsx")).toBe("Siemens.xlsx");
    expect(basename("C:\\Users\\x\\Siemens.xlsx")).toBe("Siemens.xlsx");
  });
});
