/**
 * The real interface, on the demo season, staged for a screenshot.
 *
 *   screens.html?theme=light#result
 *
 * The core is stood in for by `answer` below, which hands back the invented
 * season in ./data. SF Symbols are drawn by macOS through the capture tool
 * (scripts/screenshots/capture.swift), exactly as the app's own bridge draws
 * them; anywhere else the interface falls back to its own drawings.
 *
 * When a scene has settled, the page sets data-ready on <html>, and the
 * capture tool takes the picture.
 */

import React from "react";
import ReactDOM from "react-dom/client";
import type { SymbolRequest } from "../lib/api";
import * as demo from "./data";
import "../styles/app.css";

type Native = { postMessage(body: unknown): Promise<unknown> };
const native = (
  window as unknown as {
    webkit?: { messageHandlers?: { symbols?: Native } };
  }
).webkit?.messageHandlers?.symbols;

async function answer(cmd: string, args: Record<string, unknown>) {
  switch (cmd) {
    case "get_profile":
      return demo.profile;
    case "list_friends":
      return demo.friends;
    case "list_drives":
      return demo.drives;
    case "season":
      return demo.season;
    case "identity_stats":
      return demo.stats;
    case "get_drive_detail":
      return demo.detail(Number(args.id));
    case "find_evidence":
      return demo.detail(Number(args.driveId)).evidence;
    case "share_summary":
      return demo.summary;
    case "recent_downloads":
      return demo.recent;
    case "app_version":
      return { version: "0.1.4", built: null };
    case "check_for_update":
      return { status: "up_to_date" };
    case "watch_downloads":
      return false;
    case "take_pending_files":
    case "data_inventory":
      return [];
    case "symbols": {
      const requests = args.requests as SymbolRequest[];
      return native
        ? await native.postMessage(requests)
        : requests.map(() => null);
    }
    default:
      // Event subscriptions, the pending route, anything a scene never uses.
      return null;
  }
}

let callbacks = 0;
Object.assign(window, {
  __TAURI_INTERNALS__: {
    invoke: (cmd: string, args: Record<string, unknown> = {}) =>
      answer(cmd, args),
    transformCallback: () => ++callbacks,
    unregisterCallback: () => {},
    convertFileSrc: (path: string) => path,
    metadata: {
      currentWindow: { label: "main" },
      currentWebview: { label: "main" },
    },
  },
});

const params = new URLSearchParams(location.search);
const theme = params.get("theme") === "dark" ? "dark" : "light";
document.documentElement.dataset.theme = theme;
document.documentElement.dataset.platform = "mac";
document.documentElement.style.colorScheme = theme;
// macOS hides an idle scroll bar; a capture should too.
document.head.insertAdjacentHTML(
  "beforeend",
  "<style>::-webkit-scrollbar { width: 0; height: 0; }</style>",
);

const pause = (ms: number) => new Promise((r) => setTimeout(r, ms));

/** Waits for an element to exist, then hands it back. */
async function find<T extends Element>(
  test: () => T | null | undefined,
): Promise<T> {
  for (let i = 0; i < 100; i++) {
    const el = test();
    if (el) return el;
    await pause(50);
  }
  throw new Error("scene element never appeared");
}

const byText = (selector: string, text: string) =>
  [...document.querySelectorAll(selector)].find((el) =>
    el.textContent?.includes(text),
  ) as HTMLElement | undefined;

async function stage(scene: string) {
  const { useStore } = await import("../lib/store");
  // After startup has loaded the season, or it would put the home screen
  // back over whatever the scene opened.
  await find(() =>
    useStore.getState().drives.length > 0
      ? document.querySelector(".surface")
      : null,
  );
  await pause(300);
  const open = async (id: number) => {
    await useStore.getState().openDrive(id);
    await find(() => document.querySelector(".verdict"));
  };
  switch (scene) {
    case "result":
      await open(1);
      break;
    case "why":
      await open(1);
      (await find(() => byText("summary", "Why does it think"))).click();
      break;
    case "share":
      await open(1);
      (await find(() => byText("button", "Share"))).click();
      await find(() => document.querySelector(".share-pass"));
      // Round to the line the README quotes.
      for (let i = 0; i < 10; i++) {
        const card = document.querySelector(".share-pass");
        if (card?.getAttribute("aria-label")?.includes("Mom")) break;
        document
          .querySelector<HTMLElement>('[aria-label="Another line"]')
          ?.click();
        await pause(120);
      }
      break;
    case "circle":
      (
        await find(() =>
          document.querySelector<HTMLElement>('[aria-label="Circle"]'),
        )
      ).click();
      break;
    case "analysis": {
      await open(1);
      const block = await find(() =>
        byText(".list-head", "What this shortlist suggests"),
      );
      const content = document.querySelector(".content")!;
      content.scrollTop = block.getBoundingClientRect().top - 60;
      break;
    }
  }
  // Long enough for every entrance to finish, the count to land and the
  // symbols to arrive.
  await pause(1800);
  // For a close-up, the bounds of the one thing it shows.
  const subject =
    scene === "why"
      ? document.querySelector(".verdict")
      : scene === "analysis"
        ? byText(".card", "CGPA cutoff around")
        : null;
  if (subject) {
    const r = subject.getBoundingClientRect();
    document.documentElement.dataset.crop = [r.x, r.y, r.width, r.height]
      .map((v) => Math.round(v))
      .join(" ");
  }
  document.documentElement.dataset.ready = "1";
}

void (async () => {
  const { default: App } = await import("../App");
  ReactDOM.createRoot(document.getElementById("root")!).render(
    <React.StrictMode>
      <App />
    </React.StrictMode>,
  );
  await stage(location.hash.slice(1) || "home").catch((e: unknown) => {
    document.documentElement.dataset.ready = `error: ${String(e)}`;
  });
})();
