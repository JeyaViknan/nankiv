/**
 * Client state.
 *
 * Deliberately thin: it holds what is on screen and what the user is doing, and
 * nothing else. Every fact about students lives in the Rust core and is fetched
 * when needed, so the webview never accumulates a shadow copy of the data.
 */

import { create } from "zustand";
import { save } from "@tauri-apps/plugin-dialog";
import {
  api,
  toApiError,
  type ApiError,
  type DriveRecord,
  type DriveSnapshot,
  type ExportFormat,
  type Friend,
  type ImportOutcome,
  type IdentityStats,
  type Profile,
  type Route,
} from "./api";
import { resolveRoute } from "./deeplinks";

/** A default filename that is legal on both macOS and Windows. */
export function exportFilename(company: string, format: ExportFormat): string {
  const safe = company
    .replace(/[\\/:*?"<>|]+/g, " ")
    .replace(/\s+/g, " ")
    .trim();
  return `${safe || "Shortlist"} shortlist.${format}`;
}

/** What the confirmation says, so a partly named list is never oversold. */
export function exportMessage(rows: number, named: number): string {
  const total = `${rows.toLocaleString()} ${rows === 1 ? "student" : "students"}`;
  if (named === rows) return `Saved ${total}, all named`;
  if (named === 0) return `Saved ${total} — none could be named yet`;
  return `Saved ${total} — ${named.toLocaleString()} named, the rest by ID`;
}

export type View = "shortlists" | "drive" | "onboarding";

/**
 * Where an import has got to.
 *
 * Modelled explicitly so the interface can narrate the sequence. The previous
 * build went straight from "spinner somewhere" to "different screen", which
 * read as a jump rather than as a response to what the student just did.
 */
export type ImportStage =
  | { phase: "idle" }
  | { phase: "reading"; filename: string }
  | { phase: "failed"; filename: string; error: ApiError };

interface State {
  view: View;
  profile: Profile | null;
  friends: Friend[];
  drives: DriveRecord[];
  stats: IdentityStats | null;
  current: ImportOutcome | null;
  importStage: ImportStage;
  error: ApiError | null;
  toast: { message: string; undo?: () => void } | null;

  go: (view: View) => void;
  bootstrap: () => Promise<void>;
  refreshFriends: () => Promise<void>;
  refreshDrives: () => Promise<void>;
  refreshStats: () => Promise<void>;
  saveProfile: (p: Profile) => Promise<boolean>;
  importFile: (path: string) => Promise<void>;
  importReferenceFile: (path: string) => Promise<void>;
  openDrive: (id: number) => Promise<void>;
  renameDrive: (id: number, company: string) => Promise<void>;
  deleteDrive: (id: number, label: string) => Promise<void>;
  exportNames: (format: ExportFormat) => Promise<void>;
  followRoute: (route: Route) => Promise<void>;
  openFiles: (paths: string[]) => Promise<void>;
  dismissImport: () => void;
  clearError: () => void;
  showToast: (message: string, undo?: () => void) => void;
  dismissToast: () => void;
}

let toastTimer: ReturnType<typeof setTimeout> | undefined;

export const useStore = create<State>((set, get) => ({
  view: "shortlists",
  profile: null,
  friends: [],
  drives: [],
  stats: null,
  current: null,
  importStage: { phase: "idle" },
  error: null,
  toast: null,

  go: (view) => set({ view, error: null }),

  bootstrap: async () => {
    try {
      const [profile, friends, drives, stats] = await Promise.all([
        api.getProfile(),
        api.listFriends(),
        api.listDrives(),
        api.identityStats(),
      ]);
      set({
        profile,
        friends,
        drives,
        stats,
        view: profile.neo_id || profile.reg_no ? "shortlists" : "onboarding",
      });
    } catch (e) {
      set({ error: toApiError(e) });
      return;
    }

    // A click on the desktop widget that launched the app is waiting here, as
    // is a shortlist dropped on the app icon. Both are collected only now, once
    // the drives are loaded, so a link to a drive can be checked against what
    // actually exists.
    try {
      const route = await api.takePendingRoute();
      if (route) await get().followRoute(route);
      const files = await api.takePendingFiles();
      if (files.length > 0) await get().openFiles(files);
    } catch {
      /* nothing was waiting, or it cannot be read — start normally */
    }
  },

  /**
   * Imports shortlists the desktop handed us, one after another.
   *
   * Sequential on purpose: each import replaces what is on screen, and two at
   * once would race for it.
   */
  openFiles: async (paths) => {
    void api.takePendingFiles().catch(() => {});
    if (get().view === "onboarding") return;
    for (const path of paths) {
      await get().importFile(path);
    }
  },

  /**
   * Opens whatever a `nankiv://` link points at.
   *
   * Drives are refreshed first: the widget can be a moment behind the app, and
   * a link must be checked against the drives that exist now, not the ones
   * that existed when the list was last loaded.
   */
  followRoute: async (route) => {
    // Clears the held copy, so a link that has been followed is never
    // followed again on the next launch.
    void api.takePendingRoute().catch(() => {});
    // Until someone has said who they are there is nothing a link could show
    // them, and skipping setup would leave every verdict undetermined.
    if (get().view === "onboarding") return;
    await get().refreshDrives();
    const action = resolveRoute(route, get().drives);
    if (action.kind === "open") {
      await get().openDrive(action.id);
    } else {
      set({ view: "shortlists", current: null });
      if (action.notice) get().showToast(action.notice);
    }
  },

  refreshFriends: async () => {
    try {
      set({ friends: await api.listFriends() });
    } catch (e) {
      set({ error: toApiError(e) });
    }
  },

  refreshDrives: async () => {
    try {
      set({ drives: await api.listDrives() });
    } catch (e) {
      set({ error: toApiError(e) });
    }
  },

  refreshStats: async () => {
    try {
      set({ stats: await api.identityStats() });
    } catch (e) {
      set({ error: toApiError(e) });
    }
  },

  saveProfile: async (p) => {
    try {
      await api.saveProfile(p);
      set({ profile: await api.getProfile(), error: null });
      return true;
    } catch (e) {
      set({ error: toApiError(e) });
      return false;
    }
  },

  importFile: async (path) => {
    const filename = path.split(/[\\/]/).pop() ?? "shortlist.xlsx";
    set({ importStage: { phase: "reading", filename }, error: null });
    try {
      const outcome = await api.importShortlist(path);
      set({
        current: outcome,
        view: "drive",
        importStage: { phase: "idle" },
      });
      await Promise.all([get().refreshDrives(), get().refreshStats()]);
    } catch (e) {
      const err = toApiError(e);

      // Dropping a reference sheet is a success, not a failure — the file was
      // the right kind of thing to give nankiv, just not a shortlist.
      if (err.code === "imported_reference") {
        set({ importStage: { phase: "idle" } });
        await Promise.all([get().refreshStats(), get().refreshDrives()]);
        const cur = get().current;
        if (cur) await get().openDrive(cur.drive_id);
        get().showToast(err.message);
        return;
      }

      // Re-dropping a file you already imported is ordinary behaviour, not a
      // failure. Open the drive that exists and say so quietly.
      if (err.code === "duplicate_drive" && err.detail) {
        const id = Number(err.detail);
        if (!Number.isNaN(id)) {
          set({ importStage: { phase: "idle" } });
          await get().openDrive(id);
          get().showToast("You already imported this — showing it");
          return;
        }
      }

      set({ importStage: { phase: "failed", filename, error: err } });
    }
  },

  /**
   * Ingests a reference sheet and re-runs whatever is on screen.
   *
   * Without the refresh the student adds their CGPA sheet, watches the toast
   * confirm it, and sees the same "not set up" panel — because the open drive
   * was analysed before the data existed.
   */
  importReferenceFile: async (path) => {
    const r = await api.importReference(path);
    await Promise.all([get().refreshStats(), get().refreshDrives()]);
    const cur = get().current;
    if (cur) await get().openDrive(cur.drive_id);
    get().showToast(
      r.kind === "academic"
        ? `Added academic records for ${r.academics_learned.toLocaleString()} students`
        : `Learned ${r.verified_links.toLocaleString()} identity links`,
    );
  },

  openDrive: async (id) => {
    try {
      const outcome = await api.getDriveDetail(id);
      set({ current: outcome, view: "drive", error: null });
    } catch (e) {
      set({ error: toApiError(e) });
    }
  },

  renameDrive: async (id, company) => {
    try {
      await api.renameDrive(id, company);
      const cur = get().current;
      if (cur && cur.drive_id === id) set({ current: { ...cur, company } });
      await get().refreshDrives();
    } catch (e) {
      get().showToast(toApiError(e).message);
    }
  },

  deleteDrive: async (id, label) => {
    try {
      const snapshot: DriveSnapshot | null = await api.deleteDrive(id);
      if (get().current?.drive_id === id) {
        set({ current: null, view: "shortlists" });
      }
      await get().refreshDrives();

      // Undo rather than a confirmation dialog: the common case costs nothing
      // and the rare mistake is still recoverable.
      get().showToast(
        `Removed ${label}`,
        snapshot
          ? async () => {
              try {
                await api.restoreDrive(snapshot);
                await get().refreshDrives();
                get().showToast(`Restored ${label}`);
              } catch (e) {
                get().showToast(toApiError(e).message);
              }
            }
          : undefined,
      );
    } catch (e) {
      get().showToast(toApiError(e).message);
    }
  },

  /**
   * Saves the names on the open shortlist.
   *
   * One implementation behind both the on-screen control and the menu bar, so
   * the two can never disagree about what gets written. The native save panel
   * chooses the location; dismissing it is a normal outcome and says nothing.
   */
  exportNames: async (format) => {
    const cur = get().current;
    if (!cur) {
      get().showToast("Open a shortlist to download its names");
      return;
    }
    let path: string | null;
    try {
      path = await save({
        defaultPath: exportFilename(cur.company, format),
        filters: [
          format === "xlsx"
            ? { name: "Excel Workbook", extensions: ["xlsx"] }
            : { name: "CSV", extensions: ["csv"] },
        ],
      });
    } catch {
      return;
    }
    if (typeof path !== "string") return;

    try {
      const r = await api.exportShortlist(cur.drive_id, path, format);
      get().showToast(exportMessage(r.rows, r.named));
    } catch (e) {
      get().showToast(toApiError(e).message);
    }
  },

  dismissImport: () => set({ importStage: { phase: "idle" } }),
  clearError: () => set({ error: null }),

  showToast: (message, undo) => {
    if (toastTimer) clearTimeout(toastTimer);
    set({ toast: { message, undo } });
    // Undoable toasts linger, because acting on one takes a decision.
    toastTimer = setTimeout(() => set({ toast: null }), undo ? 8000 : 2800);
  },

  dismissToast: () => {
    if (toastTimer) clearTimeout(toastTimer);
    set({ toast: null });
  },
}));
