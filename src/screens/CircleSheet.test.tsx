/**
 * Removing someone from your circle is immediate, and undoable — the same
 * bargain as deleting a drive, instead of an "Are you sure?" dialog.
 */

import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { api, type Friend } from "../lib/api";
import { useStore } from "../lib/store";
import { CircleSheet } from "./CircleSheet";

vi.mock("../lib/api", async () => {
  const actual =
    await vi.importActual<typeof import("../lib/api")>("../lib/api");
  return {
    ...actual,
    api: {
      ...actual.api,
      removeFriend: vi.fn(),
      addFriend: vi.fn(),
      listFriends: vi.fn(),
    },
  };
});

const arjun: Friend = {
  id: 3,
  label: "Arjun",
  neo_id: "C5U6K1E7",
  reg_no: "23BAI0009",
  group_tag: "core",
};

beforeEach(() => {
  vi.clearAllMocks();
  vi.mocked(api.removeFriend).mockResolvedValue(undefined);
  vi.mocked(api.addFriend).mockResolvedValue(4);
  vi.mocked(api.listFriends).mockResolvedValue([]);
  useStore.setState({ friends: [arjun], toast: null });
});

describe("removing someone from your circle", () => {
  it("happens at once, and offers Undo", async () => {
    const user = userEvent.setup();
    render(<CircleSheet onClose={() => {}} />);
    await user.click(screen.getByRole("button", { name: "Remove Arjun" }));

    expect(api.removeFriend).toHaveBeenCalledWith(3);
    const toast = useStore.getState().toast;
    expect(toast?.message).toBe("Removed Arjun");
    expect(toast?.undo).toBeTypeOf("function");
  });

  it("puts them back exactly as they were", async () => {
    const user = userEvent.setup();
    render(<CircleSheet onClose={() => {}} />);
    await user.click(screen.getByRole("button", { name: "Remove Arjun" }));

    useStore.getState().toast?.undo?.();
    await vi.waitFor(() =>
      expect(api.addFriend).toHaveBeenCalledWith(
        "Arjun",
        "C5U6K1E7",
        "23BAI0009",
        "core",
      ),
    );
  });
});
