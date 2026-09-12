/**
 * Onboarding — one step, not two.
 *
 * The previous flow was a wall before the door: it demanded identity before the
 * student had seen the app do anything, then asked for a "reference sheet" with
 * no context for what that was. The second step is gone; reference data is
 * offered in Settings, where it makes sense once you have seen a coverage
 * figure and understand why it matters.
 *
 * The registration number is no longer labelled "optional". It was, technically,
 * and that label buried the fact that two of the fifteen real shortlist formats
 * are keyed by it — without it, those files can never be answered.
 */

import { useEffect, useRef, useState } from "react";
import { useStore } from "../lib/store";

export function OnboardingScreen() {
  const { saveProfile, error, clearError } = useStore();
  const [neoId, setNeoId] = useState("");
  const [regNo, setRegNo] = useState("");
  const first = useRef<HTMLInputElement>(null);

  useEffect(() => {
    first.current?.focus();
  }, []);

  async function save() {
    clearError();
    await saveProfile({
      neo_id: neoId.trim() || null,
      reg_no: regNo.trim() || null,
      display_name: null,
      cohort: null,
      show_friend_cgpa: false,
    });
  }

  const ready = neoId.trim().length > 0 || regNo.trim().length > 0;

  return (
    <div className="onboard">
      <div className="onboard-mark" aria-hidden="true">
        <svg width="20" height="20" viewBox="0 0 24 24" fill="none">
          <path
            d="M5 12.5l4.5 4.5L19 7"
            stroke="currentColor"
            strokeWidth="3"
            strokeLinecap="round"
            strokeLinejoin="round"
          />
        </svg>
      </div>

      <h1>Who are you?</h1>
      <p className="page-sub">
        Saved on this machine only. It's what lets nankiv answer the moment a
        shortlist lands, instead of asking you to search one.
      </p>

      <div className="field">
        <label htmlFor="ob-neo">Neo ID</label>
        <input
          ref={first}
          id="ob-neo"
          type="text"
          className="mono-input"
          placeholder="V9H0G6C4"
          maxLength={8}
          value={neoId}
          onChange={(e) => setNeoId(e.target.value.toUpperCase())}
          onKeyDown={(e) => e.key === "Enter" && ready && void save()}
        />
        <span className="hint">
          Eight characters, alternating letter and digit.
        </span>
      </div>

      <div className="field">
        <label htmlFor="ob-reg">Registration number</label>
        <input
          id="ob-reg"
          type="text"
          className="mono-input"
          placeholder="23BAI0002"
          maxLength={11}
          value={regNo}
          onChange={(e) => setRegNo(e.target.value.toUpperCase())}
          onKeyDown={(e) => e.key === "Enter" && ready && void save()}
        />
        <span className="hint">
          Worth adding: some companies key their shortlists by this instead, and
          those files can't be answered for you without it.
        </span>
      </div>

      {error && <p className="field-error">{error.message}</p>}

      <button className="btn primary wide" onClick={save} disabled={!ready}>
        Continue
      </button>
    </div>
  );
}
