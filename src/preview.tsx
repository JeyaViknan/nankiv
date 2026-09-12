/**
 * Design preview harness.
 *
 * Renders the real components with the real stylesheet against representative
 * data, so the visual design can be reviewed without launching the desktop
 * shell. Development only — it is not part of the shipped bundle and imports
 * nothing from Tauri.
 *
 *   npm run preview:design
 */

import React, { useState } from "react";
import ReactDOM from "react-dom/client";
import { VerdictBanner, VerdictPill } from "./components/Verdict";
import {
  BranchCard,
  CoverageNotice,
  CutoffCard,
  DistributionCard,
  InsufficientSample,
  StandingCard,
} from "./components/Analysis";
import { applyTheme, type ThemeChoice } from "./lib/theme";
import type {
  BranchReport,
  CutoffReport,
  Distribution,
  DriveAnalysis,
  Verdict,
} from "./lib/api";
import "./styles/app.css";

const cutoff: CutoffReport = {
  verdict: {
    value: { kind: "hard_cutoff", threshold: 9.0, observed_floor: 8.94 },
    matched: 25,
    total: 166,
  },
  comparison: [
    {
      threshold: 9.5,
      share_below_shortlist: 0.8,
      share_below_batch: 0.94,
      is_signal: false,
    },
    {
      threshold: 9.0,
      share_below_shortlist: 0.04,
      share_below_batch: 0.63,
      is_signal: true,
    },
    {
      threshold: 8.5,
      share_below_shortlist: 0.0,
      share_below_batch: 0.25,
      is_signal: false,
    },
    {
      threshold: 8.0,
      share_below_shortlist: 0.0,
      share_below_batch: 0.02,
      is_signal: false,
    },
  ],
  statement:
    "Looks like a CGPA cutoff around 9.0 — the lowest we found was 8.94 (based on 25 of 166 shortlisted students we could match). This is an estimate, not an official cutoff.",
};

const dist: Distribution = {
  n: 25,
  min: 8.94,
  max: 9.71,
  median: 9.28,
  mean: 9.3,
  std_dev: 0.21,
  p5: 9.01,
  p25: 9.1,
  p75: 9.5,
  buckets: [
    { lower: 8.5, upper: 8.75, count: 0 },
    { lower: 8.75, upper: 9.0, count: 1 },
    { lower: 9.0, upper: 9.25, count: 10 },
    { lower: 9.25, upper: 9.5, count: 9 },
    { lower: 9.5, upper: 9.75, count: 5 },
    { lower: 9.75, upper: 10.0, count: 0 },
  ],
};

const branches: BranchReport = {
  rows: {
    value: [
      {
        branch: "CSE",
        count: 17,
        share: 0.68,
        baseline_share: 0.6,
        lift: 1.13,
      },
      {
        branch: "ECE",
        count: 8,
        share: 0.32,
        baseline_share: 0.075,
        lift: 4.27,
      },
    ],
    matched: 25,
    total: 166,
  },
  over_represented: ["ECE"],
  statement:
    "ECE is noticeably over-represented compared to the batch (based on 25 of 166 students).",
};

const analysis: DriveAnalysis = {
  total_students: 166,
  matched_students: 25,
  coverage: 0.15,
  sufficient: true,
  cgpa: { value: dist, matched: 25, total: 166 },
  cutoff,
  branches,
  your_percentile: { value: 0.68, matched: 25, total: 166 },
};

const gated: DriveAnalysis = {
  total_students: 125,
  matched_students: 1,
  coverage: 0.008,
  sufficient: false,
  cgpa: null,
  cutoff: null,
  branches: null,
  your_percentile: null,
};

const IN: Verdict = { status: "shortlisted" };
const OUT: Verdict = { status: "not_shortlisted" };
const UNKNOWN: Verdict = {
  status: "undetermined",
  reason: "key_kind_not_configured",
  file_key: "reg_no",
};

const friends: { label: string; id: string; v: Verdict; cgpa?: number }[] = [
  { label: "Arjun Menon", id: "V9H0G6C4", v: IN, cgpa: 9.41 },
  { label: "Divya Rao", id: "C5U6K1E7", v: IN, cgpa: 9.12 },
  { label: "Kabir Sharma", id: "T2D4R9N9", v: OUT },
  { label: "Nisha Iyer", id: "Q2X9B4S6", v: UNKNOWN },
];

function Section({
  title,
  children,
}: {
  title: string;
  children: React.ReactNode;
}) {
  return (
    <section style={{ marginBottom: 42 }}>
      <p className="eyebrow" style={{ marginBottom: 12 }}>
        {title}
      </p>
      {children}
    </section>
  );
}

