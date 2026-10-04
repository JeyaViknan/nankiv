import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { api } from "../lib/api";
import { useStore } from "../lib/store";
import { PasteSheet } from "./PasteSheet";

vi.mock("../lib/api", async () => {
  const actual =
    await vi.importActual<typeof import("../lib/api")>("../lib/api");
  return { ...actual, api: { ...actual.api, previewPaste: vi.fn() } };
});

const TEXT = "Siemens SISW interview list\nV9H0G6C4\nC5U6K1E7";

beforeEach(() => {
  vi.clearAllMocks();
  vi.mocked(api.previewPaste).mockResolvedValue({
    neo_ids: 2,
    reg_nos: 0,
    unread_lines: 1,
    company: "Siemens SISW",
  });
});

describe("pasting a list", () => {
  it("says what it found before anything is stored", async () => {
    render(<PasteSheet text={TEXT} onClose={() => {}} />);
    expect(await screen.findByText("2 Neo IDs")).toBeInTheDocument();
    expect(screen.getByRole("status")).toHaveTextContent(
      "1 line without one was skipped.",
    );
    expect(screen.getByLabelText("Company")).toHaveValue("Siemens SISW");
  });

  it("checks the list under the name chosen", async () => {
    const importPasted = vi.fn().mockResolvedValue(undefined);
    useStore.setState({ importPasted });
    const onClose = vi.fn();
    const user = userEvent.setup();
    render(<PasteSheet text={TEXT} onClose={onClose} />);

    const company = await screen.findByLabelText("Company");
    await user.clear(company);
    await user.type(company, "Siemens");
    await user.click(screen.getByRole("button", { name: "Check this list" }));

    expect(onClose).toHaveBeenCalled();
    expect(importPasted).toHaveBeenCalledWith(TEXT, "Siemens");
  });

  it("explains a paste with nothing to check", async () => {
    vi.mocked(api.previewPaste).mockRejectedValue({
      code: "nothing_to_paste",
      message:
        "There are no Neo IDs or registration numbers in what you pasted.",
      detail: null,
    });
    render(<PasteSheet text="hello" onClose={() => {}} />);
    expect(await screen.findByRole("alert")).toHaveTextContent(
      "There are no Neo IDs or registration numbers",
    );
    expect(
      screen.queryByRole("button", { name: "Check this list" }),
    ).not.toBeInTheDocument();
  });
});
