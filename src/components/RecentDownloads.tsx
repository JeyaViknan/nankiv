/**
 * "Import a recent download" — the file you just saved, without hunting for it.
 *
 * Shortlists are downloaded from an email or the placement portal and then
 * have to be found again. This lists the newest spreadsheets in Downloads,
 * marks any already imported, and imports one in a click.
 *
 * Downloads is read only when asked: on a Mac the first look prompts for
 * access, and that prompt should follow a click, never appear at launch.
 */

import { useState } from "react";
import { api, toApiError, type RecentDownload } from "../lib/api";
import { useStore } from "../lib/store";
import { Icon } from "./Icon";

/** "Just now", "2 hours ago", "Yesterday", "3 days ago". */
export function downloadedWhen(iso: string, now = new Date()): string {
  const then = new Date(iso);
  const minutes = Math.round((now.getTime() - then.getTime()) / 60_000);
  if (Number.isNaN(minutes)) return "";
  if (minutes < 2) return "Just now";
  if (minutes < 60) return `${minutes} minutes ago`;
  const hours = Math.round(minutes / 60);
  if (hours < 24 && then.getDate() === now.getDate()) {
    return hours === 1 ? "An hour ago" : `${hours} hours ago`;
  }
  const days = Math.round(
    (new Date(now.toDateString()).getTime() -
      new Date(then.toDateString()).getTime()) /
      86_400_000,
  );
  return days <= 1 ? "Yesterday" : `${days} days ago`;
}

export function RecentDownloads({ disabled }: { disabled: boolean }) {
  const { importFile } = useStore();
  const [open, setOpen] = useState(false);
  const [files, setFiles] = useState<RecentDownload[] | null>(null);
  const [problem, setProblem] = useState<string | null>(null);

  async function toggle() {
    if (open) {
      setOpen(false);
      return;
    }
    setOpen(true);
    setFiles(null);
    setProblem(null);
    try {
      setFiles(await api.recentDownloads());
    } catch (e) {
      setProblem(toApiError(e).message);
    }
  }

  return (
    <div className="recent">
      <button
        className="link-button"
        onClick={() => void toggle()}
        aria-expanded={open}
        aria-controls="recent-downloads"
      >
        Import a recent download
        <Icon name="chevronRight" size={11} className="recent-chevron" />
      </button>
      {open && (
        <div id="recent-downloads">
          {problem ? (
            <p className="hint-line" role="alert">
              {problem}
            </p>
          ) : !files ? (
            <p className="hint-line">Looking in Downloads…</p>
          ) : files.length === 0 ? (
            <p className="hint-line">
              No spreadsheets downloaded in the last two weeks.
            </p>
          ) : (
            <ul className="recent-list" aria-label="Recent downloads">
              {files.map((f) => (
                <li key={f.path}>
                  <button
                    className="recent-file"
                    disabled={disabled}
                    onClick={() => {
                      setOpen(false);
                      void importFile(f.path);
                    }}
                  >
                    <Icon name="sheet" size={16} />
                    <span className="recent-name">{f.name}</span>
                    <span className="recent-when">
                      {f.imported && "Imported · "}
                      {downloadedWhen(f.modified)}
                    </span>
                  </button>
                </li>
              ))}
            </ul>
          )}
        </div>
      )}
    </div>
  );
}
