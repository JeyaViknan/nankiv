/**
 * Where a page opens.
 *
 * One scroller holds every page, so a page arriving would otherwise start
 * wherever the last one was left: a drive opened from far down the list
 * opened at its own bottom. A page opens at its top, and going back to the
 * list returns to where it was, as it does in Mail or Finder.
 */

import { useCallback, useLayoutEffect, useRef, type RefObject } from "react";

/** The page whose position is kept: the list of shortlists. */
export const HOME = "home";

/**
 * Puts the scroller at the top of each new `page`, or back where it was for
 * `HOME`. Returns the scroller's `onScroll`, which remembers that position.
 */
export function usePageScroll(
  scroller: RefObject<HTMLElement | null>,
  page: string,
): () => void {
  const left = useRef(0);
  const showing = useRef(page);

  // Before paint, so a page never shows at the old position first.
  useLayoutEffect(() => {
    showing.current = page;
    const el = scroller.current;
    if (el) el.scrollTop = page === HOME ? left.current : 0;
  }, [page, scroller]);

  return useCallback(() => {
    // The scroll a page change causes arrives after it, and is not the list's.
    if (showing.current === HOME && scroller.current) {
      left.current = scroller.current.scrollTop;
    }
  }, [scroller]);
}
