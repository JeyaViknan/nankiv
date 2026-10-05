/**
 * A pasted list, confirmed before it is checked.
 *
 * Shows what was found — so a paste that picked up the wrong text is caught
 * before it becomes a drive — and asks which company it is for, with a guess
 * read from the message's own words. Then it is checked exactly as a file is.
 */

import { useEffect, useState } from "react";
import { api, toApiError, type PastePreview } from "../lib/api";
import { useStore } from "../lib/store";
import { Sheet } from "./Sheet";

function counted(n: number, one: string, many: string): string {
  return `${n.toLocaleString()} ${n === 1 ? one : many}`;
}

export function PasteSheet({
  text,
  onClose,
}: {
  text: string;
  onClose: () => void;
}) {
  const { importPasted } = useStore();
  const [preview, setPreview] = useState<PastePreview | null>(null);
  const [problem, setProblem] = useState<string | null>(null);
  const [company, setCompany] = useState("");

  useEffect(() => {
    let live = true;
    api
      .previewPaste(text)
      .then((p) => {
        if (!live) return;
        setPreview(p);
        setCompany(p.company ?? "");
      })
      .catch((e) => live && setProblem(toApiError(e).message));
    return () => {
      live = false;
    };
  }, [text]);

  async function check() {
    onClose();
    await importPasted(text, company.trim() || null);
  }

  const found = preview
    ? [
        preview.neo_ids > 0 && counted(preview.neo_ids, "Neo ID", "Neo IDs"),
        preview.reg_nos > 0 &&
          counted(
            preview.reg_nos,
            "registration number",
            "registration numbers",
          ),
      ]
        .filter(Boolean)
        .join(" and ")
    : "";

  return (
    <Sheet title="Paste a shortlist" onClose={onClose} width={480}>
      {problem ? (
        <p className="field-error" role="alert">
          {problem}
        </p>
      ) : !preview ? (
        <p className="sheet-lede">Reading what you pasted…</p>
      ) : (
        <>
          <p className="paste-found" role="status">
            Found <strong>{found}</strong>.
            {preview.unread_lines > 0 &&
              ` ${counted(preview.unread_lines, "line", "lines")} without one ${
                preview.unread_lines === 1 ? "was" : "were"
              } skipped.`}
          </p>
          <div className="field">
            <label htmlFor="paste-company">Company</label>
            <input
              id="paste-company"
              type="text"
              placeholder="Which company is this list for?"
              value={company}
              autoFocus
              aria-describedby="paste-company-hint"
              onChange={(e) => setCompany(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && void check()}
            />
            <span className="hint" id="paste-company-hint">
              Only the IDs are kept.
            </span>
          </div>
          <div className="btn-row">
            <button className="btn primary" onClick={() => void check()}>
              Check this list
            </button>
            <button className="btn plain" onClick={onClose}>
              Cancel
            </button>
          </div>
        </>
      )}
    </Sheet>
  );
}
