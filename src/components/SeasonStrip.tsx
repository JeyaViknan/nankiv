/**
 * How the season is going, where the window title used to be.
 *
 * The word "Shortlists" sat above a list of shortlists and said nothing the
 * screen did not already say — and it never changed, on the one part of the
 * window that is always visible. This is the same real estate spent on the only
 * thing that moves: your own count, from the same tally the widget shows.
 *
 * Counts are stated, never celebrated. Four of twelve is four of twelve.
 */

import type { Season } from "../lib/api";

export function SeasonStrip({ season }: { season: Season | null }) {
  // Before the first import there is nothing to count, and a row of zeroes
  // reads as failure rather than as a beginning.
  if (!season || season.drives === 0) {
    return <span className="wordmark">nankiv</span>;
  }

  const parts: { key: string; tone: string; value: number; label: string }[] = [
    { key: "in", tone: "yes", value: season.shortlisted, label: "in" },
    { key: "not", tone: "no", value: season.not_shortlisted, label: "not in" },
  ];
  if (season.undetermined > 0) {
    parts.push({
      key: "unknown",
      tone: "unknown",
      value: season.undetermined,
      label: "unknown",
    });
  }

  return (
    <div
      className="season"
      aria-label={`This season: ${season.drives} shortlists`}
    >
      <span className="wordmark">nankiv</span>
      <span className="season-rule" aria-hidden="true" />
      {parts.map((p) => (
        <span className={`season-part ${p.tone}`} key={p.key}>
          <span className="season-value">{p.value}</span>
          <span className="season-label">{p.label}</span>
        </span>
      ))}
    </div>
  );
}
