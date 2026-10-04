/**
 * A shortlist the Downloads watcher imported on its own.
 *
 * The student did not ask for this one, so it does not take over the screen:
 * the list refreshes, a toast says what arrived and how it went, and Open is
 * one click away. The widget already shows it.
 */

import type { ImportOutcome, Verdict } from "./api";
import { useTauriEvent } from "./deeplinks";
import { useStore } from "./store";

export function arrivalMessage(company: string, verdict: Verdict): string {
  switch (verdict.status) {
    case "shortlisted":
      return `${company} came in from Downloads — you're in`;
    case "not_shortlisted":
      return `${company} came in from Downloads — not this time`;
    case "undetermined":
      return `${company} came in from Downloads`;
  }
}

export function useWatchedImports(): void {
  useTauriEvent<ImportOutcome>("watched-import", (outcome) => {
    const { refreshDrives, refreshStats, showToast, openDrive } =
      useStore.getState();
    void Promise.all([refreshDrives(), refreshStats()]);
    showToast(
      arrivalMessage(outcome.company, outcome.you.verdict),
      () => void openDrive(outcome.drive_id),
      "Open",
    );
  });
}
