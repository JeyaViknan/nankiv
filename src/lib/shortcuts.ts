/**
 * Keyboard shortcuts.
 *
 * Every shortcut can reach the interface two ways: as a native menu accelerator
 * (forwarded from Rust as a "menu" event) and as a keydown in the webview.
 * Which of those actually fires depends on the platform. On macOS the menu
 * consumes the key before the webview sees it. On Windows it depends on whether
 * WebView2 lets the accelerator reach the window while the page has focus — and
 * that could not be verified from a Mac.
 *
 * Removing either path would be a guess, and a wrong guess disables the
 * shortcut outright. So neither is removed. Both feed one dispatcher, and the
 * dispatcher runs an action once per press no matter how many paths delivered
 * it. That is correct on every platform whichever way the platform behaves,
 * and it can be tested without the platform.
 */

import { useEffect, useRef } from "react";
import { listen } from "@tauri-apps/api/event";

export type ShortcutId =
  | "settings"
  | "open"
  | "open_reference"
  | "search"
  | "circle"
  | "back"
  | "export_xlsx"
  | "export_csv";

export interface Chord {
  /** Lowercase `KeyboardEvent.key`, e.g. `"o"` or `","`. */
  key: string;
  shift: boolean;
}

/**
 * The chords, matched *exactly*.
 *
 * Exactness is the point. The previous check compared only the letter, so
 * Shift+Cmd+O satisfied the test for Cmd+O as well as its own. A test holds
 * this table to the accelerators declared in `src-tauri/src/desktop.rs`, so the
 * two sides cannot drift apart again.
 */
export const CHORDS: Record<ShortcutId, Chord> = {
  settings: { key: ",", shift: false },
  open: { key: "o", shift: false },
  open_reference: { key: "o", shift: true },
  export_xlsx: { key: "e", shift: false },
  export_csv: { key: "e", shift: true },
  search: { key: "f", shift: false },
  circle: { key: "d", shift: false },
  back: { key: "[", shift: false },
};

export const SHORTCUT_IDS = Object.keys(CHORDS) as ShortcutId[];

export function isShortcutId(value: unknown): value is ShortcutId {
  return typeof value === "string" && value in CHORDS;
}

export function detectMac(): boolean {
  if (typeof navigator === "undefined") return false;
  return /Mac|iPhone|iPad/.test(navigator.platform || navigator.userAgent);
}

/**
 * Finds the shortcut a keydown event means, if any.
 *
 * - The primary modifier is Cmd on macOS and Ctrl elsewhere — "CmdOrCtrl", as
 *   the menu declares it. Accepting either on every platform meant a Mac text
 *   field lost Ctrl+D and Ctrl+F, which are delete-forward and cursor-forward
 *   there.
 * - Shift must match exactly, and Alt must not be held.
 * - Auto-repeat from a held key is ignored: holding Cmd+D opens the circle once,
 *   rather than strobing it.
 */
export function matchChord(
  e: Pick<
    KeyboardEvent,
    "key" | "metaKey" | "ctrlKey" | "shiftKey" | "altKey" | "repeat"
  >,
  isMac: boolean,
): ShortcutId | null {
  if (e.repeat || e.altKey) return null;
  const primary = isMac ? e.metaKey && !e.ctrlKey : e.ctrlKey && !e.metaKey;
  if (!primary) return null;
  const key = e.key.length === 1 ? e.key.toLowerCase() : e.key;
  for (const id of SHORTCUT_IDS) {
    const c = CHORDS[id];
    if (c.key === key && c.shift === e.shiftKey) return id;
  }
  return null;
}

/**
 * How long a shortcut stays suppressed after it arrives.
 *
 * Both deliveries of one press arrive well inside this: the keydown is
 * immediate and the forwarded menu event follows within milliseconds. A person
 * does not deliberately press the same shortcut twice within 400ms, and nothing
 * here benefits from it — toggling a sheet twice that fast is a no-op, and a
 * second file picker is never wanted.
 */
export const QUIET_MS = 400;

/**
 * Builds a dispatcher that runs each action once per press.
 *
 * It fires on the first arrival and then stays quiet until the action has gone
 * unrequested for `quietMs`. The clock restarts on every arrival, suppressed or
 * not, so a key held down — which some platforms repeat through the menu as
 * well — produces one action rather than one every `quietMs`.
 */
export function createDispatcher(
  run: (id: ShortcutId) => void,
  quietMs: number = QUIET_MS,
  now: () => number = () => performance.now(),
): (id: ShortcutId) => boolean {
  const lastSeen = new Map<ShortcutId, number>();
  return (id) => {
    const t = now();
    const prev = lastSeen.get(id);
    lastSeen.set(id, t);
    if (prev !== undefined && t - prev < quietMs) return false;
    run(id);
    return true;
  };
}

export type ShortcutActions = Record<ShortcutId, () => void>;

/**
 * Wires both delivery paths to one dispatcher, for the life of the component.
 *
 * Two details carry most of the weight. The subscriptions are made once, not on
 * every render: the previous menu listener re-subscribed after each render and
 * released the old one asynchronously, so for a moment two listeners were live
 * — and every shortcut triggers a render. And the dispatcher is created once,
 * so a render between the keydown and the forwarded menu event cannot reset the
 * record of what already ran. Actions are read through a ref so they can change
 * freely without either of those being rebuilt.
 */
export function useShortcuts(
  actions: ShortcutActions,
  isMac: boolean = detectMac(),
): void {
  const actionsRef = useRef(actions);
  actionsRef.current = actions;

  const dispatchRef = useRef<((id: ShortcutId) => boolean) | null>(null);
  if (!dispatchRef.current) {
    dispatchRef.current = createDispatcher((id) => actionsRef.current[id]());
  }

  useEffect(() => {
    const dispatch = dispatchRef.current!;

    const onKey = (e: KeyboardEvent) => {
      const id = matchChord(e, isMac);
      if (!id) return;
      // Claim the key even when the dispatcher suppresses it, so the browser's
      // own binding for the chord (Find, for Ctrl+F in WebView2) never runs.
      e.preventDefault();
      dispatch(id);
    };
    window.addEventListener("keydown", onKey);

    let unlisten: (() => void) | undefined;
    let disposed = false;
    listen<string>("menu", (event) => {
      if (isShortcutId(event.payload)) dispatch(event.payload);
    })
      .then((un) => {
        if (disposed) un();
        else unlisten = un;
      })
      .catch(() => {
        // Outside the desktop shell there is no menu; the keyboard still works.
      });

    return () => {
      disposed = true;
      window.removeEventListener("keydown", onKey);
      unlisten?.();
    };
  }, [isMac]);
}
