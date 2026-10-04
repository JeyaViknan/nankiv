/**
 * The name in the title, which turns round once if you search for its maker.
 *
 * Each letter of "nankiv" flips over in turn to show "viknan", holds for a
 * moment, and turns back. Screen readers hear the name once, either way; with
 * reduced motion the letters simply swap.
 */

import { useEffect, useState } from "react";
import { useStore } from "../lib/store";

const NAME = "nankiv";
const BACKWARDS = [...NAME].reverse().join("");
const HOLD_MS = 2400;

export function Wordmark() {
  const turnedAt = useStore((s) => s.nameTurnedAt);
  const [turned, setTurned] = useState(false);

  useEffect(() => {
    if (!turnedAt) return;
    setTurned(true);
    const t = setTimeout(() => setTurned(false), HOLD_MS);
    return () => clearTimeout(t);
  }, [turnedAt]);

  return (
    <span className={`wordmark${turned ? " turned" : ""}`} aria-label={NAME}>
      {[...NAME].map((letter, i) => (
        <span
          className="wm-letter"
          key={i}
          aria-hidden="true"
          style={{ "--i": i } as React.CSSProperties}
        >
          <span className="wm-face">{letter}</span>
          <span className="wm-face wm-back">{BACKWARDS[i]}</span>
        </span>
      ))}
    </span>
  );
}
