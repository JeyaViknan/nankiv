/**
 * Shortcuts run once per press, whichever paths deliver them.
 *
 * The Windows behaviour that caused the double-fire cannot be reproduced from a
 * Mac, so these tests do not depend on it. They drive both delivery paths
 * directly — a keydown in the page and a forwarded menu event — in every order
 * and combination a platform could produce, and require exactly one action.
 */

import { act, renderHook } from "@testing-library/react";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  CHORDS,
  QUIET_MS,
  SHORTCUT_IDS,
  createDispatcher,
  matchChord,
  useShortcuts,
  type ShortcutActions,
  type ShortcutId,
} from "./shortcuts";

const bus = vi.hoisted(() => ({
  handler: null as ((e: { payload: unknown }) => void) | null,
  unlisten: vi.fn(),
  listen: vi.fn(),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: (...args: unknown[]) => bus.listen(...args),
}));

type Key = Partial<
  Pick<
    KeyboardEvent,
    "key" | "metaKey" | "ctrlKey" | "shiftKey" | "altKey" | "repeat"
  >
>;

function ev(k: Key) {
  return {
    key: "",
    metaKey: false,
    ctrlKey: false,
    shiftKey: false,
    altKey: false,
    repeat: false,
    ...k,
  };
}

// ---------------------------------------------------------------------------
// Matching
// ---------------------------------------------------------------------------

describe("matching a chord", () => {
  it("tells Cmd+O and Shift+Cmd+O apart", () => {
    // The bug: only the letter was compared, so Shift+Cmd+O also satisfied the
    // test for Cmd+O — two different dialogs from one press.
    expect(matchChord(ev({ key: "o", metaKey: true }), true)).toBe("open");
    expect(
      matchChord(ev({ key: "O", metaKey: true, shiftKey: true }), true),
    ).toBe("open_reference");
    expect(matchChord(ev({ key: "e", metaKey: true }), true)).toBe(
      "export_xlsx",
    );
    expect(
      matchChord(ev({ key: "E", metaKey: true, shiftKey: true }), true),
    ).toBe("export_csv");
  });

  it("uses Cmd on macOS and Ctrl elsewhere", () => {
    expect(matchChord(ev({ key: "d", metaKey: true }), true)).toBe("circle");
    expect(matchChord(ev({ key: "d", ctrlKey: true }), false)).toBe("circle");
  });

  it("leaves Ctrl alone on macOS, where it edits text", () => {
    // Ctrl+D deletes forward and Ctrl+F moves the cursor in any Mac text field.
    expect(matchChord(ev({ key: "d", ctrlKey: true }), true)).toBeNull();
    expect(matchChord(ev({ key: "f", ctrlKey: true }), true)).toBeNull();
  });

  it("does not treat the Windows key as Ctrl", () => {
    expect(matchChord(ev({ key: "o", metaKey: true }), false)).toBeNull();
  });

  it("requires the modifier set to match exactly", () => {
    expect(matchChord(ev({ key: "o" }), true)).toBeNull();
    expect(
      matchChord(ev({ key: "o", metaKey: true, altKey: true }), true),
    ).toBeNull();
    expect(
      matchChord(ev({ key: "d", metaKey: true, ctrlKey: true }), true),
    ).toBeNull();
    expect(
      matchChord(ev({ key: "f", metaKey: true, shiftKey: true }), true),
    ).toBeNull();
  });

  it("ignores auto-repeat from a held key", () => {
    expect(
      matchChord(ev({ key: "d", metaKey: true, repeat: true }), true),
    ).toBeNull();
  });

  it("can reach every shortcut on both platforms", () => {
    for (const id of SHORTCUT_IDS) {
      const { key, shift } = CHORDS[id];
      expect(
        matchChord(ev({ key, shiftKey: shift, metaKey: true }), true),
      ).toBe(id);
      expect(
        matchChord(ev({ key, shiftKey: shift, ctrlKey: true }), false),
      ).toBe(id);
    }
  });
});

// ---------------------------------------------------------------------------
// The dispatcher
// ---------------------------------------------------------------------------

