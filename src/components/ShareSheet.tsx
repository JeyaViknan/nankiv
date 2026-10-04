/**
 * Sharing an "I'm in" card: a preview, then Copy or Save.
 *
 * Copy puts the image on the clipboard for a chat or a story; Save writes a
 * PNG where the student chooses. What is on it is said plainly beneath the
 * preview, so no one wonders whether their ID went out with it.
 */

import { useEffect, useMemo, useState } from "react";
import { save } from "@tauri-apps/plugin-dialog";
import { api, type ImportOutcome } from "../lib/api";
import { cardFacts, renderCard } from "../lib/shareCard";
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
  const [card, setCard] = useState<{ blob: Blob; url: string } | null>(null);
  const [failed, setFailed] = useState(false);

  useEffect(() => {
    let live = true;
    let url: string | null = null;
    renderCard(facts)
      .then((blob) => {
        if (!live) return;
        url = URL.createObjectURL(blob);
        setCard({ blob, url });
      })
      .catch(() => live && setFailed(true));
    return () => {
      live = false;
      if (url) URL.revokeObjectURL(url);
    };
  }, [facts]);

  async function copy() {
    if (!card) return;
    try {
      await navigator.clipboard.write([
        new ClipboardItem({ "image/png": card.blob }),
      ]);
      showToast("Card copied — paste it into a chat or a story");
    } catch {
      showToast("Couldn't copy the card — save it instead");
    }
  }

  async function saveCard() {
    if (!card) return;
    const path = await save({
      defaultPath: `${fileStem(facts.company)}-im-in.png`,
      filters: [{ name: "PNG image", extensions: ["png"] }],
    });
    if (!path) return;
    try {
      await api.saveShareCard(path, await card.blob.arrayBuffer());
      showToast("Card saved");
    } catch {
      showToast("Couldn't save the card");
    }
  }

  return (
    <Sheet title="Share the news" onClose={onClose} width={520}>
      <div className="share-preview">
        {card ? (
          <img
            src={card.url}
            alt={`I'm in — ${facts.company}, one of ${facts.total.toLocaleString()} shortlisted`}
          />
        ) : (
          <p className="hint-line">
            {failed ? "The card couldn't be drawn." : "Drawing your card…"}
          </p>
        )}
      </div>
      <p className="hint-line share-note">
        Just the company, the round and the count. No Neo ID, no CGPA, and no
        one else's name.
      </p>
      <div className="btn-row">
        <button
          className="btn primary"
          onClick={() => void copy()}
          disabled={!card}
        >
          Copy image
        </button>
        <button
          className="btn"
          onClick={() => void saveCard()}
          disabled={!card}
        >
          Save…
        </button>
      </div>
    </Sheet>
  );
}
