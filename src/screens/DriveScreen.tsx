/**
 * The result.
 *
 * Order is the whole design here, and it is fixed: your verdict, your circle,
 * then everything else. "Did Arjun get it?" is the very next thought after "did
 * I?", every single time — so the circle is never behind a tab, and nothing
 * decorative is allowed between them.
 *
 * The previous build put a "learned N identity links" notice in the second-most
 * prominent slot on the page. It is the least important thing here and now sits
 * at the bottom with the other provenance.
 *
 * The three analytics tabs are gone. Overview / CGPA / Branches was an arbitrary
 * split — the overview tab was mostly a CGPA finding — and tabs hid content that
 * reads perfectly well as one scroll.
 */

import { useEffect, useRef, useState } from "react";
import {
  api,
  type ExportFormat,
  type ImportOutcome,
  type PersonResult,
} from "../lib/api";
import { useStore } from "../lib/store";
import { NeedsReference } from "../components/NeedsReference";
import { MenuButton } from "../components/MenuButton";
import { VerdictBanner, VerdictPill } from "../components/Verdict";
import { RoundTrail, neighbour } from "../components/Rounds";
import { ShareCard } from "../components/ShareCard";
import { MOD, chord } from "../lib/keys";
import {
  BranchCard,
  CoverageNotice,
  CutoffCard,
  DistributionCard,
  InsufficientSample,
  StandingCard,
  StandingPrompt,
} from "../components/Analysis";

/** A person in your circle: their answer, and nothing else about them. */
function FriendRow({ person }: { person: PersonResult }) {
  return (
    <div className="row">
      <span className="row-main static">
        <span className="row-title">{person.label}</span>
        {person.neo_id && (
          <span className="row-meta mono">{person.neo_id}</span>
        )}
      </span>
      <span className="row-trail">
        <VerdictPill verdict={person.verdict} />
      </span>
    </div>
  );
}

/**
 * The drive name, editable in place.
 *
 * It is inferred from the filename at import, and a filename is not a promise —
 * one of the sampled files would land as "Unnamed drive". Correcting it should
 * cost one click, not a trip to a settings screen, and it must never have
 * required a confirmation step at import time.
 */
function EditableName({ id, name }: { id: number; name: string }) {
  const { renameDrive } = useStore();
  const [editing, setEditing] = useState(false);
  const [value, setValue] = useState(name);
  const ref = useRef<HTMLInputElement>(null);

  useEffect(() => setValue(name), [name]);
  useEffect(() => {
    if (editing) ref.current?.select();
  }, [editing]);

  function commit() {
    setEditing(false);
    const next = value.trim();
    if (next && next !== name) void renameDrive(id, next);
    else setValue(name);
  }

  if (editing) {
    return (
      <input
        ref={ref}
        className="name-input"
        value={value}
        onChange={(e) => setValue(e.target.value)}
        onBlur={commit}
        onKeyDown={(e) => {
          if (e.key === "Enter") commit();
          if (e.key === "Escape") {
            setValue(name);
            setEditing(false);
          }
        }}
        aria-label="Drive name"
      />
    );
  }

  // Nothing in the file named it, so ask rather than show a placeholder as if
  // it were a name.
  const unnamed = name === UNNAMED;
  return (
    <button
      className={`name-button${unnamed ? " prompt" : ""}`}
      onClick={() => setEditing(true)}
      title="Rename this drive"
    >
      {unnamed ? "Name this drive" : name}
    </button>
  );
}

/** What the core calls a drive nothing in its file could name. */
const UNNAMED = "Unnamed drive";

