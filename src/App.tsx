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
import { useStore } from "./lib/store";
import { applyTheme, getThemeChoice, watchSystemTheme } from "./lib/theme";
import { DropSurface } from "./components/DropSurface";
import { SearchField } from "./components/SearchField";
import { Toast } from "./components/Toast";
import { CircleSheet } from "./screens/CircleSheet";
import { DriveScreen } from "./screens/DriveScreen";
import { OnboardingScreen } from "./screens/Onboarding";
import { SettingsSheet } from "./screens/Settings";
import { Shortlists } from "./screens/Shortlists";

function BackIcon() {
  return (
    <svg
      width="16"
      height="16"
      viewBox="0 0 16 16"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.7"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <path d="M10 3l-5 5 5 5" />
    </svg>
  );
}

function PeopleIcon() {
  return (
    <svg
      width="16"
      height="16"
      viewBox="0 0 20 20"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.5"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <circle cx="7.4" cy="6.6" r="2.6" />
      <path d="M2.3 16c0-2.6 2.3-4.2 5.1-4.2s5.1 1.6 5.1 4.2" />
      <path d="M13.4 5.1a2.1 2.1 0 0 1 0 4.1M14.2 11.9c2.3.3 3.5 1.6 3.5 4.1" />
    </svg>
  );
}

function GearIcon() {
  return (
    <svg
      width="16"
      height="16"
      viewBox="0 0 20 20"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.5"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <circle cx="10" cy="10" r="2.4" />
      <path d="M10 2.6l.9 1.9 2.1-.3.5 2 1.9.9-.9 1.9.9 1.9-1.9.9-.5 2-2.1-.3-.9 1.9-.9-1.9-2.1.3-.5-2-1.9-.9.9-1.9-.9-1.9 1.9-.9.5-2 2.1.3z" />
    </svg>
  );
}

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

  const goBack = useCallback(() => go("shortlists"), [go]);

  // The desktop keyboard model. A tool used several times a day should be
  // operable without reaching for the mouse.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const meta = e.metaKey || e.ctrlKey;
      const typing =
        e.target instanceof HTMLElement &&
        (e.target.tagName === "INPUT" || e.target.tagName === "TEXTAREA");

      if (meta && e.key === ",") {
        e.preventDefault();
        setSettingsOpen((v) => !v);
      } else if (meta && e.key.toLowerCase() === "o") {
        e.preventDefault();
        void browse();
      } else if (meta && e.key.toLowerCase() === "f") {
        e.preventDefault();
        searchRef.current?.focus();
        searchRef.current?.select();
      } else if (meta && e.key.toLowerCase() === "d") {
        e.preventDefault();
        setCircleOpen((v) => !v);
      } else if (e.key === "Escape" && !typing) {
        // Sheets handle their own Escape; this is the view stack.
        if (!circleOpen && !settingsOpen && view === "drive") {
          e.preventDefault();
          goBack();
        }
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [browse, goBack, view, circleOpen, settingsOpen]);

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
              <BackIcon />
            </button>
          ) : (
            <span className="wordmark">nankiv</span>
          )}
        </div>

        <SearchField inputRef={searchRef} />

        <div className="toolbar-right">
          <button
            className="icon-btn"
            onClick={() => setCircleOpen(true)}
            aria-label="Circle"
            title="Circle (⌘D)"
          >
            <PeopleIcon />
          </button>
          <button
            className="icon-btn"
            onClick={() => setSettingsOpen(true)}
            aria-label="Settings"
            title="Settings (⌘,)"
          >
            <GearIcon />
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
          <DriveScreen outcome={current} />
        ) : (
          <Shortlists onBrowse={browse} />
        )}
      </main>

      <DropSurface
        onFile={(p) => void importFile(p)}
        disabled={importStage.phase === "reading"}
      />

      {circleOpen && <CircleSheet onClose={() => setCircleOpen(false)} />}
      {settingsOpen && <SettingsSheet onClose={() => setSettingsOpen(false)} />}

      <Toast />
    </div>
  );
}
