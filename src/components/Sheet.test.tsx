import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { Sheet } from "./Sheet";

function Example({ onClose = () => {} }: { onClose?: () => void }) {
  return (
    <Sheet title="Example" onClose={onClose}>
      <input aria-label="First field" />
      <button>Last button</button>
    </Sheet>
  );
}

describe("a sheet, from the keyboard", () => {
  it("takes focus when it opens, and gives it back when it closes", () => {
    const before = document.createElement("button");
    document.body.appendChild(before);
    before.focus();
    const { unmount } = render(<Example />);
    expect(screen.getByRole("dialog")).toHaveFocus();
    unmount();
    expect(before).toHaveFocus();
    before.remove();
  });

  it("leaves focus where a field inside already took it", () => {
    render(
      <Sheet title="Example" onClose={() => {}}>
        <input aria-label="Company" autoFocus />
      </Sheet>,
    );
    expect(screen.getByLabelText("Company")).toHaveFocus();
  });

  it("keeps Tab inside, wrapping at either end", async () => {
    const user = userEvent.setup();
    render(<Example />);
    await user.tab(); // close button
    await user.tab(); // first field
    await user.tab(); // last button
    expect(screen.getByText("Last button")).toHaveFocus();
    await user.tab();
    expect(screen.getByRole("button", { name: "Close Example" })).toHaveFocus();
    await user.tab({ shift: true });
    expect(screen.getByText("Last button")).toHaveFocus();
  });

  it("closes on Escape", async () => {
    const onClose = vi.fn();
    const user = userEvent.setup();
    render(<Example onClose={onClose} />);
    await user.keyboard("{Escape}");
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("does not steal focus back when the page around it re-renders", async () => {
    const user = userEvent.setup();
    const { rerender } = render(<Example onClose={() => {}} />);
    await user.click(screen.getByLabelText("First field"));
    rerender(<Example onClose={() => {}} />);
    expect(screen.getByLabelText("First field")).toHaveFocus();
  });
});
