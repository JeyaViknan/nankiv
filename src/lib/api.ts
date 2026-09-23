/**
 * Typed bridge to the Rust core.
 *
 * Every type here mirrors a `serde` shape on the other side. The discriminated
 * unions are deliberate: `Verdict` cannot be treated as a boolean, so the
 * interface is forced to handle "undetermined" as its own case rather than
 * letting it fall through to "not shortlisted".
 */

import { invoke } from "@tauri-apps/api/core";

// --- Verdict -----------------------------------------------------------------

export type UndeterminedReason =
  | { reason: "no_identity_configured" }
  | { reason: "key_kind_not_configured"; file_key: KeyKind }
  | { reason: "file_not_understood" };

export type Verdict =
  | { status: "shortlisted" }
  | { status: "not_shortlisted" }
  | ({ status: "undetermined" } & UndeterminedReason);

export type KeyKind = "neo_id" | "reg_no";
export type Confidence = "verified" | "high" | "probable" | "unresolved";
export type FileShape =
  "neo_id_only" | "reg_no_only" | "linked" | "unrecognised";

/** True only for a definite yes. Never use for styling the negative case. */
export function isShortlisted(v: Verdict): boolean {
  return v.status === "shortlisted";
}

/**
 * True when we genuinely know the answer is no.
 *
 * Kept separate from `!isShortlisted` on purpose — that expression is true for
 * undetermined results too, and conflating them is the failure mode this whole
 * application is built to avoid.
 */
export function isDefiniteNo(v: Verdict): boolean {
  return v.status === "not_shortlisted";
}

export function isUndetermined(v: Verdict): boolean {
  return v.status === "undetermined";
}

// --- Records -----------------------------------------------------------------

export interface Profile {
  neo_id: string | null;
  reg_no: string | null;
  display_name: string | null;
  cohort: string | null;
  show_friend_cgpa: boolean;
}

export interface Friend {
  id: number;
  label: string;
  neo_id: string | null;
  reg_no: string | null;
  group_tag: string | null;
}

export interface PersonResult {
  label: string;
  neo_id: string | null;
  reg_no: string | null;
  verdict: Verdict;
  confidence: Confidence;
  cgpa: number | null;
}

export interface DriveRecord {
  id: number;
  company: string;
  drive_date: string | null;
  imported_at: string;
  source_filename: string;
  content_hash: string;
  shape: string;
  primary_key: string | null;
  total_students: number;
  round_label: string | null;
  parent_drive_id: number | null;
}

// --- Analytics ---------------------------------------------------------------

export interface Estimate<T> {
  value: T;
  matched: number;
  total: number;
}

export interface Bucket {
  lower: number;
  upper: number;
  count: number;
}

export interface Distribution {
  n: number;
  min: number;
  max: number;
  median: number;
  mean: number;
  std_dev: number;
  p5: number;
  p25: number;
  p75: number;
  buckets: Bucket[];
}

export type CutoffVerdict =
  | { kind: "hard_cutoff"; threshold: number; observed_floor: number }
  | { kind: "soft_preference"; observed_floor: number }
  | { kind: "no_cgpa_filter" };

export interface ThresholdRow {
  threshold: number;
  share_below_shortlist: number;
  share_below_batch: number;
  is_signal: boolean;
}

export interface CutoffReport {
  verdict: Estimate<CutoffVerdict>;
  comparison: ThresholdRow[];
  statement: string;
}

export interface BranchLift {
  branch: string;
  count: number;
  share: number;
  baseline_share: number | null;
  lift: number | null;
}

export interface BranchReport {
  rows: Estimate<BranchLift[]>;
  over_represented: string[];
  statement: string;
}

export interface DriveAnalysis {
  total_students: number;
  matched_students: number;
  coverage: number;
  sufficient: boolean;
  cgpa: Estimate<Distribution> | null;
  cutoff: CutoffReport | null;
  branches: BranchReport | null;
  your_percentile: Estimate<number> | null;
}

export interface ImportOutcome {
  drive_id: number;
  company: string;
  drive_date: string | null;
  total_students: number;
  shape: FileShape;
  primary_key: KeyKind | null;
  you: PersonResult;
  friends: PersonResult[];
  analysis: DriveAnalysis;
  learned_verified: number;
  learned_named: number;
  unreadable_headers: string[] | null;
}

/** How the season is going: one verdict per drive, counted by the core. */
export interface Season {
  drives: number;
  shortlisted: number;
  not_shortlisted: number;
  undetermined: number;
}

export interface IdentityStats {
  students_known: number;
  names_known: number;
  academics_known: number;
  edges: number;
  conflicts: number;
}

export type SearchResult =
  | { kind: "found"; neo_id: string; name: string; confidence: Confidence }
  | { kind: "ambiguous"; candidates: { neo_id: string; name: string }[] }
  | { kind: "not_found" };

