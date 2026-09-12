/**
 * Application shell.
 *
 * The sidebar lists things you *do*. Settings is not one of them — it is
 * configured once and then left alone, so it lives behind Cmd+, as an overlay
 * instead of taking up a permanent slot in the navigation.
 */

import { useCallback, useEffect, useState } from "react";
import { useStore } from "./lib/store";
import { getThemeChoice, watchSystemTheme, applyTheme } from "./lib/theme";
import { DriveScreen } from "./screens/DriveScreen";
import { SettingsPanel } from "./screens/Settings";
import {
  CircleScreen,
  HistoryScreen,
  HomeScreen,
  OnboardingScreen,
  SearchScreen,
} from "./screens/Screens";

type IconName = "home" | "circle" | "search" | "history";

function NavIcon({ name }: { name: IconName }) {
  const d: Record<IconName, string> = {
    home: "M3 8.6L10 3.2l7 5.4V16a1.2 1.2 0 0 1-1.2 1.2h-2.9v-4.6H7.1v4.6H4.2A1.2 1.2 0 0 1 3 16V8.6z",
    circle:
      "M7 9.2a2.6 2.6 0 1 0 0-5.2 2.6 2.6 0 0 0 0 5.2zM13.6 9a2.1 2.1 0 1 0 0-4.2 2.1 2.1 0 0 0 0 4.2zM1.8 16.4c0-2.6 2.3-4.2 5.2-4.2s5.2 1.6 5.2 4.2M13.4 12.3c2.6.2 5 1.5 5 4.1",
    search: "M8.8 15.1a6.3 6.3 0 1 0 0-12.6 6.3 6.3 0 0 0 0 12.6zm4.6-1.6l4 4",
    history: "M3.2 10a6.8 6.8 0 1 0 2-4.8M3.2 3.8v3.4h3.4M10 6.2V10l2.6 1.6",
  };
  return (
    <svg
      width="17"
      height="17"
      viewBox="0 0 20 20"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.55"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <path d={d[name]} />
    </svg>
  );
}

export default function App() {
  const {
    screen,
    go,
    bootstrap,
    current,
    drives,
    friends,
    error,
    clearError,
    toast,
    importing,
    openDrive,
  } = useStore();

  const [settingsOpen, setSettingsOpen] = useState(false);

  useEffect(() => {
    void bootstrap();
  }, [bootstrap]);

  // Follow the operating system live, but only while the user is on "System".
  useEffect(() => {
    return watchSystemTheme(() => {
      if (getThemeChoice() === "system") applyTheme("system");
    });
  }, []);

  // Cmd+, on macOS, Ctrl+, elsewhere — the platform convention for preferences.
  const toggleSettings = useCallback(() => setSettingsOpen((v) => !v), []);
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === ",") {
        e.preventDefault();
        toggleSettings();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [toggleSettings]);

  if (screen === "onboarding") {
    return (
      <div className="main">
        <OnboardingScreen />
        {toast && (
          <div className="toast" role="status">
            {toast}
          </div>
        )}
      </div>
    );
  }

  const nav: { id: IconName; label: string; count?: number }[] = [
    { id: "home", label: "Home" },
    { id: "circle", label: "Circle", count: friends.length },
    { id: "search", label: "Search" },
    { id: "history", label: "History", count: drives.length },
  ];

  return (
    <div className="shell">
      <nav className="sidebar" aria-label="Main">
        <div className="brand">
          <span className="brand-mark">
            <svg
              width="13"
              height="13"
              viewBox="0 0 24 24"
              fill="none"
              aria-hidden="true"
            >
              <path
                d="M5 12.5l4.5 4.5L19 7"
                stroke="white"
                strokeWidth="3.2"
                strokeLinecap="round"
                strokeLinejoin="round"
              />
            </svg>
          </span>
          <span className="brand-name">nankiv</span>
        </div>

        {nav.map((item) => (
          <button
            key={item.id}
            className="nav-item"
            aria-current={screen === item.id ? "page" : undefined}
            onClick={() => go(item.id)}
          >
            <NavIcon name={item.id} />
            {item.label}
            {item.count !== undefined && item.count > 0 && (
              <span className="count">{item.count}</span>
            )}
          </button>
        ))}

        <div className="sidebar-foot">
          <span className="offline-badge">
            <span className="offline-dot" />
            Works offline
          </span>
          <span className="kbd-hint">
            <kbd>⌘</kbd>
            <kbd>,</kbd>
            <span style={{ marginLeft: 2 }}>Settings</span>
          </span>
        </div>
      </nav>

      <main className="main">
        {error && (
          <div className="notice danger">
            <strong>{error.message}</strong>
            {error.detail && (
              <p style={{ margin: "6px 0 0", fontSize: 12.5 }}>
                {error.detail}
              </p>
            )}
            <div className="btn-row" style={{ marginTop: 11 }}>
              <button className="btn small" onClick={clearError}>
                Dismiss
              </button>
              {error.code === "duplicate_drive" && error.detail && (
                <button
                  className="btn small"
                  onClick={() => {
                    const id = Number(error.detail);
                    clearError();
                    if (!Number.isNaN(id)) void openDrive(id);
                  }}
                >
                  Open the one you already have
                </button>
              )}
            </div>
          </div>
        )}

        {screen === "home" && <HomeScreen />}
        {screen === "drive" &&
          (current ? (
            <DriveScreen outcome={current} />
          ) : importing ? (
            <div className="empty">
              <span className="spinner" />
            </div>
          ) : (
            <HomeScreen />
          ))}
        {screen === "circle" && <CircleScreen />}
        {screen === "search" && <SearchScreen />}
        {screen === "history" && <HistoryScreen />}
      </main>

      {settingsOpen && <SettingsPanel onClose={() => setSettingsOpen(false)} />}

      {toast && (
        <div className="toast" role="status">
          {toast}
        </div>
      )}
    </div>
  );
}
