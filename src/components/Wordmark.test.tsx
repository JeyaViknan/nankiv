import { act, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useStore } from "../lib/store";
import { Wordmark } from "./Wordmark";

beforeEach(() => {
  useStore.setState({ nameTurnedAt: null, toast: null });
});

describe("the name", () => {
  it("reads as nankiv to a screen reader, turned or not", () => {
    render(<Wordmark />);
    expect(screen.getByLabelText("nankiv")).toBeInTheDocument();
  });

  it("turns round when its maker is searched for, and says so once", () => {
    vi.useFakeTimers();
    render(<Wordmark />);
    act(() => useStore.getState().turnTheName());
    const mark = screen.getByLabelText("nankiv");
    expect(mark).toHaveClass("turned");
    const back = [...mark.querySelectorAll(".wm-back")]
      .map((e) => e.textContent)
      .join("");
    expect(back).toBe("viknan");
    expect(useStore.getState().toast?.message).toBe(
      "made by Viknan, backwards",
    );

    // Typing on doesn't repeat it.
    useStore.setState({ toast: null });
    act(() => useStore.getState().turnTheName());
    expect(useStore.getState().toast).toBeNull();

    act(() => vi.advanceTimersByTime(3000));
    expect(mark).not.toHaveClass("turned");
    vi.useRealTimers();
  });
});
