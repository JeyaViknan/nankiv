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
    <div className="shell">
      <nav className="sidebar" aria-label="Main">
        <div className="brand">
          <span className="brand-mark">
            <svg width="13" height="13" viewBox="0 0 24 24" fill="none">
              <path
                d="M5 12.5l4.5 4.5L19 7"
                stroke="white"
                strokeWidth="3.2"
                strokeLinecap="round"
                strokeLinejoin="round"
              />
            </svg>
          </span>
          <span className="brand-name">nankiv</span>
        </div>
        <button className="nav-item" aria-current="page">
          Home
        </button>
        <button className="nav-item">
          Circle <span className="count">4</span>
        </button>
        <button className="nav-item">Search</button>
        <button className="nav-item">
          History <span className="count">12</span>
        </button>
        <div className="sidebar-foot">
          <span className="offline-badge">
            <span className="offline-dot" />
            Works offline
          </span>
          <span className="kbd-hint">
            <kbd>⌘</kbd>
            <kbd>,</kbd>
            <span style={{ marginLeft: 2 }}>Settings</span>
          </span>
        </div>
      </nav>

      <main className="main">
        <div className="main-narrow">
          <div className="btn-row" style={{ marginBottom: 28 }}>
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

          <Section title="Verdict — shortlisted">
            <VerdictBanner
              verdict={IN}
              company="Siemens SISW"
              totalStudents={166}
            />
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

          <Section title="Circle">
            <div className="card">
              <div className="card-head">
                <h2>Your circle</h2>
                <span style={{ fontSize: 12, color: "var(--text-3)" }}>
                  2 of 4 shortlisted
                </span>
              </div>
              <div className="person-list">
                {friends.map((f) => (
                  <div className="person" key={f.id}>
                    <div>
                      <span className="person-name">{f.label}</span>
                      <span className="person-id">{f.id}</span>
                    </div>
                    <div className="person-status">
                      {f.cgpa && (
                        <span
                          className="mono"
                          style={{ fontSize: 11.5, color: "var(--text-faint)" }}
                        >
                          {f.cgpa.toFixed(2)}
                        </span>
                      )}
                      <VerdictPill verdict={f.v} />
                    </div>
                  </div>
                ))}
              </div>
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

          <Section title="Controls">
            <div className="card">
              <div className="btn-row" style={{ marginBottom: 16 }}>
                <button className="btn primary">Primary</button>
                <button className="btn">Secondary</button>
                <button className="btn ghost">Ghost</button>
                <button className="btn danger">Danger</button>
                <button className="btn small">Small</button>
              </div>
              <div className="btn-row" style={{ marginBottom: 16 }}>
                <span className="pill yes">In</span>
                <span className="pill no">Not in</span>
                <span className="pill unknown">Unknown</span>
                <span className="pill accent">verified</span>
                <span className="pill quiet">estimate</span>
              </div>
              <div className="field">
                <label htmlFor="p-neo">Neo ID</label>
                <input
                  id="p-neo"
                  type="text"
                  className="mono-input"
                  defaultValue="V9H0G6C4"
                />
              </div>
            </div>
          </Section>
        </div>
      </main>
    </div>
  );
}

applyTheme("dark");
ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <Preview />,
);
