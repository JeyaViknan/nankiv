/**
 * SF Symbols, asked of macOS as the interface needs them.
 *
 * Each symbol is drawn by the system at the size, weight and pixel density it
 * will be shown at — the same drawing a native Mac app gets — and cached for
 * the life of the window. Every icon that renders in the same tick is asked
 * for in one call, so opening a screen crosses the bridge once, not once per
 * icon.
 *
 * Where the system cannot supply symbols — Windows, the browser preview, the
 * tests, macOS before 11 — the answer is `null`, known without asking, and the
 * icon falls back to nankiv's own drawing.
 */

import { useSyncExternalStore } from "react";
import { api, type SymbolImage, type SymbolRequest } from "./api";

/** `undefined` while it is being drawn; `null` where there is none. */
export type SymbolEntry = SymbolImage | null | undefined;

const cache = new Map<string, SymbolImage | null>();
const asked = new Set<string>();
const listeners = new Set<() => void>();
let queue: { key: string; request: SymbolRequest }[] = [];

/** Only the Mac app can draw them: SF Symbols are licensed for Apple platforms. */
export function symbolsAvailable(): boolean {
  return (
    typeof window !== "undefined" &&
    "__TAURI_INTERNALS__" in window &&
    /Macintosh/.test(navigator.userAgent)
  );
}

/** Whole device pixels per CSS pixel, so the drawing lands on the grid. */
export function symbolScale(): number {
  const ratio = typeof window === "undefined" ? 1 : window.devicePixelRatio;
  return Math.min(4, Math.max(1, Math.ceil(ratio || 1)));
}

function keyOf(r: SymbolRequest): string {
  return `${r.name} ${r.pointSize} ${r.weight} ${r.scale}`;
}

function ask(key: string, request: SymbolRequest): void {
  if (asked.has(key)) return;
  asked.add(key);
  queue.push({ key, request });
  if (queue.length === 1) queueMicrotask(() => void flush());
}

async function flush(): Promise<void> {
  const batch = queue;
  queue = [];
  let images: (SymbolImage | null)[] = [];
  try {
    images = await api.symbols(batch.map((b) => b.request));
  } catch {
    // The drawn set is always there to fall back on.
  }
  batch.forEach((b, i) => cache.set(b.key, images[i] ?? null));
  listeners.forEach((notify) => notify());
}

function subscribe(notify: () => void): () => void {
  listeners.add(notify);
  return () => listeners.delete(notify);
}

/**
 * The symbol, once macOS has drawn it.
 *
 * Asking is idempotent, so it happens during render rather than in an effect:
 * an effect would wait for the first paint, and every icon would appear one
 * frame after the screen it belongs to.
 */
export function useSymbol(request: Omit<SymbolRequest, "scale">): SymbolEntry {
  const available = symbolsAvailable();
  const full = { ...request, scale: symbolScale() };
  const key = keyOf(full);
  const entry = useSyncExternalStore(subscribe, () =>
    available ? cache.get(key) : null,
  );
  if (available && entry === undefined) ask(key, full);
  return entry;
}

/** For tests: forget everything drawn so far. */
export function resetSymbols(): void {
  cache.clear();
  asked.clear();
  queue = [];
}

/**
 * A symbol for drawing outside the DOM — onto a canvas — or `null` where the
 * system has none. Not cached: a card is drawn once.
 */
export async function symbolImage(
  request: Omit<SymbolRequest, "scale">,
): Promise<SymbolImage | null> {
  if (!symbolsAvailable()) return null;
  try {
    const [image] = await api.symbols([{ ...request, scale: 2 }]);
    return image ?? null;
  } catch {
    return null;
  }
}
