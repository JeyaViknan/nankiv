/**
 * Settings.
 *
 * Reached with Cmd+, (Ctrl+, on Windows) rather than from the sidebar, because
 * it is not a place anyone spends time — it is configured once and then left
 * alone. Keeping it out of the navigation leaves the sidebar as a list of
 * things you actually do.
 */

import { useCallback, useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { api, toApiError } from "../lib/api";
import { cgpaUpdatedLabel, readCgpa } from "../lib/cgpa";
import { useStore } from "../lib/store";
import { getThemeChoice, setThemeChoice, type ThemeChoice } from "../lib/theme";
import { Sheet } from "../components/Sheet";
import { UpdateCard } from "../components/UpdateCard";

const THEMES: { id: ThemeChoice; label: string }[] = [
  { id: "system", label: "System" },
  { id: "light", label: "Light" },
  { id: "dark", label: "Dark" },
];

export function SettingsSheet({
  onClose,
  checkForUpdates = false,
}: {
  onClose: () => void;
  /** Opened from Check for Updates: go straight to the check. */
  checkForUpdates?: boolean;
}) {
  const {
    profile,
    saveProfile,
    stats,
    refreshStats,
    showToast,
    error,
    clearError,
  } = useStore();

  const [neoId, setNeoId] = useState(profile?.neo_id ?? "");
  const [regNo, setRegNo] = useState(profile?.reg_no ?? "");
  const [cgpa, setCgpa] = useState(profile?.cgpa?.toString() ?? "");
  const [cgpaError, setCgpaError] = useState(false);
  const [theme, setTheme] = useState<ThemeChoice>(getThemeChoice);
  const [inventory, setInventory] = useState<[string, number][]>([]);
  const [confirmWipe, setConfirmWipe] = useState(false);
  const [watching, setWatching] = useState<boolean | null>(null);
  const [watchError, setWatchError] = useState<string | null>(null);

  useEffect(() => {
    api
      .watchDownloads()
      .then(setWatching)
      .catch(() => setWatching(false));
  }, []);

  async function toggleWatching(next: boolean) {
    setWatchError(null);
    try {
      await api.setWatchDownloads(next);
      setWatching(next);
    } catch (e) {
      setWatchError(toApiError(e).message);
      setWatching(false);
    }
  }

  useEffect(() => {
    api
      .dataInventory()
      .then(setInventory)
      .catch(() => setInventory([]));
  }, [stats]);

  const pickTheme = useCallback((choice: ThemeChoice) => {
    setTheme(choice);
    setThemeChoice(choice);
  }, []);

  async function saveIdentity() {
    clearError();
    const typed = readCgpa(cgpa);
    setCgpaError(typed.kind === "invalid");
    if (typed.kind === "invalid") return;
    const ok = await saveProfile({
      neo_id: neoId.trim() || null,
      reg_no: regNo.trim() || null,
      display_name: profile?.display_name ?? null,
      cohort: profile?.cohort ?? null,
      cgpa: typed.kind === "valid" ? typed.value : null,
      cgpa_updated_at: profile?.cgpa_updated_at ?? null,
    });
    if (ok) showToast("Saved");
  }

  const updated = cgpaUpdatedLabel(profile?.cgpa_updated_at ?? null);

  async function importReference() {
    const picked = await open({
      multiple: false,
      filters: [{ name: "Spreadsheet", extensions: ["xlsx", "xls", "csv"] }],
    });
    if (typeof picked !== "string") return;
    try {
      const r = await api.importReference(picked);
      await refreshStats();
      showToast(
        r.kind === "academic"
          ? `Added CGPA for ${r.academics_learned.toLocaleString()} students`
          : `Learned ${r.verified_links.toLocaleString()} identity links`,
      );
    } catch (e) {
      showToast(toApiError(e).message);
    }
  }

  async function wipe() {
    await api.wipeAllData();
    await refreshStats();
    setConfirmWipe(false);
    window.location.reload();
  }

  return (
    <Sheet title="Settings" onClose={onClose}>
      <div className="card">
        <h2>Appearance</h2>
        <div className="theme-picker">
          {THEMES.map((t) => (
            <button
              key={t.id}
              className="theme-option"
              aria-pressed={theme === t.id}
              onClick={() => pickTheme(t.id)}
            >
              <span className="theme-swatch">
                {t.id === "system" ? (
                  <>
                    <span className="half sw-light">
                      <span className="bar a" />
                      <span className="bar b" />
                    </span>
                    <span className="half sw-dark">
                      <span className="bar a" />
                      <span className="bar b" />
                    </span>
                  </>
                ) : (
                  <span
                    className={`half ${t.id === "dark" ? "sw-dark" : "sw-light"}`}
                    style={{ flex: 1 }}
                  >
                    <span className="bar a" />
                    <span className="bar b" />
                  </span>
                )}
              </span>
              {t.label}
            </button>
          ))}
        </div>
      </div>

      <div className="card">
        <h2>Your identity</h2>
        <div className="field">
          <label htmlFor="set-neo">Neo ID</label>
          <input
            id="set-neo"
            type="text"
            className="mono-input"
            maxLength={8}
            placeholder="V9H0G6C4"
            value={neoId}
            onChange={(e) => setNeoId(e.target.value.toUpperCase())}
          />
        </div>
        <div className="field">
          <label htmlFor="set-reg">Registration number</label>
          <input
            id="set-reg"
            type="text"
            className="mono-input"
            maxLength={11}
            placeholder="23BAI0002"
            value={regNo}
            onChange={(e) => setRegNo(e.target.value.toUpperCase())}
          />
          <span className="hint">
            Some companies list students by this instead.
          </span>
        </div>
        <div className="field">
          <label htmlFor="set-cgpa">
            Your CGPA
            {updated && <span className="label-aside">· {updated}</span>}
          </label>
          <input
            id="set-cgpa"
            type="text"
            inputMode="decimal"
            className="mono-input narrow"
            maxLength={5}
            placeholder="8.42"
            value={cgpa}
            aria-invalid={cgpaError}
            aria-describedby="set-cgpa-hint"
            onChange={(e) => {
              setCgpa(e.target.value);
              setCgpaError(false);
            }}
            onKeyDown={(e) => e.key === "Enter" && void saveIdentity()}
          />
          <span className="hint" id="set-cgpa-hint">
            {cgpaError
              ? "A CGPA is out of 10 — like 8.42."
              : "Only you see this. Update it each semester."}
          </span>
        </div>
        {error && <p className="field-error">{error.message}</p>}
        <button className="btn primary" onClick={saveIdentity}>
          Save
        </button>
      </div>

      <div className="card">
        <h2>Privacy</h2>
        <p className="card-text">
          The only CGPA shown is yours. Others' are used for estimates, never
          displayed.
        </p>
        <div className="notice" style={{ marginTop: 12, marginBottom: 0 }}>
          No account, no server, no telemetry. Contact details in sheets are
          never stored.
        </div>
      </div>

      <div className="card">
        <h2>Downloads</h2>
        <label className="switch">
          <input
            type="checkbox"
            checked={watching ?? false}
            disabled={watching === null}
            onChange={(e) => void toggleWatching(e.target.checked)}
          />
          <span className="switch-text">
            <strong>Check new shortlists in Downloads</strong>
            <span>
              New ones are checked as they arrive, while nankiv is open.
            </span>
          </span>
        </label>
        {watchError && (
          <p className="field-error" role="alert">
            {watchError}
          </p>
        )}
      </div>

      <div className="card">
        <div className="card-head">
          <h2>Reference data</h2>
          <button className="btn small" onClick={importReference}>
            Import a sheet
          </button>
        </div>
        {stats && (
          <div className="stats">
            <div className="stat">
              <div className="stat-value">
                {stats.students_known.toLocaleString()}
              </div>
              <div className="stat-label">Students known</div>
            </div>
            <div className="stat">
              <div className="stat-value">
                {stats.names_known.toLocaleString()}
              </div>
              <div className="stat-label">Names resolved</div>
            </div>
            <div className="stat">
              <div className="stat-value">
                {stats.academics_known.toLocaleString()}
              </div>
              <div className="stat-label">With CGPA</div>
            </div>
            <div className="stat">
              <div className="stat-value">
                {stats.conflicts.toLocaleString()}
              </div>
              <div className="stat-label">Conflicts dropped</div>
            </div>
          </div>
        )}
      </div>

      <div className="card">
        <h2>What's stored</h2>
        <div className="table-wrap">
          <table>
            <tbody>
              {inventory.map(([name, count]) => (
                <tr key={name}>
                  <td>{name.replace(/_/g, " ")}</td>
                  <td className="num">{count.toLocaleString()}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
        <div style={{ marginTop: 15 }}>
          {confirmWipe ? (
            <div className="notice danger" style={{ marginBottom: 0 }}>
              <p style={{ margin: "0 0 11px" }}>
                Deletes every drive, person and record. This can't be undone.
              </p>
              <div className="btn-row">
                <button className="btn danger" onClick={wipe}>
                  Delete everything
                </button>
                <button className="btn" onClick={() => setConfirmWipe(false)}>
                  Cancel
                </button>
              </div>
            </div>
          ) : (
            <button className="btn danger" onClick={() => setConfirmWipe(true)}>
              Delete all local data
            </button>
          )}
        </div>
      </div>

      <UpdateCard autoCheck={checkForUpdates} />
    </Sheet>
  );
}
