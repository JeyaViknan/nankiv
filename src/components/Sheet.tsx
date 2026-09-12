/**
 * A sheet.
 *
 * Circle and Settings are both configured occasionally and then left alone, so
 * neither earns a permanent slot in the navigation. They are presented the same
 * way, behave the same way, and dismiss the same way — Escape, the close
 * button, or clicking the dimmed area outside.
 *
 * Focus moves into the sheet on open and returns to whatever had it on close,
 * so a keyboard user is never stranded behind the overlay.
 */

import { useEffect, useRef, type ReactNode } from "react";
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

  useEffect(() => {
    restoreTo.current = document.activeElement as HTMLElement | null;
    panelRef.current?.focus();

    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        e.stopPropagation();
        onClose();
      }
    };
    window.addEventListener("keydown", onKey, true);
    return () => {
      window.removeEventListener("keydown", onKey, true);
      restoreTo.current?.focus?.();
    };
  }, [onClose]);

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
