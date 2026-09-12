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
import { useStore } from "../lib/store";
import { getThemeChoice, setThemeChoice, type ThemeChoice } from "../lib/theme";
import { Sheet } from "../components/Sheet";

const THEMES: { id: ThemeChoice; label: string }[] = [
  { id: "system", label: "System" },
  { id: "light", label: "Light" },
  { id: "dark", label: "Dark" },
];

export function SettingsSheet({ onClose }: { onClose: () => void }) {
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
  const [showCgpa, setShowCgpa] = useState(profile?.show_friend_cgpa ?? false);
  const [theme, setTheme] = useState<ThemeChoice>(getThemeChoice);
  const [inventory, setInventory] = useState<[string, number][]>([]);
  const [confirmWipe, setConfirmWipe] = useState(false);

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
    const ok = await saveProfile({
      neo_id: neoId.trim() || null,
      reg_no: regNo.trim() || null,
      display_name: profile?.display_name ?? null,
      cohort: profile?.cohort ?? null,
      show_friend_cgpa: showCgpa,
    });
    if (ok) showToast("Saved");
  }

  async function toggleCgpa(next: boolean) {
    setShowCgpa(next);
    await saveProfile({
      neo_id: profile?.neo_id ?? null,
      reg_no: profile?.reg_no ?? null,
      display_name: profile?.display_name ?? null,
      cohort: profile?.cohort ?? null,
      show_friend_cgpa: next,
    });
  }

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
          ? `Learned records for ${r.academics_learned.toLocaleString()} students`
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
        <p
          style={{
            fontSize: 12.5,
            color: "var(--text-3)",
            margin: "0 0 14px",
          }}
        >
          nankiv follows your Mac's appearance by default, and switches with it.
        </p>
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
            Some companies key their shortlists by this instead. Without it,
            those files can't be answered for you.
          </span>
        </div>
        {error && <p className="field-error">{error.message}</p>}
        <button className="btn primary" onClick={saveIdentity}>
          Save
        </button>
      </div>

      <div className="card">
        <h2>Privacy</h2>
        <label className="switch">
          <input
            type="checkbox"
            checked={showCgpa}
            onChange={(e) => void toggleCgpa(e.target.checked)}
          />
          <span className="switch-text">
            <strong>Show CGPA for people in my circle</strong>
            <span>
              Off by default. Cutoff analysis works without this — it only needs
              aggregates.
            </span>
          </span>
        </label>
        <div className="notice" style={{ marginTop: 8, marginBottom: 0 }}>
          No account, no server, no telemetry. Contact details in reference
          sheets — phone, email, date of birth, resume links — are discarded as
          the file is read and have nowhere to be stored.
        </div>
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
                This permanently deletes every drive, friend, identity link and
                academic record on this machine. It cannot be undone.
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

      <p
        style={{
          fontSize: 11.5,
          color: "var(--text-faint)",
          textAlign: "center",
          margin: "18px 0 0",
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          gap: 7,
        }}
      >
        nankiv 0.1.0 — works offline
      </p>
    </Sheet>
  );
}
