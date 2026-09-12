/**
 * Drive detail — the screen a student actually lives on.
 *
 * Order is fixed and deliberate: your verdict, then your circle, then the
 * analytics. "Did Arjun get it?" is the very next thought after "did I?", every
 * single time, so the friend panel is never behind a tab.
 */

import { useState } from "react";
import { api, type ImportOutcome, type PersonResult } from "../lib/api";
import { useStore } from "../lib/store";
import { VerdictBanner, VerdictPill } from "../components/Verdict";
import {
  BranchCard,
  CoverageNotice,
  CutoffCard,
  DistributionCard,
  InsufficientSample,
  StandingCard,
} from "../components/Analysis";

type Tab = "overview" | "cgpa" | "branches";

function FriendRow({
  person,
  showCgpa,
}: {
  person: PersonResult;
  showCgpa: boolean;
}) {
  return (
    <div className="person">
      <div>
        <span className="person-name">{person.label}</span>
        {person.neo_id && <span className="person-id">{person.neo_id}</span>}
      </div>
      <div className="person-status">
        {showCgpa && person.cgpa !== null && (
          <span
            className="mono"
            style={{ fontSize: 11.5, color: "var(--text-3)" }}
          >
            {person.cgpa.toFixed(2)}
          </span>
        )}
        <VerdictPill verdict={person.verdict} />
      </div>
    </div>
  );
}

export function DriveScreen({ outcome }: { outcome: ImportOutcome }) {
  const [tab, setTab] = useState<Tab>("overview");
  const { showToast, profile, go } = useStore();

  const shortlistedFriends = outcome.friends.filter(
    (f) => f.verdict.status === "shortlisted",
  ).length;
  const a = outcome.analysis;

  async function copySummary() {
    try {
      const text = await api.shareSummary(outcome.drive_id);
      await navigator.clipboard.writeText(text);
      showToast("Summary copied — your own status isn't included");
    } catch {
      showToast("Couldn't copy the summary");
    }
  }

  return (
    <div className="main-narrow">
      <VerdictBanner
        verdict={outcome.you.verdict}
        company={outcome.company}
        totalStudents={outcome.total_students}
      />

      {outcome.learned_verified > 0 && (
        <div className="notice accent">
          This file also taught nankiv{" "}
          <strong>{outcome.learned_verified.toLocaleString()}</strong> verified
          identity links. Every past drive's analysis just got better.
        </div>
      )}

      {/* Circle — second, always, never behind a tab. */}
      <div className="card">
        <div className="card-head">
          <h2>Your circle</h2>
          <span style={{ fontSize: 12, color: "var(--text-3)" }}>
            {outcome.friends.length === 0
              ? "nobody added yet"
              : `${shortlistedFriends} of ${outcome.friends.length} shortlisted`}
          </span>
        </div>
        {outcome.friends.length === 0 ? (
          <div className="empty">
            <p className="empty-title">No friends added</p>
            <p>
              Add their Neo IDs once and they're checked on every shortlist.
            </p>
            <button
              className="btn small"
              style={{ marginTop: 12 }}
              onClick={() => go("circle")}
            >
              Add friends
            </button>
          </div>
        ) : (
          <div className="person-list">
            {outcome.friends.map((f) => (
              <FriendRow
                key={f.neo_id ?? f.reg_no ?? f.label}
                person={f}
                showCgpa={profile?.show_friend_cgpa ?? false}
              />
            ))}
          </div>
        )}
      </div>

      <div className="btn-row" style={{ margin: "18px 0 14px" }}>
        <button className="btn" onClick={copySummary}>
          Copy summary for the group chat
        </button>
      </div>

      <div className="tabs" role="tablist">
        {(["overview", "cgpa", "branches"] as Tab[]).map((t) => (
          <button
            key={t}
            role="tab"
            className="tab"
            aria-selected={tab === t}
            onClick={() => setTab(t)}
          >
            {t === "overview" ? "Overview" : t === "cgpa" ? "CGPA" : "Branches"}
          </button>
        ))}
      </div>

      {!a.sufficient ? (
        <InsufficientSample analysis={a} />
      ) : (
        <>
          <CoverageNotice analysis={a} />

          {tab === "overview" && (
            <>
              {a.cutoff && <CutoffCard report={a.cutoff} />}
              {a.your_percentile && (
                <StandingCard percentile={a.your_percentile} />
              )}
              {a.branches && a.branches.over_represented.length > 0 && (
                <div className="notice">{a.branches.statement}</div>
              )}
            </>
          )}

          {tab === "cgpa" && a.cgpa && (
            <DistributionCard dist={a.cgpa.value} yourCgpa={outcome.you.cgpa} />
          )}

          {tab === "branches" &&
            (a.branches ? (
              <BranchCard report={a.branches} />
            ) : (
              <div className="card">
                <div className="empty">
                  <p className="empty-title">No branch data</p>
                  <p>
                    Import a reference sheet with branch information to see
                    this.
                  </p>
                </div>
              </div>
            ))}
        </>
      )}
    </div>
  );
}
