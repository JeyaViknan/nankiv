import { act } from "@testing-library/react";
import { renderHook } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { mightBeShortlist, usePastedShortlist } from "./paste";

function paste(text: string, target: EventTarget = document.body) {
  const e = new Event("paste", { bubbles: true, cancelable: true });
  Object.defineProperty(e, "clipboardData", {
    value: { getData: () => text },
  });
  Object.defineProperty(e, "target", { value: target });
  act(() => {
    document.dispatchEvent(e);
  });
  return e;
}

describe("what might be a shortlist", () => {
  it("is anything carrying an identifier", () => {
    expect(mightBeShortlist("1. Arjun - V9H0G6C4")).toBe(true);
    expect(mightBeShortlist("reg no 23BCE1473 please")).toBe(true);
    expect(mightBeShortlist("v9h0g6c4")).toBe(true);
  });

  it("is not ordinary text", () => {
    expect(mightBeShortlist("See you at 9:30 in SJT")).toBe(false);
    expect(mightBeShortlist("A1B2C3D4E5")).toBe(false);
    expect(mightBeShortlist("")).toBe(false);
  });
});

describe("pasting into the window", () => {
  it("hands a list of identifiers to nankiv", () => {
    const onList = vi.fn();
    renderHook(() => usePastedShortlist(onList, true));
    const e = paste("V9H0G6C4\nC5U6K1E7");
    expect(onList).toHaveBeenCalledWith("V9H0G6C4\nC5U6K1E7");
    expect(e.defaultPrevented).toBe(true);
  });

  it("leaves a paste into a text field alone", () => {
    const onList = vi.fn();
    renderHook(() => usePastedShortlist(onList, true));
    const input = document.createElement("input");
    paste("V9H0G6C4", input);
    expect(onList).not.toHaveBeenCalled();
  });

  it("leaves ordinary text alone", () => {
    const onList = vi.fn();
    renderHook(() => usePastedShortlist(onList, true));
    const e = paste("hello");
    expect(onList).not.toHaveBeenCalled();
    expect(e.defaultPrevented).toBe(false);
  });

  it("does nothing while it is switched off", () => {
    const onList = vi.fn();
    renderHook(() => usePastedShortlist(onList, false));
    paste("V9H0G6C4");
    expect(onList).not.toHaveBeenCalled();
  });
});
