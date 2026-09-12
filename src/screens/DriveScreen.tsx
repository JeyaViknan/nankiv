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
import { api, type ImportOutcome, type PersonResult } from "../lib/api";
import { useStore } from "../lib/store";
import { NeedsReference } from "../components/NeedsReference";
import { VerdictBanner, VerdictPill } from "../components/Verdict";
import {
  BranchCard,
  CoverageNotice,
  CutoffCard,
  DistributionCard,
  InsufficientSample,
  StandingCard,
} from "../components/Analysis";

function FriendRow({
  person,
  showCgpa,
}: {
  person: PersonResult;
  showCgpa: boolean;
}) {
  return (
    <div className="row">
      <span className="row-main static">
        <span className="row-title">{person.label}</span>
        {person.neo_id && (
          <span className="row-meta mono">{person.neo_id}</span>
        )}
      </span>
      <span className="row-trail">
        {showCgpa && person.cgpa !== null && (
          <span className="row-figure">{person.cgpa.toFixed(2)}</span>
        )}
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

  return (
    <button
      className="name-button"
      onClick={() => setEditing(true)}
      title="Rename this drive"
    >
      {name}
    </button>
  );
}

export function DriveScreen({ outcome }: { outcome: ImportOutcome }) {
  const { showToast, profile, drives, openDrive, stats } = useStore();
  const [copied, setCopied] = useState(false);
  const a = outcome.analysis;

  const shortlisted = outcome.friends.filter(
    (f) => f.verdict.status === "shortlisted",
  ).length;

  // Comparison is offered where it is relevant rather than through a selection
  // mode. Other drives from the same company are almost always what you want
  // to compare against, so they are the ones offered.
  const siblings = drives.filter(
    (d) => d.id !== outcome.drive_id && d.company === outcome.company,
  );
  const [diff, setDiff] = useState<Awaited<
    ReturnType<typeof api.compareRounds>
  > | null>(null);

  async function compareWith(otherId: number) {
    try {
      setDiff(await api.compareRounds(otherId, outcome.drive_id));
    } catch {
      showToast("Couldn't compare those rounds");
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
      />

      {/* Provenance, stated once, quietly, directly under the answer. */}
      <div className="provenance">
        <EditableName id={outcome.drive_id} name={outcome.company} />
        <span className="row-dot">·</span>
        <span>{outcome.total_students.toLocaleString()} shortlisted</span>
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
            Add people once and they're checked on every shortlist from then on.
            Press <kbd>⌘</kbd>
            <kbd>D</kbd> to open your circle.
          </p>
        ) : (
          <div className="list">
            {outcome.friends.map((f) => (
              <FriendRow
                key={f.neo_id ?? f.reg_no ?? f.label}
                person={f}
                showCgpa={profile?.show_friend_cgpa ?? false}
              />
            ))}
          </div>
        )}
      </section>

      {siblings.length > 0 && (
        <section className="block">
          <div className="list-head">
            <span className="eyebrow">Rounds</span>
          </div>
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
            <div className="list">
              {siblings.map((s) => (
                <div className="row" key={s.id}>
                  <button
                    className="row-main"
                    onClick={() => void openDrive(s.id)}
                  >
                    <span className="row-title">{s.company}</span>
                    <span className="row-meta">
                      {s.total_students.toLocaleString()} shortlisted
                    </span>
                  </button>
                  <button
                    className="btn small"
                    onClick={() => void compareWith(s.id)}
                  >
                    Compare
                  </button>
                </div>
              ))}
            </div>
          )}
        </section>
      )}

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
            {a.your_percentile && (
              <StandingCard percentile={a.your_percentile} />
            )}
            {a.cgpa && (
              <DistributionCard
                dist={a.cgpa.value}
                yourCgpa={outcome.you.cgpa}
              />
            )}
            {a.branches && <BranchCard report={a.branches} />}
          </>
        )}
      </section>

      <footer className="drive-foot">
        <button className="btn" onClick={copySummary}>
          {copied ? "Copied" : "Copy summary for the group chat"}
        </button>
        {outcome.learned_verified > 0 && (
          <p className="foot-note">
            This file taught nankiv {outcome.learned_verified.toLocaleString()}{" "}
            verified identity links.
          </p>
        )}
      </footer>
    </div>
  );
}
