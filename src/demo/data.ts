/**
 * The season the README's screenshots show.
 *
 * Every student here is invented: the names, the Neo IDs, the registration
 * numbers and the CGPAs. Nothing comes from the reference data or from any
 * real shortlist, so the pictures can be published. The companies are real
 * names, because a shortlist is from someone.
 */

import type {
  DriveAnalysis,
  DriveListItem,
  Friend,
  IdentityStats,
  ImportOutcome,
  PersonResult,
  Profile,
  RecentDownload,
  Season,
  Verdict,
} from "../lib/api";

const IN: Verdict = { status: "shortlisted" };
const OUT: Verdict = { status: "not_shortlisted" };
const UNREADABLE: Verdict = {
  status: "undetermined",
  reason: "file_not_understood",
};

export const profile: Profile = {
  neo_id: "V9H0G6C4",
  reg_no: "23BCE0417",
  display_name: null,
  cohort: null,
  cgpa: 8.62,
  cgpa_updated_at: "2026-09-02 10:00:00",
};

export const friends: Friend[] = [
  ["Aarav Menon", "R4T7K2M9"],
  ["Diya Raman", "B6N3Q8W1"],
  ["Kabir Sethi", "H2J9X5C7"],
  ["Meera Iyer", "L8P1S4D6"],
].map(([label, neo], i) => ({
  id: i + 1,
  label: label!,
  neo_id: neo!,
  reg_no: null,
  group_tag: null,
}));

function day(daysAgo: number, hour = 10): string {
  const d = new Date();
  d.setDate(d.getDate() - daysAgo);
  d.setHours(hour, 0, 0, 0);
  return d.toISOString().replace("T", " ").slice(0, 19);
}

function drive(
  id: number,
  company: string,
  total: number,
  verdict: Verdict,
  daysAgo: number,
  round: string | null = null,
  parent: number | null = null,
): DriveListItem {
  return {
    id,
    company,
    drive_date: null,
    imported_at: day(daysAgo),
    source_filename: `${company} shortlist.xlsx`,
    content_hash: `demo-${id}`,
    shape: "neo_id_only",
    primary_key: "neo_id",
    total_students: total,
    round_label: round,
    parent_drive_id: parent,
    verdict,
    round,
  };
}

export const drives: DriveListItem[] = [
  drive(1, "Siemens", 149, IN, 0, "Interview", 2),
  drive(3, "Tredence", 819, IN, 1),
  drive(4, "Deloitte India", 1146, OUT, 2, "Test"),
  drive(2, "Siemens", 612, IN, 4, "Test"),
  drive(5, "Zluri", 400, IN, 6),
  drive(6, "TCS", 2210, UNREADABLE, 8),
  drive(7, "HPE", 304, IN, 11),
  drive(8, "Fractal", 1077, OUT, 13),
];

export const season: Season = {
  drives: drives.length,
  shortlisted: drives.filter((d) => d.verdict.status === "shortlisted").length,
  not_shortlisted: drives.filter((d) => d.verdict.status === "not_shortlisted")
    .length,
  undetermined: drives.filter((d) => d.verdict.status === "undetermined")
    .length,
};

export const stats: IdentityStats = {
  students_known: 2506,
  names_known: 2318,
  academics_known: 2504,
  edges: 917,
  conflicts: 3,
};

export const recent: RecentDownload[] = [
  {
    path: "/demo/Downloads/Accenture shortlist.xlsx",
    name: "Accenture shortlist.xlsx",
    modified: day(0, 9),
    imported: false,
  },
];

function person(label: string, neo: string, verdict: Verdict): PersonResult {
  return { label, neo_id: neo, reg_no: null, verdict, confidence: "verified" };
}

/** Who in the circle made which list. */
const circleIn: Record<number, string[]> = {
  1: ["Aarav Menon", "Meera Iyer"],
  2: ["Aarav Menon", "Diya Raman", "Meera Iyer"],
  3: ["Diya Raman", "Kabir Sethi"],
};

function circle(id: number): PersonResult[] {
  const made = circleIn[id] ?? [];
  return friends
    .map((f) => person(f.label, f.neo_id!, made.includes(f.label) ? IN : OUT))
    .sort(
      (a, b) =>
        Number(b.verdict.status === "shortlisted") -
          Number(a.verdict.status === "shortlisted") ||
        a.label.localeCompare(b.label),
    );
}

