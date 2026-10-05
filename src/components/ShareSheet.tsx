/**
 * Sharing an "I'm in" card: a preview, then Copy or Save.
 *
 * Copy puts the image on the clipboard for a chat or a story; Save writes a
 * PNG where the student chooses. What is on it is said plainly beneath the
 * preview, so no one wonders whether their ID went out with it.
 */

import { useEffect, useMemo, useRef, useState } from "react";
import { save } from "@tauri-apps/plugin-dialog";
import { api, type ImportOutcome } from "../lib/api";
import { cardFacts, cardPng, drawCard } from "../lib/shareCard";
import { useStore } from "../lib/store";
import { Sheet } from "./Sheet";

function fileStem(company: string): string {
  const s = company
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-|-$/g, "");
  return s || "shortlist";
}

export function ShareSheet({
  outcome,
  onClose,
}: {
  outcome: ImportOutcome;
  onClose: () => void;
}) {
  const { drives, showToast } = useStore();
  const facts = useMemo(() => cardFacts(outcome, drives), [outcome, drives]);
  // The preview is the card itself: drawn straight onto this canvas, so
  // there is no image address for the page's security policy to refuse.
  const canvas = useRef<HTMLCanvasElement>(null);
  const [ready, setReady] = useState(false);
  const [failed, setFailed] = useState(false);

  useEffect(() => {
    if (!canvas.current) return;
    setReady(false);
    drawCard(canvas.current, facts)
      .then(() => setReady(true))
      .catch(() => setFailed(true));
  }, [facts]);

  async function copy() {
    if (!ready || !canvas.current) return;
    try {
      await navigator.clipboard.write([
        new ClipboardItem({ "image/png": cardPng(canvas.current) }),
      ]);
      showToast("Card copied — paste it into a chat or a story");
    } catch {
      showToast("Couldn't copy the card — save it instead");
    }
  }

  async function saveCard() {
    if (!ready || !canvas.current) return;
    const path = await save({
      defaultPath: `${fileStem(facts.company)}-im-in.png`,
      filters: [{ name: "PNG image", extensions: ["png"] }],
    });
    if (!path) return;
    try {
      const png = await cardPng(canvas.current);
      await api.saveShareCard(path, await png.arrayBuffer());
      showToast("Card saved");
    } catch {
      showToast("Couldn't save the card");
    }
  }

  return (
    <Sheet title="Share the news" onClose={onClose} width={520}>
      <div className="share-preview">
        <canvas
          ref={canvas}
          role="img"
          aria-label={`Shortlisted — ${facts.company}, one of ${facts.total.toLocaleString()} students`}
        />
        {failed && (
          <p className="hint-line" role="alert">
            The card couldn't be drawn.
          </p>
        )}
      </div>
      <p className="hint-line share-note">
        The company, the count, the round and the date. No Neo ID, no CGPA, and
        no one else's name.
      </p>
      <div className="btn-row">
        <button
          className="btn primary"
          onClick={() => void copy()}
          disabled={!ready}
        >
          Copy image
        </button>
        <button
          className="btn"
          onClick={() => void saveCard()}
          disabled={!ready}
        >
          Save…
        </button>
      </div>
    </Sheet>
  );
}