function Preview() {
  const [theme, setTheme] = useState<ThemeChoice>("dark");

  function pick(t: ThemeChoice) {
    setTheme(t);
    applyTheme(t);
  }

  return (
    <div className="app">
      <header className="toolbar">
        <div className="toolbar-left">
          <span className="wordmark">nankiv</span>
        </div>
        <div className="search-wrap">
          <input
            className="search-input"
            placeholder="Search a name or Neo ID"
            readOnly
          />
        </div>
        <div className="toolbar-right">
          {(["system", "light", "dark"] as ThemeChoice[]).map((t) => (
            <button
              key={t}
              className={`btn small${theme === t ? " primary" : ""}`}
              onClick={() => pick(t)}
            >
              {t}
            </button>
          ))}
        </div>
      </header>

      <main className="content">
        <div className="surface">
          <Section title="Verdict — shortlisted">
            <VerdictBanner
              verdict={IN}
              company="Siemens SISW"
              totalStudents={166}
            />
            <div className="provenance">
              <span className="name-button">Siemens SISW</span>
              <span className="row-dot">·</span>
              <span>166 shortlisted</span>
              <span className="row-dot">·</span>
              <span className="prov-key">keyed by Neo ID</span>
            </div>
          </Section>

          <Section title="Verdict — not shortlisted">
            <VerdictBanner
              verdict={OUT}
              company="Siemens SISW"
              totalStudents={166}
            />
          </Section>

          <Section title="Verdict — cannot determine">
            <VerdictBanner
              verdict={UNKNOWN}
              company="HPE"
              totalStudents={874}
            />
          </Section>

          <Section title="The invitation">
            <button className="invite">
              <span className="invite-icon">
                <svg
                  width="34"
                  height="34"
                  viewBox="0 0 40 40"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="1.5"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                >
                  <path d="M20 7v17M13.5 17.5L20 24l6.5-6.5" />
                  <path d="M7 26v4.5A2.5 2.5 0 0 0 9.5 33h21a2.5 2.5 0 0 0 2.5-2.5V26" />
                </svg>
              </span>
              <span className="invite-text">
                <span className="invite-title">Drop a shortlist</span>
                <span className="invite-sub">
                  Anywhere in this window — or press <kbd>⌘</kbd>
                  <kbd>O</kbd> to choose a file
                </span>
              </span>
            </button>
          </Section>

          <Section title="Timeline">
            <div className="list">
              {[
                ["Siemens SISW", "166 shortlisted", "Today"],
                ["Tredence", "819 shortlisted", "Yesterday"],
                ["Amazon", "527 shortlisted", "3 days ago"],
              ].map(([a, b, c]) => (
                <div className="row" key={a}>
                  <button className="row-main">
                    <span className="row-title">{a}</span>
                    <span className="row-meta">
                      {b}
                      <span className="row-dot">·</span>
                      {c}
                    </span>
                  </button>
                </div>
              ))}
            </div>
          </Section>

          <Section title="Circle">
            <div className="list">
              {friends.map((f) => (
                <div className="row" key={f.id}>
                  <span className="row-main static">
                    <span className="row-title">{f.label}</span>
                    <span className="row-meta mono">{f.id}</span>
                  </span>
                  <span className="row-trail">
                    {f.cgpa && (
                      <span className="row-figure">{f.cgpa.toFixed(2)}</span>
                    )}
                    <VerdictPill verdict={f.v} />
                  </span>
                </div>
              ))}
            </div>
          </Section>

          <Section title="Analytics">
            <CoverageNotice analysis={analysis} />
            <CutoffCard report={cutoff} />
            <StandingCard percentile={analysis.your_percentile!} />
            <DistributionCard dist={dist} yourCgpa={9.41} />
            <BranchCard report={branches} />
          </Section>

          <Section title="Gated sample">
            <InsufficientSample analysis={gated} />
          </Section>

          <Section title="Import — reading">
            <div className="import-status reading">
              <span className="progress-bar">
                <span className="progress-fill" />
              </span>
              <span className="import-text">
                Reading{" "}
                <span className="import-file">
                  Siemens SISW shortlist 2027.xlsx
                </span>
              </span>
            </div>
          </Section>

          <Section title="Import — unreadable file">
            <div className="import-status failed">
              <span className="import-icon">
                <svg
                  width="17"
                  height="17"
                  viewBox="0 0 20 20"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="1.6"
                  strokeLinecap="round"
                >
                  <circle cx="10" cy="10" r="7.5" />
                  <path d="M10 6.2v4.4" />
                  <circle
                    cx="10"
                    cy="13.8"
                    r="0.9"
                    fill="currentColor"
                    stroke="none"
                  />
                </svg>
              </span>
              <div className="import-body">
                <p className="import-headline">
                  This file doesn't use Neo IDs or registration numbers.
                </p>
                <p className="import-file-line">
                  TCS Shortlist 27th (Cognizant).xlsx
                </p>
                <p className="import-detail">
                  Columns found: REFERENCE_ID, Interview Date
                </p>
                <p className="import-reassure">
                  This isn't a result — it says nothing about whether you were
                  shortlisted.
                </p>
              </div>
              <button className="btn small">Dismiss</button>
            </div>
          </Section>

          <Section title="Controls">
            <div className="card">
              <div className="btn-row" style={{ marginBottom: 14 }}>
                <button className="btn primary">Primary</button>
                <button className="btn">Secondary</button>
                <button className="btn danger">Danger</button>
              </div>
              <div className="btn-row">
                <span className="pill yes">In</span>
                <span className="pill no">Not in</span>
                <span className="pill unknown">Unknown</span>
                <span className="pill quiet">estimate</span>
              </div>
            </div>
          </Section>
        </div>
      </main>

      <div className="toast">
        <span>Removed Amazon</span>
        <button className="toast-action">Undo</button>
      </div>
    </div>
  );
}

applyTheme("dark");
ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <Preview />,
);
