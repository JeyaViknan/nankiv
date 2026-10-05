import { describe, expect, it } from "vitest";
import { fireEvent, render } from "@testing-library/react";
import { useRef } from "react";
import { HOME, usePageScroll } from "./scroll";

function Shell({ page }: { page: string }) {
  const ref = useRef<HTMLElement>(null);
  const onScroll = usePageScroll(ref, page);
  return <main data-testid="scroller" ref={ref} onScroll={onScroll} />;
}

/** Scrolls the way a person does: the position moves, then the event. */
function scrollTo(el: HTMLElement, top: number) {
  el.scrollTop = top;
  fireEvent.scroll(el);
}

describe("where a page opens", () => {
  it("opens a drive at its top, however far down the list was", () => {
    const { getByTestId, rerender } = render(<Shell page={HOME} />);
    const el = getByTestId("scroller");
    scrollTo(el, 1800);

    rerender(<Shell page="drive:7" />);
    expect(el.scrollTop).toBe(0);
  });

  it("opens the next drive at its top too", () => {
    const { getByTestId, rerender } = render(<Shell page="drive:7" />);
    const el = getByTestId("scroller");
    scrollTo(el, 900);

    rerender(<Shell page="drive:8" />);
    expect(el.scrollTop).toBe(0);
  });

  it("leaves a drive where it is when the same drive refreshes", () => {
    const { getByTestId, rerender } = render(<Shell page="drive:7" />);
    const el = getByTestId("scroller");
    scrollTo(el, 900);

    rerender(<Shell page="drive:7" />);
    expect(el.scrollTop).toBe(900);
  });

  it("returns to the same place in the list on the way back", () => {
    const { getByTestId, rerender } = render(<Shell page={HOME} />);
    const el = getByTestId("scroller");
    scrollTo(el, 1800);

    rerender(<Shell page="drive:7" />);
    // Reading the drive must not move where the list will be.
    scrollTo(el, 400);
    rerender(<Shell page={HOME} />);
    expect(el.scrollTop).toBe(1800);
  });
});
