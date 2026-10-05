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
import { Icon } from "./Icon";
import { open } from "@tauri-apps/plugin-dialog";
import { toApiError } from "../lib/api";
import { useStore } from "../lib/store";

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
          <strong>Analysis isn't set up.</strong> Add your batch's CGPA sheet to
          estimate cutoffs.
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
        <Icon name="sheet" size={21} />
      </span>
      <div className="setup-body">
        <h3>Analysis isn't set up yet</h3>
        <p>
          The results above are exact. Add your batch's CGPA sheet to estimate
          cutoffs.
        </p>
        <p className="setup-sub">
          Only registration number, name, CGPA and branch are read. Contact
          details are discarded.
        </p>
        {error && <p className="field-error">{error}</p>}
        <button className="btn primary" onClick={choose} disabled={busy}>
          {busy ? "Reading…" : "Add a reference sheet"}
        </button>
      </div>
    </div>
  );
}
