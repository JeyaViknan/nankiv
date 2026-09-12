/**
 * Client state.
 *
 * Deliberately thin: it holds what is on screen and what the user is doing, and
 * nothing else. Every fact about students lives in the Rust core and is fetched
 * when needed, so the webview never accumulates a shadow copy of the data.
 */

import { create } from "zustand";
import {
  api,
  toApiError,
  type ApiError,
  type DriveRecord,
  type DriveSnapshot,
  type Friend,
  type ImportOutcome,
  type IdentityStats,
  type Profile,
} from "./api";

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
  openDrive: (id: number) => Promise<void>;
  renameDrive: (id: number, company: string) => Promise<void>;
  deleteDrive: (id: number, label: string) => Promise<void>;
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
