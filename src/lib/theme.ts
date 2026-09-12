/**
 * Theme control.
 *
 * Three states, not two. "System" is the default and the one most people should
 * stay on: the app opens in whatever the laptop is already set to, and follows
 * it live if the OS switches at sunset.
 *
 * The preference lives in `localStorage` rather than the database on purpose.
 * It has to be readable *synchronously* before the first paint — an async round
 * trip to SQLite would render one theme and then snap to the other, which is
 * exactly the cheap-feeling flash this redesign is meant to remove. It is also
 * a per-machine view preference rather than user data, so the database is the
 * wrong home for it.
 */

export type ThemeChoice = "system" | "light" | "dark";
export type ResolvedTheme = "light" | "dark";

const KEY = "nankiv.theme";

/** Reads the stored choice, defaulting to following the operating system. */
export function getThemeChoice(): ThemeChoice {
  try {
    const v = localStorage.getItem(KEY);
    if (v === "light" || v === "dark" || v === "system") return v;
  } catch {
    /* private mode, or storage disabled */
  }
  return "system";
}

/** What the operating system is currently asking for. */
export function systemTheme(): ResolvedTheme {
  return window.matchMedia?.("(prefers-color-scheme: dark)").matches
    ? "dark"
    : "light";
}

export function resolveTheme(choice: ThemeChoice): ResolvedTheme {
  return choice === "system" ? systemTheme() : choice;
}

/**
 * Applies a choice to the document.
 *
 * On "system" the `data-theme` attribute is *removed* rather than set, so the
 * CSS media query takes over. That keeps one source of truth: a stamped
 * attribute always means an explicit override.
 */
export function applyTheme(choice: ThemeChoice): ResolvedTheme {
  const root = document.documentElement;
  if (choice === "system") {
    root.removeAttribute("data-theme");
  } else {
    root.setAttribute("data-theme", choice);
  }
  const resolved = resolveTheme(choice);
  root.style.colorScheme = resolved;
  return resolved;
}

export function setThemeChoice(choice: ThemeChoice): ResolvedTheme {
  try {
    localStorage.setItem(KEY, choice);
  } catch {
    /* the theme still applies for this session */
  }
  return applyTheme(choice);
}

/**
 * Subscribes to operating-system theme changes.
 *
 * Only meaningful while the choice is "system"; the caller re-subscribes when
 * the choice changes.
 */
export function watchSystemTheme(
  onChange: (theme: ResolvedTheme) => void,
): () => void {
  const mq = window.matchMedia?.("(prefers-color-scheme: dark)");
  if (!mq) return () => {};
  const handler = (e: MediaQueryListEvent) =>
    onChange(e.matches ? "dark" : "light");
  mq.addEventListener("change", handler);
  return () => mq.removeEventListener("change", handler);
}

/** Applies the stored choice. Called once, as early as possible. */
export function initTheme(): ResolvedTheme {
  return applyTheme(getThemeChoice());
}