export function DriveScreen({
  outcome,
  onFix,
}: {
  outcome: ImportOutcome;
  /** Opens Settings when the answer is "can't tell" and a field would fix it. */
  onFix: () => void;
}) {
  const { showToast, openDrive, refreshDrives, stats, exportNames } =
    useStore();
  const [copied, setCopied] = useState(false);
  const [sharing, setSharing] = useState(false);
  const a = outcome.analysis;

  const shortlisted = outcome.friends.filter(
    (f) => f.verdict.status === "shortlisted",
  ).length;

  // Comparison is offered where it is relevant rather than through a selection
  // mode: against the round before this one, or after it for a first round.
  const rounds = outcome.progression;
  const other = rounds ? neighbour(rounds, outcome.drive_id) : null;
  const [diff, setDiff] = useState<Awaited<
    ReturnType<typeof api.compareRounds>
  > | null>(null);
  useEffect(() => setDiff(null), [outcome.drive_id]);

  async function compare() {
    if (!other) return;
    const [from, to] = other.earlier
      ? [other.step.drive_id, outcome.drive_id]
      : [outcome.drive_id, other.step.drive_id];
    try {
      setDiff(await api.compareRounds(from, to));
    } catch {
      showToast("Couldn't compare those rounds");
    }
  }

  // Rounds are linked only on strong evidence, but evidence can still be
  // wrong; undoing a link is one click and never touches either list.
  async function separate() {
    try {
      await api.setDriveRound(outcome.drive_id, null);
      await Promise.all([refreshDrives(), openDrive(outcome.drive_id)]);
      showToast("Separated — it's its own drive now");
    } catch {
      showToast("Couldn't separate those rounds");
    }
  }

  async function copySummary() {
    try {
      const text = await api.shareSummary(outcome.drive_id);
      await navigator.clipboard.writeText(text);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
      showToast("Copied — your own status isn't included");
    } catch {
      showToast("Couldn't copy the summary");
    }
  }

  return (
    <div className="surface">
      <VerdictBanner
        verdict={outcome.you.verdict}
        company={outcome.company}
        totalStudents={outcome.total_students}
        onFix={onFix}
        evidence={outcome.evidence}
        onShare={() => setSharing(true)}
      />
      {sharing && (
        <ShareCard outcome={outcome} onClose={() => setSharing(false)} />
      )}

      {/* Provenance, stated once, quietly, directly under the answer. */}
      <div className="provenance">
        <EditableName id={outcome.drive_id} name={outcome.company} />
        {/* A yes or a no already says how many; "can't tell" does not. */}
        {outcome.you.verdict.status === "undetermined" && (
          <>
            <span className="row-dot">·</span>
            <span>{outcome.total_students.toLocaleString()} shortlisted</span>
          </>
        )}
        <span className="row-dot">·</span>
        <span className="prov-key">
          keyed by {outcome.primary_key === "reg_no" ? "reg number" : "Neo ID"}
        </span>
      </div>

      <section className="block">
        <div className="list-head">
          <span className="eyebrow">Your circle</span>
          <span className="list-count">
            {outcome.friends.length === 0
              ? "nobody added"
              : `${shortlisted} of ${outcome.friends.length} in`}
          </span>
        </div>
        {outcome.friends.length === 0 ? (
          <p className="search-note">
            Add friends with <kbd>{MOD}</kbd>
            <kbd>D</kbd> and they're checked on every list.
          </p>
        ) : (
          <div className="list">
            {outcome.friends.map((f) => (
              <FriendRow key={f.neo_id ?? f.reg_no ?? f.label} person={f} />
            ))}
          </div>
        )}
      </section>

      {rounds && (
        <section className="block">
          <div className="list-head">
            <span className="eyebrow">Rounds</span>
          </div>
          <RoundTrail
            progression={rounds}
            current={outcome.drive_id}
            onOpen={(id) => void openDrive(id)}
          />
          {diff ? (
            <div className="panel">
              <div className="stats">
                <div className="stat">
                  <div className="stat-value ok">{diff.advanced}</div>
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
                <p className="panel-note">
                  <strong>Advanced from your circle:</strong>{" "}
                  {diff.advanced_friends.join(", ")}
                </p>
              )}
              {diff.dropped_friends.length > 0 && (
                <p className="panel-note muted">
                  Dropped: {diff.dropped_friends.join(", ")}
                </p>
              )}
              <button className="btn small" onClick={() => setDiff(null)}>
                Done
              </button>
            </div>
          ) : (
            <div className="btn-row round-actions">
              {other && (
                <button className="btn small" onClick={() => void compare()}>
                  Compare with {other.step.label}
                </button>
              )}
              {other?.earlier && (
                <button
                  className="btn small plain"
                  onClick={() => void separate()}
                >
                  Not a round of this drive? Separate it
                </button>
              )}
            </div>
          )}
        </section>
      )}

      {/* Last, whatever the answer was. Your people come straight after it
          every time; the page never rearranges itself around a no. */}
      <section className="block">
        <div className="list-head">
          <span className="eyebrow">What this shortlist suggests</span>
        </div>

        {/* Two different situations that were being shown as one. No academic
            records at all is a setup step, not a thin sample. */}
        {stats?.academics_known === 0 ? (
          <NeedsReference />
        ) : !a.sufficient ? (
          <InsufficientSample analysis={a} />
        ) : (
          <>
            <CoverageNotice analysis={a} />
            {a.cutoff && <CutoffCard report={a.cutoff} />}
            {a.your_percentile ? (
              <StandingCard percentile={a.your_percentile} />
            ) : (
              outcome.your_cgpa === null && <StandingPrompt onAdd={onFix} />
            )}
            {a.cgpa && (
              <DistributionCard
                dist={a.cgpa.value}
                yourCgpa={outcome.your_cgpa}
              />
            )}
            {a.branches && <BranchCard report={a.branches} />}
          </>
        )}
      </section>

      <footer className="drive-foot">
        <div className="btn-row">
          <MenuButton
            label="Download names"
            choices={[
              {
                id: "xlsx",
                label: "Excel workbook",
                hint: `.xlsx  ${chord("E")}`,
              },
              { id: "csv", label: "CSV", hint: `.csv  ${chord("E", true)}` },
            ]}
            onChoose={(id) => void exportNames(id as ExportFormat)}
          />
          <button className="btn" onClick={copySummary}>
            {copied ? "Copied" : "Copy summary"}
          </button>
        </div>
        {outcome.learned_verified > 0 && (
          <p className="foot-note">
            Learned {outcome.learned_verified.toLocaleString()} identity links
            from this file.
          </p>
        )}
      </footer>
    </div>
  );
}
