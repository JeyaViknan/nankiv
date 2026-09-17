/**
 * The pop-up menu behind "Download names".
 *
 * It stands in for a native menu, so it has to behave like one from the
 * keyboard as well as the mouse — otherwise the export is mouse-only.
 */

import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { MenuButton } from "./MenuButton";

const choices = [
  { id: "xlsx", label: "Excel workbook", hint: ".xlsx" },
  { id: "csv", label: "CSV", hint: ".csv" },
];

function setup() {
  const onChoose = vi.fn();
  const user = userEvent.setup();
  render(
    <MenuButton label="Download names" choices={choices} onChoose={onChoose} />,
  );
  const button = screen.getByRole("button", { name: /Download names/ });
  return { onChoose, user, button };
}

describe("MenuButton", () => {
  it("starts closed and says it has a menu", () => {
    const { button } = setup();
    expect(button).toHaveAttribute("aria-haspopup", "menu");
    expect(button).toHaveAttribute("aria-expanded", "false");
    expect(screen.queryByRole("menu")).not.toBeInTheDocument();
  });

  it("opens on click and offers both formats", async () => {
    const { user, button } = setup();
    await user.click(button);
    expect(button).toHaveAttribute("aria-expanded", "true");
    expect(screen.getAllByRole("menuitem")).toHaveLength(2);
  });

  it("reports the chosen format and closes", async () => {
    const { user, button, onChoose } = setup();
    await user.click(button);
    await user.click(screen.getByRole("menuitem", { name: /CSV/ }));
    expect(onChoose).toHaveBeenCalledWith("csv");
    expect(screen.queryByRole("menu")).not.toBeInTheDocument();
  });

  it("is fully operable from the keyboard", async () => {
    const { user, button, onChoose } = setup();
    button.focus();
    await user.keyboard("{ArrowDown}");
    expect(screen.getByRole("menuitem", { name: /Excel/ })).toHaveFocus();
    await user.keyboard("{ArrowDown}");
    expect(screen.getByRole("menuitem", { name: /CSV/ })).toHaveFocus();
    await user.keyboard("{Enter}");
    expect(onChoose).toHaveBeenCalledWith("csv");
  });

  it("wraps around at either end", async () => {
    const { user, button } = setup();
    button.focus();
    await user.keyboard("{ArrowDown}{ArrowUp}");
    expect(screen.getByRole("menuitem", { name: /CSV/ })).toHaveFocus();
  });

  it("closes on Escape and returns focus to the button", async () => {
    const { user, button, onChoose } = setup();
    await user.click(button);
    await user.keyboard("{Escape}");
    expect(screen.queryByRole("menu")).not.toBeInTheDocument();
    expect(button).toHaveFocus();
    expect(onChoose).not.toHaveBeenCalled();
  });

  it("closes when clicking elsewhere without choosing", async () => {
    const { user, button, onChoose } = setup();
    await user.click(button);
    await user.click(document.body);
    expect(screen.queryByRole("menu")).not.toBeInTheDocument();
    expect(onChoose).not.toHaveBeenCalled();
  });
});
