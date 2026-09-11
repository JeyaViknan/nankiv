/**
 * The remaining screens: home, onboarding, circle, search, history, settings.
 *
 * Grouped in one module because each is small and they share helpers; the drive
 * detail screen is separate because it carries the weight of the product.
 */

import { useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import {
  api,
  toApiError,
  type DriveRecord,
  type PersonResult,
  type SearchResult,
} from "../lib/api";
import { useStore } from "../lib/store";
import { DropZone } from "../components/DropZone";
import { VerdictPill } from "../components/Verdict";

// --- helpers -----------------------------------------------------------------

function formatDate(iso: string): string {
  const d = new Date(iso.replace(" ", "T") + "Z");
  if (Number.isNaN(d.getTime())) return iso;
  return d.toLocaleDateString(undefined, {
    day: "numeric",
    month: "short",
    year: "numeric",
  });
}

// --- Home --------------------------------------------------------------------

export function HomeScreen() {
  const { drives, importFile, importing, openDrive, profile } = useStore();
  const recent = drives.slice(0, 8);

  return (
    <div className="main-narrow">
      <h1>Drop today's shortlist</h1>
      <p className="page-sub">
        Everything happens on this machine. Nothing is uploaded, and nankiv
        works with no internet connection.
      </p>

      <DropZone onFile={(p) => void importFile(p)} busy={importing} />

      {profile && !profile.reg_no && (
        <div className="notice" style={{ marginTop: 16 }}>
          Some companies send shortlists keyed by{" "}
          <strong>registration number</strong> instead of Neo ID. Add yours in
          Settings so those can be checked too.
        </div>
      )}

      <div style={{ marginTop: 26 }}>
        <h2>Recent drives</h2>
        {recent.length === 0 ? (
          <div className="card">
            <div className="empty">
              <p className="empty-title">Nothing imported yet</p>
              <p>Drop a shortlist above and it'll be saved here.</p>
            </div>
          </div>
        ) : (
          recent.map((d) => (
            <DriveRow key={d.id} drive={d} onOpen={openDrive} />
          ))
        )}
      </div>
    </div>
  );
}

function DriveRow({
  drive,
  onOpen,
}: {
  drive: DriveRecord;
  onOpen: (id: number) => void;
}) {
  return (
    <button className="drive-row" onClick={() => onOpen(drive.id)}>
      <div style={{ minWidth: 0, flex: 1 }}>
        <div className="drive-company">{drive.company}</div>
        <div className="drive-meta">
          {drive.total_students.toLocaleString()} shortlisted ·{" "}
          {formatDate(drive.imported_at)}
          {drive.drive_date ? ` · ${drive.drive_date}` : ""}
        </div>
      </div>
      <span className="pill quiet">
        {drive.primary_key === "reg_no" ? "Reg no" : "Neo ID"}
      </span>
    </button>
  );
}

// --- Onboarding --------------------------------------------------------------

export function OnboardingScreen() {
  const { saveProfile, error, clearError, go, refreshStats, showToast } =
    useStore();
  const [step, setStep] = useState(0);
  const [neoId, setNeoId] = useState("");
  const [regNo, setRegNo] = useState("");
  const [importingRef, setImportingRef] = useState(false);

  async function saveIdentity() {
    clearError();
    const ok = await saveProfile({
      neo_id: neoId.trim() || null,
      reg_no: regNo.trim() || null,
      display_name: null,
      cohort: null,
      show_friend_cgpa: false,
    });
    if (ok) setStep(1);
  }

  async function importReference() {
    const picked = await open({
      multiple: false,
      filters: [{ name: "Spreadsheet", extensions: ["xlsx", "xls", "csv"] }],
    });
    if (typeof picked !== "string") return;
    setImportingRef(true);
    try {
      const r = await api.importReference(picked);
      await refreshStats();
      showToast(
        r.kind === "academic"
          ? `Learned academic records for ${r.academics_learned.toLocaleString()} students`
          : `Learned ${r.verified_links.toLocaleString()} identity links`,
      );
    } catch (e) {
      useStore.setState({ error: toApiError(e) });
    } finally {
      setImportingRef(false);
    }
  }

  return (
    <div className="onboard">
      <div className="step-dots">
        <span className={`step-dot${step >= 0 ? " done" : ""}`} />
        <span className={`step-dot${step >= 1 ? " done" : ""}`} />
      </div>

      {step === 0 ? (
        <>
          <h1>Who are you?</h1>
          <p className="page-sub">
            Saved once, on this machine only. It's what lets nankiv answer
            instantly instead of asking you to search.
          </p>

          <div className="field">
            <label htmlFor="ob-neo">Your Neo ID</label>
            <input
              id="ob-neo"
              type="text"
              className="mono-input"
              placeholder="V9H0G6C4"
              maxLength={8}
              value={neoId}
              onChange={(e) => setNeoId(e.target.value.toUpperCase())}
            />
            <span className="hint">
              Eight characters, alternating letter and digit.
            </span>
          </div>

          <div className="field">
            <label htmlFor="ob-reg">Your registration number (optional)</label>
            <input
              id="ob-reg"
              type="text"
              className="mono-input"
              placeholder="23BAI0002"
              maxLength={11}
              value={regNo}
              onChange={(e) => setRegNo(e.target.value.toUpperCase())}
            />
            <span className="hint">
              Some companies key their shortlists by this instead. Adding it
              means those files can be checked too.
            </span>
          </div>

          {error && <p className="field-error">{error.message}</p>}

          <div className="btn-row" style={{ marginTop: 18 }}>
            <button
              className="btn primary"
              onClick={saveIdentity}
              disabled={!neoId.trim() && !regNo.trim()}
            >
              Continue
            </button>
          </div>
        </>
      ) : (
        <>
          <h1>Add reference data</h1>
          <p className="page-sub">
            Optional, and you can do it later. nankiv ships with no student data
            in it — drop in the roster or CGPA sheets you already received and
            it can show names and analyse cutoffs.
          </p>

          <div className="notice accent">
            Only registration number, name, CGPA and branch are read. Phone
            numbers, email addresses, dates of birth and resume links are
            ignored at the point of reading — they never reach storage.
          </div>

          <div className="btn-row" style={{ marginTop: 18 }}>
            <button
              className="btn"
              onClick={importReference}
              disabled={importingRef}
            >
              {importingRef ? "Reading…" : "Choose a reference sheet"}
            </button>
            <button className="btn primary" onClick={() => go("home")}>
              Done
            </button>
          </div>
          <p style={{ fontSize: 12, color: "var(--muted)", marginTop: 16 }}>
            Checking whether you're shortlisted works without any of this — it
            reads the file directly.
          </p>
        </>
      )}
    </div>
  );
}

// --- Circle ------------------------------------------------------------------

export function CircleScreen() {
  const { friends, refreshFriends, showToast } = useStore();
  const [label, setLabel] = useState("");
  const [neoId, setNeoId] = useState("");
  const [err, setErr] = useState<string | null>(null);

  async function add() {
    setErr(null);
    try {
      await api.addFriend(
        label.trim() || neoId.trim(),
        neoId.trim() || null,
        null,
        null,
      );
      setLabel("");
      setNeoId("");
      await refreshFriends();
      showToast("Added to your circle");
    } catch (e) {
      setErr(toApiError(e).message);
    }
  }

  async function remove(id: number) {
    await api.removeFriend(id);
    await refreshFriends();
  }

  return (
    <div className="main-narrow">
      <h1>Your circle</h1>
      <p className="page-sub">
        Added once, checked on every shortlist from then on. Nobody needs to
        remember anyone's Neo ID again.
      </p>

      <div className="card">
        <h2>Add someone</h2>
        <div style={{ display: "flex", gap: 10, alignItems: "flex-start" }}>
          <div className="field" style={{ flex: 1, marginBottom: 0 }}>
            <label htmlFor="fr-name">Name</label>
            <input
              id="fr-name"
              type="text"
              placeholder="Arjun"
              value={label}
              onChange={(e) => setLabel(e.target.value)}
            />
          </div>
          <div className="field" style={{ flex: 1, marginBottom: 0 }}>
            <label htmlFor="fr-neo">Neo ID</label>
            <input
              id="fr-neo"
              type="text"
              className="mono-input"
              placeholder="V9H0G6C4"
              maxLength={8}
              value={neoId}
              onChange={(e) => setNeoId(e.target.value.toUpperCase())}
            />
          </div>
          <button
            className="btn primary"
            style={{ marginTop: 21 }}
            onClick={add}
            disabled={!neoId.trim()}
          >
            Add
          </button>
        </div>
        {err && (
          <p className="field-error" style={{ marginTop: 8 }}>
            {err}
          </p>
        )}
      </div>

      <div className="card">
        <div className="card-head">
          <h2>Tracked</h2>
          <span style={{ fontSize: 12, color: "var(--muted)" }}>
            {friends.length} {friends.length === 1 ? "person" : "people"}
          </span>
        </div>
        {friends.length === 0 ? (
          <div className="empty">
            <p className="empty-title">Nobody yet</p>
            <p>Add a friend above to check them automatically.</p>
          </div>
        ) : (
          <div className="person-list">
            {friends.map((f) => (
              <div className="person" key={f.id}>
                <div>
                  <span className="person-name">{f.label}</span>
                  <span className="person-id">{f.neo_id ?? f.reg_no}</span>
                </div>
                <div className="person-status">
                  <button
                    className="btn small"
                    onClick={() => void remove(f.id)}
                  >
                    Remove
                  </button>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

// --- Search ------------------------------------------------------------------

export function SearchScreen() {
  const [query, setQuery] = useState("");
  const [byId, setById] = useState<PersonResult[] | null>(null);
  const [byName, setByName] = useState<SearchResult | null>(null);
  const [err, setErr] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  async function run() {
    setErr(null);
    setById(null);
    setByName(null);
    setBusy(true);
    const q = query.trim();
    try {
      // An identifier-shaped query is a membership lookup; anything else is a
      // name search against the identity graph.
      if (
        /^(?:[A-Za-z][0-9]){4}$/.test(q) ||
        /^\d{2}[A-Za-z]{3}\d{4,5}$/.test(q)
      ) {
        setById(await api.lookupIdentifier(q));
      } else if (q.length >= 2) {
        setByName(await api.searchStudents(q));
      }
    } catch (e) {
      setErr(toApiError(e).message);
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="main-narrow">
      <h1>Look someone up</h1>
      <p className="page-sub">
        Paste a Neo ID to see which of your imported drives they're on, or type
        a name to find their Neo ID.
      </p>

      <div className="card">
        <div style={{ display: "flex", gap: 10 }}>
          <input
            type="text"
            placeholder="V9H0G6C4, 23BAI0002, or a name"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && void run()}
          />
          <button
            className="btn primary"
            onClick={run}
            disabled={busy || !query.trim()}
          >
            {busy ? "…" : "Search"}
          </button>
        </div>
        {err && (
          <p className="field-error" style={{ marginTop: 8 }}>
            {err}
          </p>
        )}
      </div>

      {byId && (
        <div className="card">
          <h2>Drives this person is on</h2>
          {byId.length === 0 ? (
            <div className="notice">
              Not found on any shortlist you've imported. That only covers the
              drives in your history — it isn't a statement about drives you
              haven't imported.
            </div>
          ) : (
            <div className="person-list">
              {byId.map((p, i) => (
                <div className="person" key={i}>
                  <span className="person-name">{p.label}</span>
                  <div className="person-status">
                    <VerdictPill verdict={p.verdict} />
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      )}

      {byName && (
        <div className="card">
          <h2>Name search</h2>
          {byName.kind === "found" && (
            <div className="person-list">
              <div className="person">
                <div>
                  <span className="person-name">{byName.name}</span>
                  <span className="person-id selectable">{byName.neo_id}</span>
                </div>
                <div className="person-status">
                  <span className="pill quiet">{byName.confidence}</span>
                </div>
              </div>
            </div>
          )}
          {byName.kind === "ambiguous" && (
            <>
              <div className="notice warn">
                More than one student matches that name, and nankiv won't guess
                between them. Pick the right Neo ID yourself.
              </div>
              <div className="person-list">
                {byName.candidates.map((c) => (
                  <div className="person" key={c.neo_id}>
                    <div>
                      <span className="person-name">{c.name}</span>
                      <span className="person-id selectable">{c.neo_id}</span>
                    </div>
                  </div>
                ))}
              </div>
            </>
          )}
          {byName.kind === "not_found" && (
            <div className="notice">
              No confident match. nankiv only knows the students it has learned
              from files you've imported — import more reference data to widen
              this.
            </div>
          )}
        </div>
      )}
    </div>
  );
}

// --- History -----------------------------------------------------------------

export function HistoryScreen() {
  const { drives, openDrive, refreshDrives, showToast } = useStore();
  const [fromId, setFromId] = useState<number | null>(null);
  const [diff, setDiff] = useState<Awaited<
    ReturnType<typeof api.compareRounds>
  > | null>(null);

  async function compare(toId: number) {
    if (fromId === null) return;
    try {
      setDiff(await api.compareRounds(fromId, toId));
    } catch (e) {
      showToast(toApiError(e).message);
    }
  }

  async function remove(id: number) {
    await api.deleteDrive(id);
    await refreshDrives();
    showToast("Drive removed");
  }

  return (
    <div className="main-narrow">
      <h1>History</h1>
      <p className="page-sub">
        Every shortlist you've imported. Pick two rounds of the same drive to
        see who advanced — that comparison is exact, whatever the coverage.
      </p>

      {drives.length === 0 ? (
        <div className="card">
          <div className="empty">
            <p className="empty-title">Nothing yet</p>
            <p>Imported shortlists appear here.</p>
          </div>
        </div>
      ) : (
        drives.map((d) => (
          <div
            key={d.id}
            style={{
              display: "flex",
              gap: 8,
              alignItems: "stretch",
              marginBottom: 7,
            }}
          >
            <button
              className="drive-row"
              style={{ marginBottom: 0 }}
              onClick={() => openDrive(d.id)}
            >
              <div style={{ minWidth: 0, flex: 1 }}>
                <div className="drive-company">{d.company}</div>
                <div className="drive-meta">
                  {d.total_students.toLocaleString()} shortlisted ·{" "}
                  {formatDate(d.imported_at)}
                </div>
              </div>
            </button>
            <button
              className="btn small"
              onClick={() =>
                fromId === d.id
                  ? setFromId(null)
                  : fromId === null
                    ? setFromId(d.id)
                    : compare(d.id)
              }
              title="Compare rounds"
            >
              {fromId === d.id
                ? "Selected"
                : fromId === null
                  ? "Compare"
                  : "vs this"}
            </button>
            <button className="btn small" onClick={() => void remove(d.id)}>
              Delete
            </button>
          </div>
        ))
      )}

      {diff && (
        <div className="card" style={{ marginTop: 18 }}>
          <div className="card-head">
            <h2>
              {diff.from_company} → {diff.to_company}
            </h2>
            <button
              className="btn small"
              onClick={() => {
                setDiff(null);
                setFromId(null);
              }}
            >
              Clear
            </button>
          </div>
          <div className="stats">
            <div className="stat">
              <div className="stat-value" style={{ color: "var(--accent)" }}>
                {diff.advanced}
              </div>
              <div className="stat-label">Advanced</div>
            </div>
            <div className="stat">
              <div className="stat-value">{diff.dropped}</div>
              <div className="stat-label">Dropped</div>
            </div>
            <div className="stat">
              <div className="stat-value">{diff.added}</div>
              <div className="stat-label">Newly added</div>
            </div>
          </div>
          {diff.advanced_friends.length > 0 && (
            <p style={{ fontSize: 13, marginTop: 14, marginBottom: 0 }}>
              <strong>Advanced from your circle:</strong>{" "}
              {diff.advanced_friends.join(", ")}
            </p>
          )}
          {diff.dropped_friends.length > 0 && (
            <p
              style={{
                fontSize: 13,
                marginTop: 6,
                marginBottom: 0,
                color: "var(--muted)",
              }}
            >
              Dropped: {diff.dropped_friends.join(", ")}
            </p>
          )}
        </div>
      )}
    </div>
  );
}

// --- Settings ----------------------------------------------------------------

export function SettingsScreen() {
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
  const [inventory, setInventory] = useState<[string, number][]>([]);
  const [confirmWipe, setConfirmWipe] = useState(false);

  useEffect(() => {
    api
      .dataInventory()
      .then(setInventory)
      .catch(() => setInventory([]));
  }, [stats]);

  async function save() {
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
    showToast("All local data deleted");
    window.location.reload();
  }

  return (
    <div className="main-narrow">
      <h1>Settings</h1>
      <p className="page-sub">
        Everything here is stored on this machine only.
      </p>

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
            Lets nankiv answer for shortlists keyed by registration number.
          </span>
        </div>
        {error && <p className="field-error">{error.message}</p>}
        <button className="btn primary" onClick={save}>
          Save
        </button>
      </div>

      <div className="card">
        <h2>Privacy</h2>
        <label className="switch">
          <input
            type="checkbox"
            checked={showCgpa}
            onChange={(e) => {
              setShowCgpa(e.target.checked);
              void saveProfile({
                neo_id: profile?.neo_id ?? null,
                reg_no: profile?.reg_no ?? null,
                display_name: profile?.display_name ?? null,
                cohort: profile?.cohort ?? null,
                show_friend_cgpa: e.target.checked,
              });
            }}
          />
          <span className="switch-text">
            <strong>Show CGPA for people in my circle</strong>
            <span>
              Off by default. The cutoff analysis works without this — it only
              needs aggregates.
            </span>
          </span>
        </label>

        <div className="notice" style={{ marginTop: 6 }}>
          nankiv has no account, no server and no telemetry. It reads files you
          give it and stores the results locally. Contact details in reference
          sheets — phone, email, date of birth, resume links — are discarded
          when the file is read and have nowhere to be stored.
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
        <div style={{ marginTop: 14 }}>
          {confirmWipe ? (
            <div className="notice danger">
              <p style={{ margin: "0 0 10px" }}>
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

      <p style={{ fontSize: 11.5, color: "var(--muted)", textAlign: "center" }}>
        nankiv 0.1.0
      </p>
    </div>
  );
}
