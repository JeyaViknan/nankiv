/**
 * Theme behaviour.
 *
 * The rule that matters: "system" must leave the document *unstamped* so the
 * CSS media query governs. A stamped attribute always means an explicit
 * override — if "system" also stamped one, the app would stop following the OS
 * the moment someone visited Settings.
 */

import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  applyTheme,
  getThemeChoice,
  resolveTheme,
  setThemeChoice,
  systemTheme,
} from "./theme";

function mockSystem(dark: boolean) {
  Object.defineProperty(window, "matchMedia", {
    writable: true,
    configurable: true,
    value: vi.fn().mockImplementation((query: string) => ({
      matches: query.includes("dark") ? dark : !dark,
      media: query,
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      addListener: vi.fn(),
      removeListener: vi.fn(),
      dispatchEvent: vi.fn(),
      onchange: null,
    })),
  });
}

beforeEach(() => {
  localStorage.clear();
  document.documentElement.removeAttribute("data-theme");
  mockSystem(false);
});

describe("default behaviour", () => {
  it("follows the operating system when nothing has been chosen", () => {
    expect(getThemeChoice()).toBe("system");
  });

  it("opens dark when the laptop is set to dark", () => {
    mockSystem(true);
    applyTheme("system");
    expect(resolveTheme("system")).toBe("dark");
    expect(systemTheme()).toBe("dark");
  });

  it("opens light when the laptop is set to light", () => {
    mockSystem(false);
    applyTheme("system");
    expect(resolveTheme("system")).toBe("light");
  });

  it("leaves the document unstamped on system, so the media query governs", () => {
    applyTheme("system");
    expect(document.documentElement.hasAttribute("data-theme")).toBe(false);
  });
});

describe("explicit overrides", () => {
  it("stamps the root element so the choice beats the operating system", () => {
    mockSystem(true);
    applyTheme("light");
    expect(document.documentElement.getAttribute("data-theme")).toBe("light");
    expect(resolveTheme("light")).toBe("light");
  });

  it("wins in the other direction too", () => {
    mockSystem(false);
    applyTheme("dark");
    expect(document.documentElement.getAttribute("data-theme")).toBe("dark");
    expect(resolveTheme("dark")).toBe("dark");
  });

  it("sets colorScheme so native controls and scrollbars follow", () => {
    applyTheme("dark");
    expect(document.documentElement.style.colorScheme).toBe("dark");
  });
});

describe("persistence", () => {
  it("remembers an explicit choice across restarts", () => {
    setThemeChoice("dark");
    expect(getThemeChoice()).toBe("dark");
    expect(localStorage.getItem("nankiv.theme")).toBe("dark");
  });

  it("can be put back to following the system", () => {
    setThemeChoice("dark");
    setThemeChoice("system");
    expect(getThemeChoice()).toBe("system");
    expect(document.documentElement.hasAttribute("data-theme")).toBe(false);
  });

  it("ignores a corrupted stored value rather than breaking", () => {
    localStorage.setItem("nankiv.theme", "chartreuse");
    expect(getThemeChoice()).toBe("system");
  });

  it("survives storage being unavailable", () => {
    const original = Storage.prototype.getItem;
    Storage.prototype.getItem = () => {
      throw new Error("storage disabled");
    };
    expect(getThemeChoice()).toBe("system");
    Storage.prototype.getItem = original;
  });
});
