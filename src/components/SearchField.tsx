/**
 * Search, as a field rather than a destination.
 *
 * It was previously a whole navigation item containing an input and a button
 * you had to press. Neither was earning its place: search is a thing you do
 * *while* looking at something else, and a search that needs a button click is
 * a holdover from page-reload interfaces.
 *
 * Results appear in a popover beneath the field and offer the obvious next
 * action. Finding someone by name used to be a dead end — you could learn a Neo
 * ID and then had nowhere to put it.
 */

import { useEffect, useRef, useState, type MutableRefObject } from "react";
import {
  api,
  toApiError,
  type PersonResult,
  type SearchResult,
} from "../lib/api";
import { useStore } from "../lib/store";

const NEO = /^(?:[A-Za-z][0-9]){4}$/;
const REG = /^\d{2}[A-Za-z]{3}\d{4,5}$/;

function SearchIcon() {
  return (
    <svg
      width="14"
      height="14"
      viewBox="0 0 16 16"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.6"
      strokeLinecap="round"
      aria-hidden="true"
    >
      <circle cx="7" cy="7" r="4.6" />
      <path d="M10.5 10.5L14 14" />
    </svg>
  );
}

export function SearchField({
  inputRef,
}: {
  inputRef: MutableRefObject<HTMLInputElement | null>;
}) {
  const { refreshFriends, showToast } = useStore();
  const [query, setQuery] = useState("");
  const [byName, setByName] = useState<SearchResult | null>(null);
  const [byId, setById] = useState<PersonResult[] | null>(null);
  const [busy, setBusy] = useState(false);
  const [open, setOpen] = useState(false);
  const wrapRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const q = query.trim();
    if (q.length < 2) {
      setByName(null);
      setById(null);
      setOpen(false);
      return;
    }
    setOpen(true);
    setBusy(true);
    const t = setTimeout(async () => {
      try {
        if (NEO.test(q) || REG.test(q)) {
          setById(await api.lookupIdentifier(q));
          setByName(null);
        } else {
          setByName(await api.searchStudents(q));
          setById(null);
        }
      } catch {
        setByName(null);
        setById(null);
      } finally {
        setBusy(false);
      }
    }, 180);
    return () => clearTimeout(t);
  }, [query]);

  // Dismiss on an outside click, the way a popover should.
  useEffect(() => {
    const onDown = (e: MouseEvent) => {
      if (!wrapRef.current?.contains(e.target as Node)) setOpen(false);
    };
    document.addEventListener("mousedown", onDown);
    return () => document.removeEventListener("mousedown", onDown);
  }, []);

  async function addToCircle(label: string, neoId: string) {
    try {
      await api.addFriend(label, neoId, null, null);
      await refreshFriends();
      showToast(`Added ${label} to your circle`);
      setQuery("");
      setOpen(false);
    } catch (e) {
      showToast(toApiError(e).message);
    }
  }

  return (
    <div className="search-wrap" ref={wrapRef}>
      <span className="search-icon" aria-hidden="true">
        <SearchIcon />
      </span>
      <input
        ref={inputRef}
        className="search-input"
        type="text"
        placeholder="Search a name or Neo ID"
        value={query}
        aria-label="Search students"
        onChange={(e) => setQuery(e.target.value)}
        onFocus={() => query.trim().length >= 2 && setOpen(true)}
        onKeyDown={(e) => {
          if (e.key === "Escape") {
            e.stopPropagation();
            if (query) setQuery("");
            else (e.target as HTMLInputElement).blur();
            setOpen(false);
          }
        }}
      />

      {open && (
        <div className="popover" role="listbox" aria-label="Search results">
          {busy && <p className="pop-note">Searching…</p>}

          {!busy && byName?.kind === "found" && (
            <button
              className="pop-row"
              onClick={() => void addToCircle(byName.name, byName.neo_id)}
            >
              <span>
                <span className="person-name">{byName.name}</span>
                <span className="person-id">{byName.neo_id}</span>
              </span>
              <span className="pop-action">Add to circle</span>
            </button>
          )}

          {!busy && byName?.kind === "ambiguous" && (
            <>
              <p className="pop-note warn">
                More than one match — nankiv won't guess between them.
              </p>
              {byName.candidates.map((c) => (
                <button
                  key={c.neo_id}
                  className="pop-row"
                  onClick={() => void addToCircle(c.name, c.neo_id)}
                >
                  <span>
                    <span className="person-name">{c.name}</span>
                    <span className="person-id">{c.neo_id}</span>
                  </span>
                  <span className="pop-action">Add</span>
                </button>
              ))}
            </>
          )}

          {!busy && byName?.kind === "not_found" && (
            <p className="pop-note">
              No confident match. nankiv only knows students it has learned from
              files you've imported.
            </p>
          )}

          {!busy && byId && byId.length === 0 && (
            <p className="pop-note">
              Not on any shortlist you've imported. That only covers the drives
              in your history.
            </p>
          )}

          {!busy &&
            byId?.map((p, i) => (
              <div className="pop-row static" key={i}>
                <span className="person-name">{p.label}</span>
                <span className="pill yes">In</span>
              </div>
            ))}
        </div>
      )}
    </div>
  );
}