describe("the dispatcher", () => {
  function harness() {
    let t = 0;
    const ran: ShortcutId[] = [];
    const dispatch = createDispatcher(
      (id) => ran.push(id),
      QUIET_MS,
      () => t,
    );
    return {
      ran,
      at: (ms: number, id: ShortcutId) => {
        t = ms;
        return dispatch(id);
      },
    };
  }

  it("runs a single delivery", () => {
    const h = harness();
    expect(h.at(0, "settings")).toBe(true);
    expect(h.ran).toEqual(["settings"]);
  });

  it("runs once when both paths deliver the same press", () => {
    const h = harness();
    h.at(0, "settings"); // keydown
    expect(h.at(12, "settings")).toBe(false); // forwarded menu event
    expect(h.ran).toEqual(["settings"]);
  });

  it("still runs once if the second delivery is slow", () => {
    const h = harness();
    h.at(0, "open");
    h.at(QUIET_MS - 1, "open");
    expect(h.ran).toEqual(["open"]);
  });

  it("runs again for a genuinely separate press", () => {
    const h = harness();
    h.at(0, "circle");
    h.at(QUIET_MS + 50, "circle");
    expect(h.ran).toEqual(["circle", "circle"]);
  });

  it("runs once for a held key, however long it is held", () => {
    // Some platforms repeat menu accelerators while a key is held. Measuring
    // the quiet period from the last arrival, not the last run, is what stops
    // that turning into one toggle every few hundred milliseconds.
    const h = harness();
    for (let ms = 0; ms <= 3000; ms += 40) h.at(ms, "circle");
    expect(h.ran).toEqual(["circle"]);
  });

  it("keeps different shortcuts independent", () => {
    const h = harness();
    h.at(0, "open");
    h.at(5, "search");
    expect(h.ran).toEqual(["open", "search"]);
  });
});

// ---------------------------------------------------------------------------
// The hook, with both real delivery paths
// ---------------------------------------------------------------------------

