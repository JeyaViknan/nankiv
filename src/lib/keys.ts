/**
 * Key names as the platform writes them: ⌘ on a Mac, Ctrl everywhere else.
 * The shortcuts themselves are matched per platform in shortcuts.ts; this is
 * only what the interface tells people to press.
 */

import { detectMac } from "./shortcuts";

export const IS_MAC = detectMac();

export const MOD = IS_MAC ? "⌘" : "Ctrl";

/** A shortcut as the platform writes one: ⇧⌘E on a Mac, Ctrl+Shift+E elsewhere. */
export function chord(key: string, shift = false): string {
  return IS_MAC
    ? `${shift ? "⇧" : ""}⌘${key}`
    : `Ctrl+${shift ? "Shift+" : ""}${key}`;
}
