/**
 * Following `nankiv://` links — how a click on the desktop widget lands on the
 * shortlist it was showing, rather than on the app's front page.
 *
 * Links arrive two ways. If the app is already running, the core emits a
 * "deep-link" event. If the click launched the app, nothing was listening yet,
 * so the core holds the route until the interface asks for it during bootstrap.
 * Both paths end in `followRoute`, and both clear the held copy, so a link is
 * never followed again later by surprise.
 */

import { useEffect, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import type { DriveRecord, Route } from "./api";

export type RouteAction =
  { kind: "open"; id: number } | { kind: "home"; notice?: string };

/**
 * Decides what a route means given the drives that exist right now.
 *
 * The widget can lag the app — a shortlist deleted a moment ago may still be
 * on the desktop — so a link to a drive that is gone lands on the shortlists
 * with a plain explanation, never on an error.
 */
export function resolveRoute(route: Route, drives: DriveRecord[]): RouteAction {
  switch (route.kind) {
    case "drive":
      return drives.some((d) => d.id === route.id)
        ? { kind: "open", id: route.id }
        : { kind: "home", notice: "That shortlist is no longer in nankiv" };
    case "latest": {
      const newest = drives[0];
      return newest ? { kind: "open", id: newest.id } : { kind: "home" };
    }
    case "shortlists":
      return { kind: "home" };
  }
}

export function isRoute(value: unknown): value is Route {
  if (typeof value !== "object" || value === null) return false;
  const v = value as { kind?: unknown; id?: unknown };
  if (v.kind === "latest" || v.kind === "shortlists") return true;
  return (
    v.kind === "drive" &&
    typeof v.id === "number" &&
    Number.isInteger(v.id) &&
    v.id > 0
  );
}

/**
 * Follows links that arrive while the app is running. Subscribed once for the
 * life of the component, for the same reason the shortcuts are: re-subscribing
 * on every render leaves two listeners live for a moment.
 */
export function useDeepLinks(follow: (route: Route) => void): void {
  const followRef = useRef(follow);
  followRef.current = follow;

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let disposed = false;
    listen<unknown>("deep-link", (event) => {
      if (isRoute(event.payload)) followRef.current(event.payload);
    })
      .then((un) => {
        if (disposed) un();
        else unlisten = un;
      })
      .catch(() => {
        // Outside the desktop shell there are no links to follow.
      });
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);
}
