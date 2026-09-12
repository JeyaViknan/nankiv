/**
 * The drop target.
 *
 * Present on every screen, because importing is the only thing a student does
 * daily. Tauri's drag-drop gives us a file *path*, which is handed straight to
 * the Rust side — the file contents never pass through the webview.
 */

import { useCallback, useEffect, useState } from "react";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { open } from "@tauri-apps/plugin-dialog";

interface Props {
  onFile: (path: string) => void;
  busy?: boolean;
  title?: string;
  subtitle?: string;
  compact?: boolean;
}

const SPREADSHEET = ["xlsx", "xls", "xlsm", "ods", "csv"];

function isSpreadsheet(path: string): boolean {
  const ext = path.split(".").pop()?.toLowerCase() ?? "";
  return SPREADSHEET.includes(ext);
}

function FileIcon() {
  return (
    <svg
      width="30"
      height="30"
      viewBox="0 0 24 24"
      fill="none"
      aria-hidden="true"
    >
      <path
        d="M13 2.5H7a2 2 0 0 0-2 2v15a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8.5L13 2.5z"
        stroke="currentColor"
        strokeWidth="1.6"
        strokeLinejoin="round"
      />
      <path
        d="M13 2.5V8.5H19"
        stroke="currentColor"
        strokeWidth="1.6"
        strokeLinejoin="round"
      />
      <path
        d="M9 13h6M9 16.5h4"
        stroke="currentColor"
        strokeWidth="1.6"
        strokeLinecap="round"
      />
    </svg>
  );
}

export function DropZone({ onFile, busy, title, subtitle, compact }: Props) {
  const [over, setOver] = useState(false);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let cancelled = false;

    getCurrentWebview()
      .onDragDropEvent((event) => {
        if (event.payload.type === "over") {
          setOver(true);
        } else if (event.payload.type === "drop") {
          setOver(false);
          const first = event.payload.paths.find(isSpreadsheet);
          if (first) onFile(first);
        } else {
          setOver(false);
        }
      })
      .then((fn) => {
        if (cancelled) fn();
        else unlisten = fn;
      })
      .catch(() => {
        // Drag-drop is unavailable outside the Tauri shell (e.g. in tests).
        // The browse button still works, so this is not worth surfacing.
      });

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [onFile]);

  const browse = useCallback(async () => {
    try {
      const picked = await open({
        multiple: false,
        filters: [{ name: "Spreadsheet", extensions: SPREADSHEET }],
      });
      if (typeof picked === "string") onFile(picked);
    } catch {
      /* the dialog was dismissed */
    }
  }, [onFile]);

  return (
    <button
      type="button"
      className={`dropzone${over ? " over" : ""}${busy ? " busy" : ""}`}
      style={compact ? { padding: "20px 18px" } : undefined}
      onClick={busy ? undefined : browse}
      disabled={busy}
    >
      {busy ? (
        <>
          <span className="spinner" />
          <p className="dropzone-title">Reading the shortlist…</p>
          <p className="dropzone-sub">This stays on your machine.</p>
        </>
      ) : (
        <>
          <span className="dropzone-icon">
            <FileIcon />
          </span>
          <p className="dropzone-title">
            {title ?? (over ? "Drop it" : "Drop a shortlist here")}
          </p>
          <p className="dropzone-sub">
            {subtitle ?? "or click to browse — .xlsx, .xls, .csv"}
          </p>
        </>
      )}
    </button>
  );
}
