/**
 * Application shell.
 *
 * A persistent sidebar and one screen at a time. The drop target lives on the
 * home screen and the window accepts a dropped file from anywhere, because
 * importing is the only thing a student does every day.
 */

import { useEffect } from "react";
import { useStore } from "./lib/store";
import { DriveScreen } from "./screens/DriveScreen";
import {
  CircleScreen,
  HistoryScreen,
  HomeScreen,
  OnboardingScreen,
  SearchScreen,
  SettingsScreen,
} from "./screens/Screens";

function NavIcon({ name }: { name: string }) {
  const paths: Record<string, JSX.Element> = {
    home: (
      <path d="M3 9.5L10 4l7 5.5V16a1 1 0 0 1-1 1h-3v-4H7v4H4a1 1 0 0 1-1-1V9.5z" />
    ),
    circle: (
      <path d="M7 9a2.5 2.5 0 1 0 0-5 2.5 2.5 0 0 0 0 5zm6.5 0a2 2 0 1 0 0-4 2 2 0 0 0 0 4zM2 16c0-2.5 2.2-4 5-4s5 1.5 5 4M13 12c2.5.2 5 1.4 5 4" />
    ),
    search: <path d="M9 15A6 6 0 1 0 9 3a6 6 0 0 0 0 12zm4.5-1.5L17.5 17.5" />,
    history: (
      <path d="M3.5 10a6.5 6.5 0 1 0 1.9-4.6M3.5 4v3h3M10 6.5V10l2.5 1.5" />
    ),
    settings: (
      <path d="M10 12.5a2.5 2.5 0 1 0 0-5 2.5 2.5 0 0 0 0 5z M10 2.5l1.2 2 2.3-.3.6 2.2 2 1.2-1.1 2 1.1 2-2 1.2-.6 2.2-2.3-.3L10 17.5l-1.2-2-2.3.3-.6-2.2-2-1.2 1.1-2-1.1-2 2-1.2.6-2.2 2.3.3z" />
    ),
  };
  return (
    <svg
      width="17"
      height="17"
      viewBox="0 0 20 20"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.6"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      {paths[name]}
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
  } = useStore();

  useEffect(() => {
    void bootstrap();
  }, [bootstrap]);

  if (screen === "onboarding") {
    return (
      <div className="main">
        <OnboardingScreen />
      </div>
    );
  }

  const nav = [
    { id: "home", label: "Home", icon: "home" },
    { id: "circle", label: "Circle", icon: "circle", count: friends.length },
    { id: "search", label: "Search", icon: "search" },
    { id: "history", label: "History", icon: "history", count: drives.length },
    { id: "settings", label: "Settings", icon: "settings" },
  ] as const;

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
                strokeWidth="3"
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
            <NavIcon name={item.icon} />
            {item.label}
            {"count" in item && item.count > 0 && (
              <span className="count">{item.count}</span>
            )}
          </button>
        ))}

        <div className="sidebar-foot">
          <span className="offline-badge">
            <span className="offline-dot" />
            Works offline
          </span>
          <span>Nothing leaves this machine.</span>
        </div>
      </nav>

      <main className="main">
        {error && (
          <div className="notice danger" style={{ marginBottom: 18 }}>
            <strong>{error.message}</strong>
            {error.detail && (
              <p style={{ margin: "6px 0 0", fontSize: 12.5 }}>
                {error.detail}
              </p>
            )}
            <div className="btn-row" style={{ marginTop: 10 }}>
              <button className="btn small" onClick={clearError}>
                Dismiss
              </button>
              {error.code === "duplicate_drive" && error.detail && (
                <button
                  className="btn small"
                  onClick={() => {
                    const id = Number(error.detail);
                    clearError();
                    if (!Number.isNaN(id))
                      void useStore.getState().openDrive(id);
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
        {screen === "settings" && <SettingsScreen />}
      </main>

      {toast && (
        <div className="toast" role="status">
          {toast}
        </div>
      )}
    </div>
  );
}
