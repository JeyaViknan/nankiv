/**
 * A sheet.
 *
 * Circle and Settings are both configured occasionally and then left alone, so
 * neither earns a permanent slot in the navigation. They are presented the same
 * way, behave the same way, and dismiss the same way — Escape, the close
 * button, or clicking the dimmed area outside.
 *
 * Focus moves into the sheet on open — unless something inside it has already
 * taken it, like a field marked to receive it — stays inside while it is open,
 * Tab and Shift-Tab wrapping at either end, and returns to whatever had it on
 * close, so a keyboard user is never stranded behind the overlay.
 */

import { useEffect, useRef, type ReactNode } from "react";

const FOCUSABLE =
  'a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), summary, [tabindex]:not([tabindex="-1"])';
import { Icon } from "./Icon";

export function Sheet({
  title,
  onClose,
  children,
  width = 620,
}: {
  title: string;
  onClose: () => void;
  children: ReactNode;
  width?: number;
}) {
  const panelRef = useRef<HTMLDivElement>(null);
  const restoreTo = useRef<HTMLElement | null>(null);
  // Read through a ref: callers pass a new function on every render, and
  // re-running the setup each time pulled focus out of whatever was being
  // typed in whenever anything else on screen changed.
  const close = useRef(onClose);
  close.current = onClose;

  useEffect(() => {
    const panel = panelRef.current;
    restoreTo.current = document.activeElement as HTMLElement | null;
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
      restoreTo.current?.focus?.();
    };
  }, []);

  return (
    <div
      className="scrim"
      role="presentation"
      onMouseDown={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div
        className="sheet"
        style={{ maxWidth: width }}
        role="dialog"
        aria-modal="true"
        aria-label={title}
        tabIndex={-1}
        ref={panelRef}
      >
        <header className="sheet-head">
          <h2>{title}</h2>
          <button
            className="icon-btn"
            onClick={onClose}
            aria-label={`Close ${title}`}
          >
            <Icon name="close" size={15} />
          </button>
        </header>
        <div className="sheet-body">{children}</div>
      </div>
    </div>
  );
}
