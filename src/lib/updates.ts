/**
 * Keeping nankiv current, without it phoning home.
 *
 * Nothing here touches the network. The reminder comes from how old the
 * installed build is: past six weeks, nankiv says so once, quietly, and then
 * not again for a fortnight. Checking is always the student's click.
 */

import { useEffect } from "react";
import { api } from "./api";
import { useStore } from "./store";

export const RELEASES_URL = "https://github.com/JeyaViknan/nankiv/releases";

const DAY = 86_400_000;
const STALE_AFTER = 42 * DAY;
const QUIET_FOR = 14 * DAY;
const REMINDED_KEY = "nankiv.update-reminded";

/** Whether a build from `built` is old enough to mention, given the last time. */
export function shouldRemind(
  built: string | null,
  lastReminded: string | null,
  now: Date = new Date(),
): boolean {
  if (!built) return false;
  const age = now.getTime() - new Date(built).getTime();
  if (Number.isNaN(age) || age < STALE_AFTER) return false;
  const since = lastReminded
    ? now.getTime() - new Date(lastReminded).getTime()
    : Infinity;
  return !(since < QUIET_FOR);
}

/** "5 October 2026". */
export function builtLabel(built: string | null): string | null {
  if (!built) return null;
  const d = new Date(built);
  return Number.isNaN(d.getTime())
    ? null
    : d.toLocaleDateString(undefined, {
        day: "numeric",
        month: "long",
        year: "numeric",
      });
}

function readReminded(): string | null {
  try {
    return localStorage.getItem(REMINDED_KEY);
  } catch {
    return null;
  }
}

function markReminded(now: Date): void {
  try {
    localStorage.setItem(REMINDED_KEY, now.toISOString());
  } catch {
    // Without storage it may remind again next launch; that is all.
  }
}

/** Once per launch, an offline nudge when the installed build has aged. */
export function useUpdateReminder(openUpdates: () => void): void {
  useEffect(() => {
    let live = true;
    api
      .appVersion()
      .then(({ version, built }) => {
        const now = new Date();
        if (!live || !shouldRemind(built, readReminded(), now)) return;
        markReminded(now);
        useStore
          .getState()
          .showToast(
            `You're on nankiv ${version} — a newer version may be out`,
            openUpdates,
            "Check",
          );
      })
      .catch(() => {});
    return () => {
      live = false;
    };
    // Once per launch, by design.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);
}
