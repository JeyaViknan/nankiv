/**
 * The drag interaction.
 *
 * The whole window is the target, because the whole window already *was* the
 * target — the previous build listened at the webview level while only a small
 * dashed rectangle reacted, so the app quietly accepted more than it admitted
 * to. An affordance that lies is worse than no affordance.
 *
 * The sequence is designed as one continuous motion rather than a widget with
 * states:
 *
 *   enter   the window recedes and one clear invitation appears
 *   invalid rejected during the drag, with the reason, before the drop
 *   drop    acknowledged by name before anything is computed
 *
 * Rejecting an unsupported file mid-drag matters more than it sounds: the
 * alternative is letting someone complete the gesture and then telling them it
 * was wrong, which is the interaction equivalent of a shrug.
 */

import { useEffect, useState } from "react";
import { getCurrentWebview } from "@tauri-apps/api/webview";

const SPREADSHEET = ["xlsx", "xls", "xlsm", "ods", "csv"];

export function isSpreadsheet(path: string): boolean {
  const ext = path.split(".").pop()?.toLowerCase() ?? "";
  return SPREADSHEET.includes(ext);
}

export function basename(path: string): string {
  return path.split(/[\\/]/).pop() ?? path;
}

type DragState =
  | { phase: "idle" }
  | { phase: "valid"; name: string }
  | { phase: "invalid"; name: string };

function TrayIcon() {
  return (
    <svg
      width="40"
      height="40"
      viewBox="0 0 40 40"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.6"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <path d="M20 6v18M13.5 17.5L20 24l6.5-6.5" />
      <path d="M7 26v4.5A2.5 2.5 0 0 0 9.5 33h21a2.5 2.5 0 0 0 2.5-2.5V26" />
    </svg>
  );
}

function BlockedIcon() {
  return (
    <svg
      width="40"
      height="40"
      viewBox="0 0 40 40"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.6"
      strokeLinecap="round"
      aria-hidden="true"
    >
      <circle cx="20" cy="20" r="13" />
      <path d="M11 11l18 18" />
    </svg>
  );
}

/**
 * Registers the window-level drag handling and renders the overlay.
 *
 * Mounted once, at the shell. Every view is a drop target for free.
 */
export function DropSurface({
  onFile,
  disabled,
}: {
  onFile: (path: string) => void;
  disabled?: boolean;
}) {
  const [drag, setDrag] = useState<DragState>({ phase: "idle" });

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let cancelled = false;

    getCurrentWebview()
      .onDragDropEvent((event) => {
        const p = event.payload;

        // `enter` and `drop` carry paths; `over` carries only a position, so
        // the verdict reached on enter is held for the rest of the drag.
        if (p.type === "enter") {
          const paths = p.paths ?? [];
          const good = paths.find(isSpreadsheet);
          if (good) {
            setDrag({ phase: "valid", name: basename(good) });
          } else if (paths.length > 0) {
            setDrag({ phase: "invalid", name: basename(paths[0]!) });
          }
          return;
        }

        if (p.type === "drop") {
          setDrag({ phase: "idle" });
          const good = (p.paths ?? []).find(isSpreadsheet);
          if (good && !disabled) onFile(good);
          return;
        }

        if (p.type === "leave") setDrag({ phase: "idle" });
      })
      .then((fn) => {
        if (cancelled) fn();
        else unlisten = fn;
      })
      .catch(() => {
        // Outside the Tauri shell there is no drag event source. The Open
        // command still works, so this is not worth surfacing.
      });

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [onFile, disabled]);

  if (drag.phase === "idle" || disabled) return null;

  const invalid = drag.phase === "invalid";

  return (
    <div className={`drag-veil${invalid ? " invalid" : ""}`} aria-hidden="true">
      <div className="drag-card">
        <span className="drag-icon">
          {invalid ? <BlockedIcon /> : <TrayIcon />}
        </span>
        <p className="drag-title">
          {invalid ? "That file won't work" : "Drop to analyse"}
        </p>
        <p className="drag-name">{drag.name}</p>
        {invalid && (
          <p className="drag-reason">
            nankiv reads spreadsheets — .xlsx, .xls or .csv.
          </p>
        )}
      </div>
    </div>
  );
}
