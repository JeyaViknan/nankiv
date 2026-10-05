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
import { NeedsReference } from "./components/NeedsReference";
import { SeasonStrip } from "./components/SeasonStrip";
import { MenuButton } from "./components/MenuButton";
import { Icon, type IconName } from "./components/Icon";
import { applyTheme, getThemeChoice, type ThemeChoice } from "./lib/theme";
import { drawCard, type CardFacts } from "./lib/shareCard";
import { ShareCard } from "./components/ShareCard";
import type {
  BranchReport,
  CutoffReport,
  Distribution,
  DriveAnalysis,
  ImportOutcome,
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
    "Looks like a CGPA cutoff around 9.0, lowest found 8.94. An estimate, not an official cutoff.",
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
  statement: "ECE is over-represented against the batch.",
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

/** The share overlay, as the drive screen opens it. */
function ShareOverlayDemo() {
  const [open, setOpen] = useState(false);
  const outcome = {
    drive_id: 1,
    company: "Siemens SISW",
    total_students: 149,
    progression: null,
  } as unknown as ImportOutcome;
  return (
    <>
      <button className="btn" onClick={() => setOpen(true)}>
        Open the share card
      </button>
      {open && <ShareCard outcome={outcome} onClose={() => setOpen(false)} />}
    </>
  );
}

/** The share card, drawn exactly as the app draws it. */
function CardPreview({
  facts,
  large = false,
}: {
  facts: CardFacts;
  large?: boolean;
}) {
  const ref = React.useRef<HTMLCanvasElement>(null);
  React.useEffect(() => {
    if (ref.current) void drawCard(ref.current, facts);
  }, [facts]);
  return (
    <canvas
      ref={ref}
      style={large ? { width: 540, height: 675 } : { width: 324, height: 405 }}
      aria-label={`I'm in card for ${facts.company}`}
    />
  );
}

const CARDS: CardFacts[] = [
  {
    company: "Siemens SISW",
    round: "Interview",
    when: new Date("2026-10-05T10:00:00Z"),
    seed: 0,
  },
  {
    company: "Deloitte Consultative Offerings",
    round: null,
    when: new Date("2026-09-12T10:00:00Z"),
    seed: 1,
  },
  {
    company: "Axxela",
    round: "Test",
    when: new Date("2026-10-05T10:00:00Z"),
    seed: 7,
  },
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
  const [theme, setTheme] = useState<ThemeChoice>(getThemeChoice);

  function pick(t: ThemeChoice) {
    setTheme(t);
    localStorage.setItem("nankiv.theme", t);
    applyTheme(t);
  }

  // The cards alone, large, on grey: the corners and notches are cut out of
  // the image, and only show against something.
  if (window.location.hash === "#cards") {
    return (
      <div
        style={{
          display: "flex",
          flexWrap: "wrap",
          gap: 32,
          padding: 32,
          minHeight: "100vh",
          background: "#48484a",
        }}
      >
        {CARDS.map((f) => (
          <CardPreview key={f.company} facts={f} large />
        ))}
      </div>
    );
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
          <Section title="Icon set — one grid, one optical weight">
            <div className="card">
              <div
                style={{
                  display: "grid",
                  gridTemplateColumns: "repeat(auto-fill, minmax(78px, 1fr))",
                  gap: 14,
                }}
              >
                {(
                  [
                    "back",
                    "check",
                    "chevronRight",
                    "close",
                    "compare",
                    "dash",
                    "gear",
                    "people",
                    "plus",
                    "question",
                    "search",
                    "sheet",
                    "trash",
                    "tray",
                    "warning",
                  ] as IconName[]
                ).map((n) => (
                  <div key={n} style={{ textAlign: "center" }}>
                    <div
                      style={{
                        display: "grid",
                        placeItems: "center",
                        height: 40,
                        color: "var(--text-2)",
                      }}
                    >
                      <Icon name={n} size={20} />
                    </div>
                    <div style={{ fontSize: 10, color: "var(--text-faint)" }}>
                      {n}
                    </div>
                  </div>
                ))}
              </div>
              <div
                style={{
                  display: "flex",
                  alignItems: "center",
                  gap: 20,
                  marginTop: 20,
                  paddingTop: 16,
                  borderTop: "1px solid var(--border)",
                  color: "var(--text-2)",
                }}
              >
                {[14, 17, 20, 26, 34, 44].map((sz) => (
                  <Icon key={sz} name="tray" size={sz} />
                ))}
                <span style={{ fontSize: 11, color: "var(--text-faint)" }}>
                  optical weight held across sizes
                </span>
              </div>
            </div>
          </Section>

          <Section title="Share — the overlay">
            <ShareOverlayDemo />
          </Section>

          <Section title="Share card — I'm in">
            <div style={{ display: "flex", gap: 24, flexWrap: "wrap" }}>
              {CARDS.map((f) => (
                <CardPreview key={f.company} facts={f} />
              ))}
            </div>
          </Section>

          <Section title="Season strip — what replaced the window title">
            <div className="preview-strip">
              <SeasonStrip
                season={{
                  drives: 12,
                  shortlisted: 4,
                  not_shortlisted: 7,
                  undetermined: 1,
                }}
              />
            </div>
            <div className="preview-strip">
              <SeasonStrip season={null} />
            </div>
          </Section>

          <Section title="Verdict — shortlisted">
            <VerdictBanner
              verdict={IN}
              company="Siemens SISW"
              totalStudents={166}
              evidence={{
                key: "neo_id",
                yours: "X5L6S5B8",
                listed: 166,
                found_at: {
                  kind: "neo_id",
                  value: "X5L6S5B8",
                  sheet: "Shortlist",
                  row: 42,
                  column: "C",
                  header: "Neo ID",
                },
                excerpt: [
                  { row: 40, value: "H4J3V6N2" },
                  { row: 41, value: "J2T0S6Y7" },
                  { row: 42, value: "X5L6S5B8" },
                  { row: 43, value: "D7J9E5J8" },
                  { row: 44, value: "K8M2P4R6" },
                ],
              }}
            />
            <div className="provenance">
              <span className="name-button">Siemens SISW</span>
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
              onFix={() => {}}
              verdict={UNKNOWN}
              company="HPE"
              totalStudents={874}
            />
          </Section>

          <Section title="The invitation">
            <button className="invite">
              <span className="invite-icon">
                <Icon name="tray" size={32} />
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

          <Section title="Download names">
            <div
              className="drive-foot"
              style={{ marginTop: 150, borderTop: "none" }}
            >
              <div className="btn-row">
                <MenuButton
                  label="Download names"
                  choices={[
                    { id: "xlsx", label: "Excel workbook", hint: ".xlsx  ⌘E" },
                    { id: "csv", label: "CSV", hint: ".csv  ⇧⌘E" },
                  ]}
                  onChoose={() => {}}
                />
                <button className="btn">Copy summary</button>
              </div>
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

          <Section title="Not set up — no academic data at all">
            <NeedsReference />
          </Section>

          <Section title="Not set up — compact">
            <NeedsReference compact />
          </Section>

          <Section title="Thin sample — reference data exists, coverage is low">
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
                <Icon name="warning" size={17} />
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

applyTheme(getThemeChoice());
ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <Preview />,
);
