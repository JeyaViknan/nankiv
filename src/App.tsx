/**
 * The shell.
 *
 * The sidebar is gone. It listed five destinations — Home, Circle, Search,
 * History, Settings — which was the database schema wearing a navigation bar.
 * The student's model is "a file arrives, I need to know, sometimes I look
 * back": one recurring act and a few occasional side tasks.
 *
 * So: one surface (Shortlists, which is both the drop target and the history),
 * one detail view (a drive), and two sheets (Circle, Settings). Search is a
 * field that is always visible rather than a destination or a hidden shortcut —
 * discoverable *and* fast.
 *
 * The whole window is the drop target, handled once here, so every view accepts
 * a file without knowing anything about dragging.
 */

import { useCallback, useEffect, useRef, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { useShortcuts } from "./lib/shortcuts";
import { useDeepLinks, useOpenedFiles } from "./lib/deeplinks";
import { useStore } from "./lib/store";
import { applyTheme, getThemeChoice, watchSystemTheme } from "./lib/theme";
import { DropSurface } from "./components/DropSurface";
import { SeasonStrip } from "./components/SeasonStrip";
import { Icon } from "./components/Icon";
import { SearchField } from "./components/SearchField";
import { Toast } from "./components/Toast";
import { CircleSheet } from "./screens/CircleSheet";
import { DriveScreen } from "./screens/DriveScreen";
import { OnboardingScreen } from "./screens/Onboarding";
import { SettingsSheet } from "./screens/Settings";
import { Shortlists } from "./screens/Shortlists";

export default function App() {
  const {
    view,
    go,
    bootstrap,
    current,
    error,
    clearError,
    importFile,
    importStage,
    season,
    friends,
  } = useStore();

  const [circleOpen, setCircleOpen] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const searchRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    void bootstrap();
  }, [bootstrap]);

  // Follow the operating system live, but only while the choice is "System".
  useEffect(() => {
    return watchSystemTheme(() => {
      if (getThemeChoice() === "system") applyTheme("system");
    });
  }, []);

  const browse = useCallback(async () => {
    try {
      const picked = await open({
        multiple: false,
        filters: [
          {
            name: "Spreadsheet",
            extensions: ["xlsx", "xls", "xlsm", "ods", "csv"],
          },
        ],
      });
      if (typeof picked === "string") void importFile(picked);
    } catch {
      /* the dialog was dismissed */
    }
  }, [importFile]);

  const browseReference = useCallback(async () => {
    try {
      const picked = await open({
        multiple: false,
        filters: [
          {
            name: "Spreadsheet",
            extensions: ["xlsx", "xls", "xlsm", "ods", "csv"],
          },
        ],
      });
      // Import routes on content, so a reference sheet and a shortlist go
      // through the same door and land in the right place either way.
      if (typeof picked === "string") void importFile(picked);
    } catch {
      /* the dialog was dismissed */
    }
  }, [importFile]);

  const goBack = useCallback(() => go("shortlists"), [go]);

  // A click on the desktop widget opens the shortlist it was showing. Links
  // that launch the app are picked up during bootstrap instead.
  useDeepLinks((route) => void useStore.getState().followRoute(route));

  // Dropping a shortlist on the app icon imports it, from anywhere in the
  // system — the Dock, Finder, Open With.
  useOpenedFiles((paths) => void useStore.getState().openFiles(paths));

  // One implementation per shortcut, reachable from the menu bar and the
  // keyboard alike. The hook guarantees each runs once per press, however many
  // of those paths deliver it on the platform the app happens to be running on.
  useShortcuts({
    settings: () => setSettingsOpen((v) => !v),
    circle: () => setCircleOpen((v) => !v),
    open: () => void browse(),
    open_reference: () => void browseReference(),
    search: () => {
      searchRef.current?.focus();
      searchRef.current?.select();
    },
    back: goBack,
    export_xlsx: () => void useStore.getState().exportNames("xlsx"),
    export_csv: () => void useStore.getState().exportNames("csv"),
  });

  // Escape is not a menu accelerator, so it has only the one path and needs no
  // deduplication. Sheets handle their own Escape; this one is the view stack.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== "Escape") return;
      const typing =
        e.target instanceof HTMLElement &&
        (e.target.tagName === "INPUT" || e.target.tagName === "TEXTAREA");
      if (!typing && !circleOpen && !settingsOpen && view === "drive") {
        e.preventDefault();
        goBack();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [goBack, view, circleOpen, settingsOpen]);

  if (view === "onboarding") {
    return (
      <div className="app">
        <OnboardingScreen />
        <Toast />
      </div>
    );
  }

  const inDrive = view === "drive" && current;

  return (
    <div className="app">
      {/* The toolbar doubles as the window drag region, which is what makes a
          chrome-less window feel native rather than like a page. */}
      <header className="toolbar" data-tauri-drag-region>
        <div className="toolbar-left">
          {inDrive ? (
            <button
              className="icon-btn"
              onClick={goBack}
              aria-label="Back to shortlists"
              title="Back (Esc)"
            >
              <Icon name="back" size={16} />
            </button>
          ) : (
            <SeasonStrip season={season} />
          )}
        </div>

        <SearchField inputRef={searchRef} />

        <div className="toolbar-right">
          {/* The daily action, in the same place on every screen — including
              the drive view, where there was previously no way to import at
              all without going back first. */}
          <button
            className="icon-btn primary"
            onClick={() => void browse()}
            aria-label="Import a shortlist"
            title="Import a shortlist (⌘O)"
          >
            <Icon name="plus" size={17} />
          </button>
          <button
            className="icon-btn"
            onClick={() => setCircleOpen(true)}
            aria-label={`Circle, ${friends.length} people`}
            title="Circle (⌘D)"
          >
            <Icon name="people" size={17} />
            {friends.length > 0 && (
              <span className="icon-badge">{friends.length}</span>
            )}
          </button>
          <button
            className="icon-btn"
            onClick={() => setSettingsOpen(true)}
            aria-label="Settings"
            title="Settings (⌘,)"
          >
            <Icon name="gear" size={17} />
          </button>
        </div>
      </header>

      <main className="content">
        {error && (
          <div className="banner" role="alert">
            <span>{error.message}</span>
            <button className="btn small" onClick={clearError}>
              Dismiss
            </button>
          </div>
        )}

        {inDrive ? (
          <div className="view-drive" key={current.drive_id}>
            <DriveScreen
              outcome={current}
              onFix={() => setSettingsOpen(true)}
            />
          </div>
        ) : (
          <div className="view-root">
            <Shortlists onBrowse={browse} />
          </div>
        )}
      </main>

      <DropSurface
        onFile={(file) => void useStore.getState().importDropped(file)}
        disabled={importStage.phase === "reading"}
      />

      {circleOpen && <CircleSheet onClose={() => setCircleOpen(false)} />}
      {settingsOpen && <SettingsSheet onClose={() => setSettingsOpen(false)} />}

      <Toast />
    </div>
  );
}
