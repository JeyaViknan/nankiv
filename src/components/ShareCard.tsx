/**
 * Sharing an "I'm in" card: the card itself, presented on its own.
 *
 * No sheet, no title bar, no paragraph — the pass rises out of the blurred
 * window and settles, the way a pass is presented in Wallet, with two
 * floating actions beneath it. Escape, the close button or a click outside
 * puts it away. The actions confirm where they are pressed rather than in a
 * toast, which would sit behind the blur.
 *
 * The card is drawn straight onto the canvas shown here, so there is no image
 * address for the page's security policy to refuse; Copy and Save take their
 * PNG from that same canvas.
 */

import { useEffect, useMemo, useRef, useState } from "react";
import { save } from "@tauri-apps/plugin-dialog";
import { api, type ImportOutcome } from "../lib/api";
import { useDialog } from "../lib/dialog";
import { cardFacts, cardPng, drawCard } from "../lib/shareCard";
import { useStore } from "../lib/store";
import { Icon } from "./Icon";

type Done = "copied" | "saved" | "copy-failed" | "save-failed" | null;

const SAID: Record<Exclude<Done, null>, string> = {
  copied: "Copied",
  saved: "Saved",
  "copy-failed": "Couldn't copy",
  "save-failed": "Couldn't save",
};

function fileStem(company: string): string {
  const s = company
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-|-$/g, "");
  return s || "shortlist";
}

export function ShareCard({
  outcome,
  onClose,
}: {
  outcome: ImportOutcome;
  onClose: () => void;
}) {
  const { drives } = useStore();
  const facts = useMemo(() => cardFacts(outcome, drives), [outcome, drives]);
  const stage = useRef<HTMLDivElement>(null);
  const canvas = useRef<HTMLCanvasElement>(null);
  const [ready, setReady] = useState(false);
  const [done, setDone] = useState<Done>(null);
  useDialog(stage, onClose);

  useEffect(() => {
    if (!canvas.current) return;
    setReady(false);
    drawCard(canvas.current, facts)
      .then(() => setReady(true))
      .catch(() => setReady(false));
  }, [facts]);

  // Each confirmation stays a moment, then the button reads as itself again.
  useEffect(() => {
    if (!done) return;
    const t = setTimeout(() => setDone(null), 1800);
    return () => clearTimeout(t);
  }, [done]);

  async function copy() {
    if (!ready || !canvas.current) return;
    try {
      await navigator.clipboard.write([
        new ClipboardItem({ "image/png": cardPng(canvas.current) }),
      ]);
      setDone("copied");
    } catch {
      setDone("copy-failed");
    }
  }

  async function saveCard() {
    if (!ready || !canvas.current) return;
    const path = await save({
      defaultPath: `${fileStem(facts.company)}-shortlisted.png`,
      filters: [{ name: "PNG image", extensions: ["png"] }],
    });
    if (!path) return;
    try {
      const png = await cardPng(canvas.current);
      await api.saveShareCard(path, await png.arrayBuffer());
      setDone("saved");
    } catch {
      setDone("save-failed");
    }
  }

  const copyLabel =
    done === "copied" || done === "copy-failed" ? SAID[done] : "Copy image";
  const saveLabel =
    done === "saved" || done === "save-failed" ? SAID[done] : "Save…";

  return (
    <div
      ref={stage}
      className="share-stage"
      role="dialog"
      aria-modal="true"
      aria-label="Share the news"
      tabIndex={-1}
      onMouseDown={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <canvas
        ref={canvas}
        className="share-pass"
        role="img"
        aria-label={`Shortlisted — ${facts.company}, one of ${facts.total.toLocaleString()} students`}
      />
      <div className="share-actions" aria-live="polite">
        <button
          className="float-btn primary"
          onClick={() => void copy()}
          disabled={!ready}
        >
          {done === "copied" && <Icon name="check" size={14} weight="bold" />}
          {copyLabel}
        </button>
        <button
          className="float-btn glass"
          onClick={() => void saveCard()}
          disabled={!ready}
        >
          {done === "saved" && <Icon name="check" size={14} weight="bold" />}
          {saveLabel}
        </button>
      </div>
      <button
        className="float-close glass"
        onClick={onClose}
        aria-label="Close"
      >
        <Icon name="close" size={14} weight="semibold" />
      </button>
    </div>
  );
}
