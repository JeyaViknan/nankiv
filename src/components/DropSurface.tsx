/**
 * The drag interaction.
 *
 * The whole window is the target, because the whole window already *was* the
 * target — an earlier build listened at the window level while only a small
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
 *
 * These are the web view's own drag events rather than the ones the window
 * layer reports. Tauri's native handler is switched off in tauri.conf.json:
 * on macOS it reads dropped paths through a pasteboard type that has been
 * deprecated for years, and on recent systems the app is handed a drop with
 * nothing in it. The web view's own events are the platform's supported path,
 * and they carry the file itself rather than a path to it.
 */

import { useEffect, useState } from "react";
import { Icon } from "./Icon";

const SPREADSHEET = ["xlsx", "xls", "xlsm", "ods", "csv"];

export function isSpreadsheet(path: string): boolean {
  const ext = path.split(".").pop()?.toLowerCase() ?? "";
  return SPREADSHEET.includes(ext);
}

export function basename(path: string): string {
  return path.split(/[\\/]/).pop() ?? path;
}

/** The first spreadsheet in a drag, or the first file to explain the refusal. */
export function chooseFile(files: File[]): { file: File; ok: boolean } | null {
  const good = files.find((f) => isSpreadsheet(f.name));
  if (good) return { file: good, ok: true };
  const first = files[0];
  return first ? { file: first, ok: false } : null;
}

/** Whether a drag carries files at all, rather than selected text or a link. */
export function carriesFiles(transfer: DataTransfer | null): boolean {
  if (!transfer) return false;
  return Array.from(transfer.types ?? []).includes("Files");
}

type DragState =
  | { phase: "idle" }
  | { phase: "valid"; name: string }
  | { phase: "invalid"; name: string };

/**
 * Registers the window-level drag handling and renders the overlay.
 *
 * Mounted once, at the shell. Every view is a drop target for free.
 */
export function DropSurface({
  onFile,
  disabled,
}: {
  onFile: (file: File) => void;
  disabled?: boolean;
}) {
  const [drag, setDrag] = useState<DragState>({ phase: "idle" });

  useEffect(() => {
    // Counted rather than toggled: dragging across a child element fires
    // `dragleave` for the one being left, and the overlay would flicker.
    let depth = 0;

    const over = (event: DragEvent) => {
      if (!carriesFiles(event.dataTransfer)) return;
      // Without this the web view opens the file itself, replacing the app.
      event.preventDefault();
      if (event.dataTransfer) event.dataTransfer.dropEffect = "copy";
    };

    const enter = (event: DragEvent) => {
      if (!carriesFiles(event.dataTransfer)) return;
      event.preventDefault();
      depth += 1;
      // Names only arrive with the drop itself; during the drag the web view
      // reports how many files there are and nothing else.
      const items = Array.from(event.dataTransfer?.items ?? []);
      const names = items.filter((i) => i.kind === "file").length;
      setDrag({ phase: "valid", name: names > 1 ? `${names} files` : "" });
    };

    const leave = (event: DragEvent) => {
      if (!carriesFiles(event.dataTransfer)) return;
      depth = Math.max(0, depth - 1);
      if (depth === 0) setDrag({ phase: "idle" });
    };

    const drop = (event: DragEvent) => {
      if (!carriesFiles(event.dataTransfer)) return;
      event.preventDefault();
      depth = 0;
      const chosen = chooseFile(Array.from(event.dataTransfer?.files ?? []));
      if (!chosen) {
        setDrag({ phase: "idle" });
        return;
      }
      if (!chosen.ok) {
        setDrag({ phase: "invalid", name: chosen.file.name });
        window.setTimeout(() => setDrag({ phase: "idle" }), 2500);
        return;
      }
      setDrag({ phase: "idle" });
      if (!disabled) onFile(chosen.file);
    };

    window.addEventListener("dragenter", enter);
    window.addEventListener("dragover", over);
    window.addEventListener("dragleave", leave);
    window.addEventListener("drop", drop);
    return () => {
      window.removeEventListener("dragenter", enter);
      window.removeEventListener("dragover", over);
      window.removeEventListener("dragleave", leave);
      window.removeEventListener("drop", drop);
    };
  }, [onFile, disabled]);

  if (drag.phase === "idle" || disabled) return null;

  const invalid = drag.phase === "invalid";

  return (
    <div className={`drag-veil${invalid ? " invalid" : ""}`} aria-hidden="true">
      <div className="drag-card">
        <span className="drag-icon">
          <Icon name={invalid ? "close" : "tray"} size={38} />
        </span>
        <p className="drag-title">
          {invalid ? "That file won't work" : "Drop to analyse"}
        </p>
        {drag.name && <p className="drag-name">{drag.name}</p>}
        {invalid && (
          <p className="drag-reason">
            nankiv reads spreadsheets — .xlsx, .xls or .csv.
          </p>
        )}
      </div>
    </div>
  );
}
