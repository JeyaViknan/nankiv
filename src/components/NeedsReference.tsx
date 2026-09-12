/**
 * The difference between "not set up" and "not enough data".
 *
 * These were being shown as the same thing, and they are not remotely the same
 * thing. "We matched 0 of 1815 students (0%)" describes a *sampling* problem —
 * as if the file were thin, or the cohort unlucky. The actual situation on a
 * fresh install is that nankiv holds no academic records at all, which is not a
 * failure of the shortlist and not something a bigger sample would fix.
 *
 * Conflating them is the one kind of dishonesty the rest of this app is built
 * to avoid, so the empty case gets its own surface, its own words, and the
 * action that resolves it.
 */

import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { toApiError } from "../lib/api";
import { useStore } from "../lib/store";

function SheetIcon() {
  return (
    <svg
      width="22"
      height="22"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.5"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <rect x="3.5" y="3" width="17" height="18" rx="2.5" />
      <path d="M3.5 9h17M9 9v12M3.5 15h17" />
    </svg>
  );
}

export function NeedsReference({ compact }: { compact?: boolean }) {
  const { importReferenceFile } = useStore();
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function choose() {
    setError(null);
    try {
      const picked = await open({
        multiple: false,
        filters: [
          {
            name: "Spreadsheet",
            extensions: ["xlsx", "xls", "xlsm", "ods", "csv"],
          },
        ],
      });
      if (typeof picked !== "string") return;
      setBusy(true);
      await importReferenceFile(picked);
    } catch (e) {
      setError(toApiError(e).message);
    } finally {
      setBusy(false);
    }
  }

  if (compact) {
    return (
      <div className="notice accent setup-line">
        <span>
          <strong>Analysis isn't set up yet.</strong> Add the CGPA sheet your
          batch circulated and nankiv can estimate cutoffs.
        </span>
        <button className="btn small" onClick={choose} disabled={busy}>
          {busy ? "Reading…" : "Add sheet"}
        </button>
      </div>
    );
  }

  return (
    <div className="setup-card">
      <span className="setup-icon">
        <SheetIcon />
      </span>
      <div className="setup-body">
        <h3>Analysis isn't set up yet</h3>
        <p>
          Membership above is exact — it comes straight from the file. But
          nankiv doesn't hold any academic records yet, so there's nothing to
          compare this shortlist against.
        </p>
        <p className="setup-sub">
          Add the CGPA sheet your batch already circulated. Only registration
          number, name, CGPA and branch are read — phone numbers, emails and
          resume links are discarded as the file is read.
        </p>
        {error && <p className="field-error">{error}</p>}
        <button className="btn primary" onClick={choose} disabled={busy}>
          {busy ? "Reading…" : "Add a reference sheet"}
        </button>
      </div>
    </div>
  );
}
