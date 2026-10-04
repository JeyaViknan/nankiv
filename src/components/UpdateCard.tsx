/**
 * About nankiv: which version this is, and whether there is a newer one.
 *
 * Checking is always a click — the only time the app reaches the network.
 * Installing replaces this copy in place and restarts into the new one, with
 * everything the student has stored left as it was. If a check cannot be
 * made, the releases page opens in the browser instead: the app itself makes
 * no request for that.
 */

import { useCallback, useEffect, useRef, useState } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { api, toApiError, type AppVersion } from "../lib/api";
import { RELEASES_URL, builtLabel } from "../lib/updates";

type State =
  | { kind: "idle" }
  | { kind: "checking" }
  | { kind: "current" }
  | { kind: "available"; version: string; notes: string | null }
  | { kind: "installing"; version: string }
  | { kind: "failed"; message: string };

export function UpdateCard({ autoCheck = false }: { autoCheck?: boolean }) {
  const [about, setAbout] = useState<AppVersion | null>(null);
  const [state, setState] = useState<State>({ kind: "idle" });
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    api
      .appVersion()
      .then(setAbout)
      .catch(() => setAbout(null));
  }, []);

  const check = useCallback(async () => {
    setState({ kind: "checking" });
    try {
      const r = await api.checkForUpdate();
      setState(
        r.status === "available"
          ? { kind: "available", version: r.version, notes: r.notes }
          : { kind: "current" },
      );
    } catch (e) {
      setState({ kind: "failed", message: toApiError(e).message });
    }
  }, []);

  useEffect(() => {
    if (!autoCheck) return;
    ref.current?.scrollIntoView?.({ block: "nearest" });
    void check();
  }, [autoCheck, check]);

  async function install(version: string) {
    setState({ kind: "installing", version });
    try {
      await api.installUpdate(); // restarts on success
    } catch (e) {
      setState({ kind: "failed", message: toApiError(e).message });
    }
  }

  const built = builtLabel(about?.built ?? null);

  return (
    <div className="card" ref={ref}>
      <h2>About nankiv</h2>
      <p className="card-text">
        Version {about?.version ?? "…"}
        {built && <span className="label-aside">· built {built}</span>}
      </p>

      <div className="update-row" aria-live="polite">
        {state.kind === "idle" && (
          <button className="btn small" onClick={() => void check()}>
            Check for updates
          </button>
        )}
        {state.kind === "checking" && (
          <button className="btn small" disabled>
            Checking…
          </button>
        )}
        {state.kind === "current" && (
          <p className="card-text">You have the newest version.</p>
        )}
        {state.kind === "available" && (
          <>
            <p className="card-text">
              <strong>nankiv {state.version}</strong> is available.
            </p>
            {state.notes && <p className="update-notes">{state.notes}</p>}
            <button
              className="btn primary small"
              onClick={() => void install(state.version)}
            >
              Install and restart
            </button>
          </>
        )}
        {state.kind === "installing" && (
          <p className="card-text">
            Downloading and installing nankiv {state.version}… nankiv will
            restart when it's done.
          </p>
        )}
        {state.kind === "failed" && (
          <>
            <p className="card-text" role="alert">
              {state.message}
            </p>
            <button
              className="btn small"
              onClick={() => void openUrl(RELEASES_URL)}
            >
              Open the releases page
            </button>
          </>
        )}
      </div>
    </div>
  );
}