describe("useShortcuts", () => {
  let actions: ShortcutActions;

  beforeEach(() => {
    bus.handler = null;
    bus.unlisten.mockReset();
    // Set per test: restoring mocks after each test clears implementations.
    bus.listen.mockReset();
    bus.listen.mockImplementation(
      async (_name: string, h: (e: { payload: unknown }) => void) => {
        bus.handler = h;
        return bus.unlisten;
      },
    );
    actions = Object.fromEntries(
      SHORTCUT_IDS.map((id) => [id, vi.fn()]),
    ) as unknown as ShortcutActions;
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  async function mount(isMac = false) {
    const hook = renderHook(
      ({ a }: { a: ShortcutActions }) => useShortcuts(a, isMac),
      { initialProps: { a: actions } },
    );
    await act(async () => {}); // let the menu subscription resolve
    return hook;
  }

  function press(k: Key) {
    const e = new KeyboardEvent("keydown", { cancelable: true, ...ev(k) });
    act(() => {
      window.dispatchEvent(e);
    });
    return e;
  }

  function menu(payload: unknown) {
    act(() => bus.handler?.({ payload }));
  }

  it("runs once when the keydown and the menu both fire — the Windows case", async () => {
    await mount(false);
    press({ key: ",", ctrlKey: true });
    menu("settings");
    expect(actions.settings).toHaveBeenCalledTimes(1);
  });

  it("runs once whichever path arrives first", async () => {
    await mount(false);
    menu("open");
    press({ key: "o", ctrlKey: true });
    expect(actions.open).toHaveBeenCalledTimes(1);
  });

  it("runs once even when a render lands between the two deliveries", async () => {
    // Every shortcut causes a render. If that rebuilt the dispatcher, its record
    // of what had just run would be lost and the second delivery would fire.
    const hook = await mount(false);
    press({ key: "d", ctrlKey: true });
    hook.rerender({ a: { ...actions } });
    menu("circle");
    expect(actions.circle).toHaveBeenCalledTimes(1);
  });

  it("subscribes to the menu once, not on every render", async () => {
    // The previous listener re-subscribed after each render and released the
    // old one asynchronously, leaving two live for a moment.
    const hook = await mount(false);
    hook.rerender({ a: { ...actions } });
    hook.rerender({ a: { ...actions } });
    expect(bus.listen).toHaveBeenCalledTimes(1);
  });

  it("uses the latest actions after a render", async () => {
    const hook = await mount(false);
    const next = { ...actions, search: vi.fn() };
    hook.rerender({ a: next });
    press({ key: "f", ctrlKey: true });
    expect(next.search).toHaveBeenCalledTimes(1);
    expect(actions.search).not.toHaveBeenCalled();
  });

  it("fires only the Shift variant for Shift+Ctrl+O", async () => {
    await mount(false);
    press({ key: "O", ctrlKey: true, shiftKey: true });
    menu("open_reference");
    expect(actions.open_reference).toHaveBeenCalledTimes(1);
    expect(actions.open).not.toHaveBeenCalled();
  });

  it("opens one save panel per press of the export shortcut", async () => {
    await mount(false);
    press({ key: "e", ctrlKey: true });
    menu("export_xlsx");
    expect(actions.export_xlsx).toHaveBeenCalledTimes(1);
  });

  it("claims the key so the browser's own binding does not also run", async () => {
    await mount(false);
    const e = press({ key: "f", ctrlKey: true });
    expect(e.defaultPrevented).toBe(true);
  });

  it("does not claim keys it does not handle", async () => {
    await mount(true);
    const e = press({ key: "d", ctrlKey: true }); // delete-forward on a Mac
    expect(e.defaultPrevented).toBe(false);
    expect(actions.circle).not.toHaveBeenCalled();
  });

  it("still works from the menu alone, as on macOS", async () => {
    await mount(true);
    menu("search");
    expect(actions.search).toHaveBeenCalledTimes(1);
  });

  it("ignores menu events it does not recognise", async () => {
    await mount(false);
    menu("something_else");
    menu(42);
    for (const id of SHORTCUT_IDS) expect(actions[id]).not.toHaveBeenCalled();
  });

  it("runs again for a separate press after the quiet period", async () => {
    let t = 1000;
    vi.spyOn(performance, "now").mockImplementation(() => t);
    await mount(false);
    press({ key: "d", ctrlKey: true });
    t += QUIET_MS + 100;
    press({ key: "d", ctrlKey: true });
    expect(actions.circle).toHaveBeenCalledTimes(2);
  });

  it("stops listening on both paths when unmounted", async () => {
    const hook = await mount(false);
    hook.unmount();
    expect(bus.unlisten).toHaveBeenCalledTimes(1);
    press({ key: "d", ctrlKey: true });
    expect(actions.circle).not.toHaveBeenCalled();
  });
});

// ---------------------------------------------------------------------------
// The two sides of the menu must agree
// ---------------------------------------------------------------------------

describe("the native menu and this table", () => {
  const rust = readFileSync(
    join(__dirname, "../../src-tauri/src/desktop.rs"),
    "utf8",
  );

  function parseAccelerator(a: string) {
    const parts = a.split("+");
    const key = parts[parts.length - 1]!;
    return {
      primary: parts.includes("CmdOrCtrl"),
      shift: parts.includes("Shift"),
      key: key.length === 1 ? key.toLowerCase() : key,
    };
  }

  const declared = [
    ...rust.matchAll(/\.id\("([a-z_]+)"\)\s*\.accelerator\("([^"]+)"\)/g),
  ].map((m) => ({ id: m[1]!, ...parseAccelerator(m[2]!) }));

  it("declares an accelerator for every shortcut, and no others", () => {
    expect(declared.map((d) => d.id).sort()).toEqual([...SHORTCUT_IDS].sort());
  });

  it("uses the same chord on both sides", () => {
    // Drift between these two is how Shift+Cmd+O came to fire two actions.
    for (const d of declared) {
      const c = CHORDS[d.id as ShortcutId];
      expect(d.primary, `${d.id} must use CmdOrCtrl`).toBe(true);
      expect({ key: d.key, shift: d.shift }, d.id).toEqual(c);
    }
  });

  it("forwards every shortcut id from Rust to the interface", () => {
    const block = rust.match(/matches!\(\s*id,([\s\S]*?)\)/);
    expect(block, "forwarding list not found in desktop.rs").not.toBeNull();
    const forwarded = [...block![1]!.matchAll(/"([a-z_]+)"/g)].map((m) => m[1]);
    expect(forwarded.sort()).toEqual([...SHORTCUT_IDS].sort());
  });

  it("gives every chord a single meaning", () => {
    const seen = new Set<string>();
    for (const id of SHORTCUT_IDS) {
      const sig = `${CHORDS[id].shift ? "shift+" : ""}${CHORDS[id].key}`;
      expect(seen.has(sig), `${sig} is bound twice`).toBe(false);
      seen.add(sig);
    }
  });
});
