/**
 * Key names as the platform writes them: ⌘ on a Mac, Ctrl everywhere else.
 * The shortcuts themselves are matched per platform in shortcuts.ts; this is
 * only what the interface tells people to press.
 */

import { detectMac } from "./shortcuts";

export const MOD = detectMac() ? "⌘" : "Ctrl";