export interface RoundDiff {
  from_company: string;
  to_company: string;
  advanced: number;
  dropped: number;
  added: number;
  advanced_friends: string[];
  dropped_friends: string[];
}

/** A deleted drive, held just long enough to undo. */
export interface DriveSnapshot {
  company: string;
  drive_date: string | null;
  source_filename: string;
  content_hash: string;
  shape: string;
  primary_key: string | null;
  round_label: string | null;
  neo_ids: string[];
  reg_nos: string[];
}

export interface ReferenceImportResult {
  students_learned: number;
  academics_learned: number;
  verified_links: number;
  kind: string;
}

/** Error shape returned across the IPC boundary. */
export interface ApiError {
  code: string;
  message: string;
  detail: string | null;
}

export function isApiError(e: unknown): e is ApiError {
  return typeof e === "object" && e !== null && "code" in e && "message" in e;
}

/** Normalises anything thrown by `invoke` into a displayable error. */
export function toApiError(e: unknown): ApiError {
  if (isApiError(e)) return e;
  return {
    code: "unknown",
    message: typeof e === "string" ? e : "Something went wrong.",
    detail: null,
  };
}

// --- Commands ----------------------------------------------------------------

export const api = {
  getProfile: () => invoke<Profile>("get_profile"),
  saveProfile: (profile: Profile) => invoke<void>("save_profile", { profile }),

  listFriends: () => invoke<Friend[]>("list_friends"),
  addFriend: (
    label: string,
    neoId: string | null,
    regNo: string | null,
    groupTag: string | null,
  ) =>
    invoke<number>("add_friend", {
      label,
      neoId,
      regNo,
      groupTag,
    }),
  removeFriend: (id: number) => invoke<void>("remove_friend", { id }),

  importShortlist: (
    path: string,
    companyOverride?: string,
    replaceExisting?: boolean,
  ) =>
    invoke<ImportOutcome>("import_shortlist", {
      path,
      companyOverride: companyOverride ?? null,
      replaceExisting: replaceExisting ?? false,
    }),
  importReference: (path: string) =>
    invoke<ReferenceImportResult>("import_reference", { path }),

  listDrives: () => invoke<DriveRecord[]>("list_drives"),
  deleteDrive: (id: number) =>
    invoke<DriveSnapshot | null>("delete_drive", { id }),
  restoreDrive: (snapshot: DriveSnapshot) =>
    invoke<number>("restore_drive", { snapshot }),
  renameDrive: (id: number, company: string) =>
    invoke<void>("rename_drive", { id, company }),
  setDriveRound: (id: number, parentId: number | null) =>
    invoke<void>("set_drive_round", { id, parentId }),
  getDriveDetail: (id: number) =>
    invoke<ImportOutcome>("get_drive_detail", { id }),

  lookupIdentifier: (query: string) =>
    invoke<PersonResult[]>("lookup_identifier", { query }),
  searchStudents: (query: string) =>
    invoke<SearchResult>("search_students", { query }),
  compareRounds: (fromId: number, toId: number) =>
    invoke<RoundDiff>("compare_rounds", { fromId, toId }),

  identityStats: () => invoke<IdentityStats>("identity_stats"),
  dataInventory: () => invoke<[string, number][]>("data_inventory"),
  wipeAllData: () => invoke<void>("wipe_all_data"),
  shareSummary: (driveId: number) =>
    invoke<string>("share_summary", { driveId }),
  takePendingRoute: () => invoke<Route | null>("take_pending_route"),
  takePendingFiles: () => invoke<string[]>("take_pending_files"),
  season: () => invoke<Season>("season"),
  /** A single trackpad tap. Silent on a mouse, and where the system says so. */
  tap: () => invoke<void>("tap"),
  /**
   * Hands the core the bytes of a file dropped on the window, and gets back a
   * path the ordinary import can read. A web view is never told where a
   * dropped file lives on disk, so this is the only way in.
   */
  stageDroppedFile: (name: string, bytes: ArrayBuffer) =>
    invoke<string>("stage_dropped_file", bytes, {
      headers: { "x-filename": encodeURIComponent(name) },
    }),
  exportShortlist: (driveId: number, path: string, format: ExportFormat) =>
    invoke<ExportSummary>("export_shortlist", { driveId, path, format }),
};

/** `xlsx` is the Excel format every current spreadsheet app opens natively. */
export type ExportFormat = "xlsx" | "csv";

export interface ExportSummary {
  path: string;
  rows: number;
  named: number;
}

/**
 * A `nankiv://` destination. Navigation only: no route performs an action,
 * because any app or web page can open a URL scheme.
 */
export type Route =
  { kind: "drive"; id: number } | { kind: "latest" } | { kind: "shortlists" };
