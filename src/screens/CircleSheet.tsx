/**
 * Circle — managed in a sheet.
 *
 * The previous build required a Neo ID to add anyone, which meant the product's
 * own premise went unsolved in its own interface: nankiv exists because nobody
 * can remember an eight-character alphanumeric code, and it was asking for one.
 * A name search already existed elsewhere in the app and was not wired up here.
 *
 * So the primary path is now: type a name, pick the person, done. Typing an ID
 * still works for anyone who has it — the field accepts either and decides for
 * itself, rather than making the student choose a mode first.
 */

import { useCallback, useEffect, useRef, useState } from "react";
import { api, toApiError, type SearchResult } from "../lib/api";
import { useStore } from "../lib/store";
import { Sheet } from "../components/Sheet";
import { Icon } from "../components/Icon";

const NEO = /^(?:[A-Za-z][0-9]){4}$/;
const REG = /^\d{2}[A-Za-z]{3}\d{4,5}$/;

function looksLikeIdentifier(q: string): boolean {
  return NEO.test(q.trim()) || REG.test(q.trim());
}

export function CircleSheet({ onClose }: { onClose: () => void }) {
  const { friends, refreshFriends, showToast } = useStore();
  const [query, setQuery] = useState("");
  const [result, setResult] = useState<SearchResult | null>(null);
  const [searching, setSearching] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  // Live search, debounced. Pressing a button to search something is a
  // holdover from page-reload interfaces.
  useEffect(() => {
    const q = query.trim();
    setError(null);
    if (q.length < 2 || looksLikeIdentifier(q)) {
      setResult(null);
      return;
    }
    setSearching(true);
    const t = setTimeout(async () => {
      try {
        setResult(await api.searchStudents(q));
      } catch (e) {
        setError(toApiError(e).message);
      } finally {
        setSearching(false);
      }
    }, 180);
    return () => {
      clearTimeout(t);
      setSearching(false);
    };
  }, [query]);

  const add = useCallback(
    async (label: string, neoId: string | null, regNo: string | null) => {
      try {
        await api.addFriend(label, neoId, regNo, null);
        await refreshFriends();
        setQuery("");
        setResult(null);
        showToast(`Added ${label}`);
        inputRef.current?.focus();
      } catch (e) {
        setError(toApiError(e).message);
      }
    },
    [refreshFriends, showToast],
  );

  async function addFromIdentifier() {
    const q = query.trim().toUpperCase();
    if (!looksLikeIdentifier(q)) return;
    await add(q, NEO.test(q) ? q : null, REG.test(q) ? q : null);
  }

  async function remove(id: number, label: string) {
    await api.removeFriend(id);
    await refreshFriends();
    showToast(`Removed ${label}`);
  }

  const identifierMode = looksLikeIdentifier(query);

  return (
    <Sheet title="Circle" onClose={onClose} width={520}>
      <p className="sheet-lede">
        Added once, then checked on every shortlist. Search by name — you don't
        need to know anyone's Neo ID.
      </p>

      <div className="search-add">
        <input
          ref={inputRef}
          type="text"
          placeholder="Search a name, or paste a Neo ID"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter" && identifierMode) void addFromIdentifier();
          }}
          aria-label="Search for someone to add"
        />
        {identifierMode && (
          <button className="btn primary" onClick={addFromIdentifier}>
            Add
          </button>
        )}
      </div>

      {error && <p className="field-error">{error}</p>}

      {searching && query.trim().length >= 2 && !identifierMode && (
        <p className="search-note">Searching…</p>
      )}

      {result?.kind === "found" && (
        <div className="result-list">
          <button
            className="result-row"
            onClick={() => void add(result.name, result.neo_id, null)}
          >
            <span>
              <span className="person-name">{result.name}</span>
              <span className="person-id">{result.neo_id}</span>
            </span>
            <span className="result-add">
              <Icon name="plus" size={14} /> Add
            </span>
          </button>
        </div>
      )}

      {result?.kind === "ambiguous" && (
        <>
          <p className="search-note warn">
            More than one student matches that name. nankiv won't guess — pick
            the right one.
          </p>
          <div className="result-list">
            {result.candidates.map((c) => (
              <button
                key={c.neo_id}
                className="result-row"
                onClick={() => void add(c.name, c.neo_id, null)}
              >
                <span>
                  <span className="person-name">{c.name}</span>
                  <span className="person-id">{c.neo_id}</span>
                </span>
                <span className="result-add">
                  <Icon name="plus" size={14} /> Add
                </span>
              </button>
            ))}
          </div>
        </>
      )}

      {result?.kind === "not_found" && (
        <p className="search-note">
          No confident match. nankiv only knows students it has learned from
          files you've imported — if you have their Neo ID, paste it above.
        </p>
      )}

      <div className="sheet-section">
        <div className="list-head">
          <span className="eyebrow">Tracked</span>
          <span className="list-count">
            {friends.length} {friends.length === 1 ? "person" : "people"}
          </span>
        </div>

        {friends.length === 0 ? (
          <p className="search-note">
            Nobody yet. Everyone you add is checked automatically on every
            shortlist you import.
          </p>
        ) : (
          <div className="list">
            {friends.map((f) => (
              <div className="row" key={f.id}>
                <span className="row-main static">
                  <span className="row-title">{f.label}</span>
                  <span className="row-meta mono">{f.neo_id ?? f.reg_no}</span>
                </span>
                <button
                  className="row-action"
                  aria-label={`Remove ${f.label}`}
                  title="Remove"
                  onClick={() => void remove(f.id, f.label)}
                >
                  <Icon name="close" size={14} />
                </button>
              </div>
            ))}
          </div>
        )}
      </div>
    </Sheet>
  );
}
