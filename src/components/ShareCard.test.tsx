import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { ImportOutcome } from "../lib/api";
import { useStore } from "../lib/store";
import { drawCard } from "../lib/shareCard";
import { ShareCard } from "./ShareCard";

// jsdom has no canvas drawing; the card's own drawing is tested elsewhere.
vi.mock("../lib/shareCard", async () => {
  const actual =
    await vi.importActual<typeof import("../lib/shareCard")>(
      "../lib/shareCard",
    );
  return {
    ...actual,
    drawCard: vi.fn().mockResolvedValue(undefined),
    cardPng: vi.fn().mockResolvedValue(new Blob(["png"])),
  };
});

const outcome = {
  drive_id: 1,
  company: "Siemens SISW",
  total_students: 149,
  progression: null,
} as unknown as ImportOutcome;

beforeEach(() => {
  useStore.setState({ drives: [] });
  Object.defineProperty(globalThis, "ClipboardItem", {
    value: class {
      constructor(public items: Record<string, unknown>) {}
    },
    configurable: true,
  });
});

describe("sharing the card", () => {
  it("presents the card on its own, with its actions", async () => {
    render(<ShareCard outcome={outcome} onClose={() => {}} />);
    expect(
      screen.getByRole("dialog", { name: "Share the news" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("img", { name: /Shortlisted — Siemens SISW/ }),
    ).toBeInTheDocument();
    expect(screen.queryByRole("heading")).not.toBeInTheDocument();
    await waitFor(() =>
      expect(screen.getByRole("button", { name: "Copy image" })).toBeEnabled(),
    );
    expect(screen.getByRole("button", { name: "Save image" })).toBeEnabled();
    // Symbols only: no words on the buttons themselves.
    for (const name of ["Another line", "Copy image", "Save image"]) {
      expect(screen.getByRole("button", { name })).toHaveTextContent("");
    }
  });

  it("never says how many made the list", () => {
    render(<ShareCard outcome={outcome} onClose={() => {}} />);
    const card = screen.getByRole("img", { name: /Shortlisted/ });
    expect(card.getAttribute("aria-label")).not.toMatch(/149/);
  });

  it("offers another line, and draws the card again with it", async () => {
    const user = userEvent.setup();
    render(<ShareCard outcome={outcome} onClose={() => {}} />);
    const card = screen.getByRole("img", { name: /Shortlisted/ });
    const first = card.getAttribute("aria-label");
    vi.mocked(drawCard).mockClear();

    await user.click(screen.getByRole("button", { name: "Another line" }));
    expect(card.getAttribute("aria-label")).not.toBe(first);
    const [, , line] = vi.mocked(drawCard).mock.lastCall!;
    expect(card.getAttribute("aria-label")).toContain(line);
  });

  it("confirms a copy with a tick, and says so to a screen reader", async () => {
    // After setup, which puts its own clipboard in place.
    const user = userEvent.setup();
    const write = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, "clipboard", {
      value: { write },
      configurable: true,
    });
    render(<ShareCard outcome={outcome} onClose={() => {}} />);
    const copy = await screen.findByRole("button", { name: "Copy image" });
    await waitFor(() => expect(copy).toBeEnabled());
    await user.click(copy);
    expect(write).toHaveBeenCalled();
    expect(await screen.findByRole("status")).toHaveTextContent("Copied");
  });

  it("goes away with Escape, the close button, or a click outside", async () => {
    const onClose = vi.fn();
    const user = userEvent.setup();
    const { unmount } = render(
      <ShareCard outcome={outcome} onClose={onClose} />,
    );
    await user.keyboard("{Escape}");
    await user.click(screen.getByRole("button", { name: "Close" }));
    await user.pointer({
      keys: "[MouseLeft]",
      target: screen.getByRole("dialog"),
    });
    expect(onClose).toHaveBeenCalledTimes(3);
    unmount();
  });
});
