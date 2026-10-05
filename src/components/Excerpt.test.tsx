import { describe, expect, it } from "vitest";
import { render, screen, within } from "@testing-library/react";
import type { Evidence } from "../lib/api";
import { PASTED_SHEET } from "../lib/evidence";
import { Excerpt } from "./Excerpt";

const pasted: Evidence = {
  key: "reg_no",
  yours: "23BAI0002",
  listed: 3,
  found_at: {
    kind: "reg_no",
    value: "23BAI0002",
    sheet: PASTED_SHEET,
    row: 1,
    column: null,
    header: null,
  },
  excerpt: [
    { row: 1, value: "23BAI0002" },
    { row: 2, value: "23BCE0417" },
  ],
};

describe("the excerpt", () => {
  it("speaks of lines for a pasted list, and names the column itself", () => {
    render(<Excerpt evidence={pasted} />);
    expect(screen.getByText("What you pasted")).toBeInTheDocument();
    const sheet = screen.getByRole("table", { name: /around line 1/ });
    expect(within(sheet).getByText("Registration number")).toBeInTheDocument();
    expect(
      screen.getByText(/Matched by registration number/),
    ).toBeInTheDocument();
  });

  it("starts at your line when nothing is above it", () => {
    render(<Excerpt evidence={pasted} />);
    const rows = screen.getAllByRole("row");
    expect(rows[1]).toHaveAttribute("aria-current", "true");
    expect(rows[2]).not.toHaveAttribute("aria-current");
  });

  it("shows nothing when the file's lines were never kept", () => {
    const { container } = render(
      <Excerpt evidence={{ ...pasted, found_at: null, excerpt: [] }} />,
    );
    expect(container).toBeEmptyDOMElement();
  });
});
