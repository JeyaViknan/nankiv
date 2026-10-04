/**
 * Pasting a shortlist that arrived as a message.
 *
 * ⌘V anywhere outside a text field — or Edit → Paste — hands the clipboard
 * to nankiv, which looks for identifiers in it. Nothing is stored until the
 * student has seen what was found and named the drive.
 *
 * The check here is only a gate, so an ordinary paste is never hijacked: the
 * core decides what is really an identifier, by the same rules as a file.
 */

import { useEffect, useRef } from "react";

const NEO =
  /(^|[^A-Za-z0-9])[A-Za-z]\d[A-Za-z]\d[A-Za-z]\d[A-Za-z]\d($|[^A-Za-z0-9])/;
const REG = /(^|[^A-Za-z0-9])\d{2}[A-Za-z]{3}\d{4}($|[^A-Za-z0-9])/;

/** Whether pasted text could be a list of students at all. */
export function mightBeShortlist(text: string): boolean {
  return NEO.test(text) || REG.test(text);
}

/** A paste aimed at a field is the field's business, never nankiv's. */
export function isTypingTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  return (
    target.isContentEditable ||
    target.tagName === "INPUT" ||
    target.tagName === "TEXTAREA" ||
    target.tagName === "SELECT"
  );
}

/**
 * Calls `onList` with pasted text that might be a shortlist, while `enabled`.
 * Subscribed once, for the reason every listener here is: re-subscribing on
 * each render briefly leaves two alive.
 */
export function usePastedShortlist(
  onList: (text: string) => void,
  enabled: boolean,
): void {
  const handler = useRef(onList);
  handler.current = onList;
  const on = useRef(enabled);
  on.current = enabled;

  useEffect(() => {
    function onPaste(e: ClipboardEvent) {
      if (!on.current || isTypingTarget(e.target)) return;
      const text = e.clipboardData?.getData("text/plain") ?? "";
      if (!mightBeShortlist(text)) return;
      e.preventDefault();
      handler.current(text);
    }
    document.addEventListener("paste", onPaste);
    return () => document.removeEventListener("paste", onPaste);
  }, []);
}
