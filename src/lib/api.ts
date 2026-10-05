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
  /** Your own CGPA, as you typed it — the only CGPA nankiv ever shows. */
  cgpa: number | null;
  cgpa_updated_at: string | null;
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
}

/** Where one identifier sat in the file its drive came from. */
export interface MemberOrigin {
  kind: "neo_id" | "reg_no";
  value: string;
  sheet: string;
  /** 1-based, as the spreadsheet numbers it. */
  row: number;
  column: string | null;
  header: string | null;
}

export interface Evidence {
  /** What the file lists students by. */
  key: KeyKind | null;
  /** Your identifier of that kind, as you entered it. */
  yours: string | null;
  /** How many identifiers of that kind the file lists. */
  listed: number;
  /** Where yours appears; absent for drives imported before this was kept. */
  found_at: MemberOrigin | null;
}

export interface RoundStep {
  drive_id: number;
  /** "R1", "R2", "Final" — the file's word for it, or its position. */
  label: string;
  verdict: Verdict;
}

export interface Progression {
  steps: RoundStep[];
  /** You're in the latest round, and nothing says it was the last. */
  next_pending: boolean;
}

/** What a paste would import, shown before it does. */
export interface PastePreview {
  neo_ids: number;
  reg_nos: number;
  /** Lines of words with no identifier on them, which will be skipped. */
  unread_lines: number;
  /** A name read from the words around the identifiers, if any named one. */
  company: string | null;
}

/** A spreadsheet in the Downloads folder, offered for import. */
export interface RecentDownload {
  path: string;
  name: string;
  /** RFC 3339. */
  modified: string;
  /** A drive already came from a file of this name. */
  imported: boolean;
}

export interface AppVersion {
  version: string;
  /** When this copy was built, RFC 3339. */
  built: string | null;
}

export type UpdateCheck =
  | { status: "up_to_date" }
  | { status: "available"; version: string; notes: string | null }
  /** No release has published update information yet. Not a failure. */
  | { status: "unpublished" };

/** A drive as the list shows it: with your answer, and its round. */
export interface DriveListItem extends DriveRecord {
  verdict: Verdict;
  /** Its place among linked rounds, or the stage its file named. */
  round: string | null;
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
  your_cgpa: number | null;
  /** What your answer rests on, for "Why does it think I'm in?". */
  evidence: Evidence;
  /** This drive's rounds, when it is one of several. */
  progression: Progression | null;
  friends: PersonResult[];
  analysis: DriveAnalysis;
  learned_verified: number;
  learned_named: number;
  unreadable_headers: string[] | null;
}

/** The nine weights of San Francisco, which SF Symbols share. */
export type SymbolWeight =
  | "ultralight"
  | "thin"
  | "light"
  | "regular"
  | "medium"
  | "semibold"
  | "bold"
  | "heavy"
  | "black";

export interface SymbolRequest {
  /** As the SF Symbols app shows it, e.g. `gearshape`. */
  name: string;
  pointSize: number;
  weight: SymbolWeight;
  /** Device pixels per point. */
  scale: number;
}

/** A symbol macOS drew, as a mask sized in points. */
export interface SymbolImage {
  width: number;
  height: number;
  url: string;
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

  listDrives: () => invoke<DriveListItem[]>("list_drives"),
  /** Reads a pasted list without storing anything. */
  previewPaste: (text: string) =>
    invoke<PastePreview>("preview_paste", { text }),
  importPasted: (text: string, company: string | null) =>
    invoke<ImportOutcome>("import_pasted", { text, company }),
  /** Newest spreadsheets in Downloads. Asks for access on first use. */
  recentDownloads: () => invoke<RecentDownload[]>("recent_downloads"),
  appVersion: () => invoke<AppVersion>("app_version"),
  /** The app's one network request, made only when the student asks. */
  checkForUpdate: () => invoke<UpdateCheck>("check_for_update"),
  /** Downloads, verifies and installs, then restarts into the new version. */
  installUpdate: () => invoke<void>("install_update"),
  /** The releases page, opened in the browser by the core. */
  openReleasesPage: () => invoke<void>("open_releases_page"),
  watchDownloads: () => invoke<boolean>("watch_downloads"),
  /** Turning it on reads Downloads once, so any permission prompt follows. */
  setWatchDownloads: (on: boolean) =>
    invoke<void>("set_watch_downloads", { on }),
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
  /** SF Symbols from macOS, `null` for any the system cannot supply. */
  symbols: (requests: SymbolRequest[]) =>
    invoke<(SymbolImage | null)[]>("symbols", { requests }),
  /**
   * Hands the core the bytes of a file dropped on the window, and gets back a
   * path the ordinary import can read. A web view is never told where a
   * dropped file lives on disk, so this is the only way in.
   */
  stageDroppedFile: (name: string, bytes: ArrayBuffer) =>
    invoke<string>("stage_dropped_file", bytes, {
      headers: { "x-filename": encodeURIComponent(name) },
    }),
  /** Writes a PNG card to a path chosen in the native save panel. */
  saveShareCard: (path: string, png: ArrayBuffer) =>
    invoke<void>("save_share_card", png, {
      headers: { "x-path": encodeURIComponent(path) },
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
