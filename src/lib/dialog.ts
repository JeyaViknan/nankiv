/**
 * How a dialog behaves at the keyboard, shared by every one in the app.
 *
 * Focus moves into it on open — unless something inside has already taken
 * it, like a field marked to receive it — stays inside while it is open, Tab
 * and Shift-Tab wrapping at either end, Escape closes it, and focus returns
 * to whatever had it on close, so a keyboard user is never stranded behind
 * the overlay.
 */

import { useEffect, useRef, type RefObject } from "react";

const FOCUSABLE =
  'a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), summary, [tabindex]:not([tabindex="-1"])';

export function useDialog(
  panelRef: RefObject<HTMLElement>,
  onClose: () => void,
): void {
  // Read through a ref: callers pass a new function on every render, and
  // re-running the setup each time pulled focus out of whatever was being
  // typed in whenever anything else on screen changed.
  const close = useRef(onClose);
  close.current = onClose;

  useEffect(() => {
    const panel = panelRef.current;
    const restoreTo = document.activeElement as HTMLElement | null;
    if (panel && !panel.contains(document.activeElement)) panel.focus();

    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        e.stopPropagation();
        close.current();
        return;
      }
      if (e.key !== "Tab" || !panel) return;
      const items = [...panel.querySelectorAll<HTMLElement>(FOCUSABLE)];
      const first = items[0];
      const last = items[items.length - 1];
      if (!first || !last) return;
      const at = document.activeElement;
      if (e.shiftKey && (at === first || at === panel)) {
        e.preventDefault();
        last.focus();
      } else if (!e.shiftKey && (at === last || !panel.contains(at))) {
        e.preventDefault();
        first.focus();
      }
    };
    window.addEventListener("keydown", onKey, true);
    return () => {
      window.removeEventListener("keydown", onKey, true);
      restoreTo?.focus?.();
    };
    // The panel and its behaviour are set once, for the life of the dialog.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);
}
