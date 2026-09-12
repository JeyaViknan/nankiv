/**
 * Shortlists — the root surface.
 *
 * Home and History were never two screens. They were one thing split in half:
 * a place where shortlists arrive, and a record of the ones that already did.
 * Merging them means the primary action and its consequences share a surface,
 * and the daily loop needs no navigation at all.
 *
 * The timeline is a real list, not a grid of cards. Rows are scannable, support
 * arrow-key navigation, and carry their actions on hover rather than
 * permanently — the previous build put three buttons on every row, so control
 * density grew with the length of the season.
 */

import { useCallback, useEffect, useRef, useState } from "react";
import type { DriveRecord } from "../lib/api";
import { useStore } from "../lib/store";
import { ImportStatus } from "../components/ImportStatus";
import { NeedsReference } from "../components/NeedsReference";

function relativeDay(iso: string): string {
  const d = new Date(iso.replace(" ", "T") + "Z");
  if (Number.isNaN(d.getTime())) return iso;
  const now = new Date();
  const days = Math.floor(
    (new Date(now.getFullYear(), now.getMonth(), now.getDate()).getTime() -
      new Date(d.getFullYear(), d.getMonth(), d.getDate()).getTime()) /
      86_400_000,
  );
  if (days === 0) return "Today";
  if (days === 1) return "Yesterday";
  if (days < 7) return `${days} days ago`;
  return d.toLocaleDateString(undefined, { day: "numeric", month: "short" });
}

function TrayIcon() {
  return (
    <svg
      width="34"
      height="34"
      viewBox="0 0 40 40"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.5"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <path d="M20 7v17M13.5 17.5L20 24l6.5-6.5" />
      <path d="M7 26v4.5A2.5 2.5 0 0 0 9.5 33h21a2.5 2.5 0 0 0 2.5-2.5V26" />
    </svg>
  );
}

function DeleteIcon() {
  return (
    <svg
      width="14"
      height="14"
      viewBox="0 0 16 16"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.5"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <path d="M2.5 4h11M6 4V2.5h4V4M4 4l.6 9a1 1 0 0 0 1 1h4.8a1 1 0 0 0 1-1L12 4" />
    </svg>
  );
}

function DriveRow({
  drive,
  active,
  onOpen,
  onDelete,
}: {
  drive: DriveRecord;
  active: boolean;
  onOpen: () => void;
  onDelete: () => void;
}) {
  return (
    <div className={`row${active ? " active" : ""}`}>
      <button className="row-main" onClick={onOpen} tabIndex={-1}>
        <span className="row-title">{drive.company}</span>
        <span className="row-meta">
          {drive.total_students.toLocaleString()} shortlisted
          <span className="row-dot">·</span>
          {relativeDay(drive.imported_at)}
        </span>
      </button>
      <button
        className="row-action"
        aria-label={`Remove ${drive.company}`}
        title="Remove"
        tabIndex={-1}
        onClick={(e) => {
          e.stopPropagation();
          onDelete();
        }}
      >
        <DeleteIcon />
      </button>
    </div>
  );
}

export function Shortlists({ onBrowse }: { onBrowse: () => void }) {
  const { drives, openDrive, deleteDrive, importStage, profile, stats } =
    useStore();
  // -1 until the keyboard is actually used: a highlight nobody asked for
  // reads as a selection and is just noise.
  const [cursor, setCursor] = useState(-1);
  const listRef = useRef<HTMLDivElement>(null);

  const busy = importStage.phase === "reading";

  // Arrow keys move through the timeline; Enter opens. A desktop list that
  // cannot be driven from the keyboard is a web page in a window.
  const onKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (drives.length === 0) return;
      if (e.key === "ArrowDown") {
        e.preventDefault();
        setCursor((c) => Math.min(c + 1, drives.length - 1));
      } else if (e.key === "ArrowUp") {
        e.preventDefault();
        setCursor((c) => Math.max(c - 1, 0));
      } else if (e.key === "Enter" && cursor >= 0) {
        e.preventDefault();
        const d = drives[cursor];
        if (d) void openDrive(d.id);
      }
    },
    [drives, cursor, openDrive],
  );

  useEffect(() => {
    if (cursor > drives.length - 1) setCursor(drives.length - 1);
  }, [drives.length, cursor]);

  const empty = drives.length === 0;

  return (
    <div className="surface">
      {/* The invitation. Sized to the moment: large when there is nothing else
          on the surface, quiet once the season is under way. */}
      <button
        className={`invite${empty ? " large" : ""}`}
        onClick={onBrowse}
        disabled={busy}
      >
        <span className="invite-icon">
          <TrayIcon />
        </span>
        <span className="invite-text">
          <span className="invite-title">
            {empty ? "Drop a shortlist to begin" : "Drop a shortlist"}
          </span>
          <span className="invite-sub">
            Anywhere in this window — or press <kbd>⌘</kbd>
            <kbd>O</kbd> to choose a file
          </span>
        </span>
      </button>

      <ImportStatus />

      {/* Removing the reference step from onboarding left this feature with no
          way to be found. It belongs where the gap is felt, not in Settings. */}
      {stats?.academics_known === 0 && !empty && <NeedsReference compact />}

      {profile && !profile.reg_no && !empty && (
        <p className="hint-line">
          Some companies key their shortlists by registration number. Add yours
          in Settings so those files can be answered too.
        </p>
      )}

      {!empty && (
        <>
          <div className="list-head">
            <span className="eyebrow">This season</span>
            <span className="list-count">
              {drives.length} {drives.length === 1 ? "drive" : "drives"}
            </span>
          </div>
          <div
            className="list"
            role="listbox"
            aria-label="Imported shortlists"
            tabIndex={0}
            ref={listRef}
            onKeyDown={onKeyDown}
            onBlur={() => setCursor(-1)}
          >
            {drives.map((d, i) => (
              <DriveRow
                key={d.id}
                drive={d}
                active={i === cursor}
                onOpen={() => {
                  setCursor(i);
                  void openDrive(d.id);
                }}
                onDelete={() => void deleteDrive(d.id, d.company)}
              />
            ))}
          </div>
        </>
      )}
    </div>
  );
}