/** A shortlist that clearly filtered on CGPA: almost nobody under 8.0. */
const analysis: DriveAnalysis = {
  total_students: 149,
  matched_students: 121,
  coverage: 121 / 149,
  sufficient: true,
  cgpa: {
    value: {
      n: 121,
      min: 8.04,
      max: 9.71,
      median: 8.86,
      mean: 8.84,
      std_dev: 0.38,
      p5: 8.12,
      p25: 8.55,
      p75: 9.12,
      buckets: [
        [7.6, 0],
        [7.8, 0],
        [8.0, 6],
        [8.2, 11],
        [8.4, 17],
        [8.6, 22],
        [8.8, 24],
        [9.0, 18],
        [9.2, 12],
        [9.4, 7],
        [9.6, 4],
        [9.8, 0],
      ].map(([lower, count]) => ({
        lower: lower!,
        upper: Math.round((lower! + 0.2) * 10) / 10,
        count: count!,
      })),
    },
    matched: 121,
    total: 149,
  },
  cutoff: {
    verdict: {
      value: { kind: "hard_cutoff", threshold: 8.0, observed_floor: 8.04 },
      matched: 121,
      total: 149,
    },
    comparison: [
      [7.0, 0, 0.09, false],
      [7.5, 0, 0.21, false],
      [8.0, 0, 0.38, true],
      [8.5, 0.21, 0.61, false],
    ].map(([threshold, s, b, signal]) => ({
      threshold: threshold as number,
      share_below_shortlist: s as number,
      share_below_batch: b as number,
      is_signal: signal as boolean,
    })),
    statement:
      "Looks like a CGPA cutoff around 8.0, lowest found 8.04. An estimate, not an official cutoff.",
  },
  branches: {
    rows: {
      value: [
        ["CSE", 58, 0.48, 0.52],
        ["ECE", 31, 0.26, 0.12],
        ["IT", 17, 0.14, 0.15],
        ["EEE", 15, 0.12, 0.11],
      ].map(([branch, count, share, base]) => ({
        branch: branch as string,
        count: count as number,
        share: share as number,
        baseline_share: base as number,
        lift: Math.round(((share as number) / (base as number)) * 10) / 10,
      })),
      matched: 121,
      total: 149,
    },
    over_represented: ["ECE"],
    statement: "ECE is over-represented against the batch.",
  },
  your_percentile: { value: 0.38, matched: 121, total: 149 },
};

/** The bare outcome for a drive without an analysis of its own. */
function outcome(d: DriveListItem): ImportOutcome {
  return {
    drive_id: d.id,
    company: d.company,
    drive_date: null,
    total_students: d.total_students,
    shape: "neo_id_only",
    primary_key: "neo_id",
    you: person("You", profile.neo_id!, d.verdict),
    your_cgpa: profile.cgpa,
    evidence: {
      key: "neo_id",
      yours: profile.neo_id,
      listed: d.total_students,
      found_at: null,
      excerpt: [],
    },
    progression: null,
    friends: circle(d.id),
    analysis: {
      ...analysis,
      total_students: d.total_students,
      sufficient: false,
      matched_students: 0,
      coverage: 0,
    },
    learned_verified: 0,
    learned_named: 0,
    unreadable_headers: null,
  };
}

export function detail(id: number): ImportOutcome {
  const d = drives.find((x) => x.id === id) ?? drives[0]!;
  const base = outcome(d);
  if (id !== 1) return base;
  return {
    ...base,
    evidence: {
      key: "neo_id",
      yours: profile.neo_id,
      listed: 149,
      found_at: {
        kind: "neo_id",
        value: profile.neo_id!,
        sheet: "Shortlist",
        row: 42,
        column: "C",
        header: "Neo ID",
      },
      excerpt: [
        { row: 40, value: "H4J3V6N2" },
        { row: 41, value: "J2T0S6Y7" },
        { row: 42, value: profile.neo_id! },
        { row: 43, value: "D7J9E5J8" },
        { row: 44, value: "K8M2P4R6" },
      ],
    },
    progression: {
      steps: [
        { drive_id: 2, label: "Test", verdict: IN },
        { drive_id: 1, label: "Interview", verdict: IN },
      ],
      next_pending: true,
    },
    analysis,
  };
}

export const summary = [
  "*Siemens* · Interview",
  "149 shortlisted",
  "From 612 in Test",
  "",
  "CGPA cutoff ≈ 8.0 _(estimate)_",
  "More ECE than the batch",
].join("\n");
