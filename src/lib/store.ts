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
  type Friend,
  type ImportOutcome,
  type IdentityStats,
  type Profile,
} from "./api";

export type Screen =
  | "home"
  | "drive"
  | "circle"
  | "search"
  | "history"
  | "settings"
  | "onboarding";

interface State {
  screen: Screen;
  profile: Profile | null;
  friends: Friend[];
  drives: DriveRecord[];
  stats: IdentityStats | null;
  current: ImportOutcome | null;
  importing: boolean;
  error: ApiError | null;
  toast: string | null;

  go: (screen: Screen) => void;
  bootstrap: () => Promise<void>;
  refreshFriends: () => Promise<void>;
  refreshDrives: () => Promise<void>;
  refreshStats: () => Promise<void>;
  saveProfile: (p: Profile) => Promise<boolean>;
  importFile: (path: string, replace?: boolean) => Promise<void>;
  openDrive: (id: number) => Promise<void>;
  clearError: () => void;
  showToast: (msg: string) => void;
}

export const useStore = create<State>((set, get) => ({
  screen: "home",
  profile: null,
  friends: [],
  drives: [],
  stats: null,
  current: null,
  importing: false,
  error: null,
  toast: null,

  go: (screen) => set({ screen, error: null }),

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
        // A student who hasn't told us who they are gets the setup flow first.
        screen: profile.neo_id || profile.reg_no ? "home" : "onboarding",
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

  importFile: async (path, replace = false) => {
    set({ importing: true, error: null });
    try {
      const outcome = await api.importShortlist(path, undefined, replace);
      set({ current: outcome, screen: "drive", importing: false });
      await Promise.all([get().refreshDrives(), get().refreshStats()]);
    } catch (e) {
      set({ error: toApiError(e), importing: false });
    }
  },

  openDrive: async (id) => {
    set({ importing: true, error: null });
    try {
      const outcome = await api.getDriveDetail(id);
      set({ current: outcome, screen: "drive", importing: false });
    } catch (e) {
      set({ error: toApiError(e), importing: false });
    }
  },

  clearError: () => set({ error: null }),
  showToast: (msg) => {
    set({ toast: msg });
    setTimeout(() => set({ toast: null }), 2600);
  },
}));
