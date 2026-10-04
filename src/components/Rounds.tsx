/**
 * A drive's rounds, as one line: R1 ✓ → R2 ✓ → Next ?
 *
 * Rounds are linked by the core, and only when a later list was clearly drawn
 * from an earlier one at the same company — two roles at one company are left
 * apart. Each step carries your answer in it, the round you are looking at is
 * marked, and every other step opens its own drive. "Next ?" appears only when
 * you are in the latest round and nothing says that round was the last.
 */

import type { Progression, RoundStep, Verdict } from "../lib/api";
import { Icon } from "./Icon";

function mark(verdict: Verdict): { glyph: JSX.Element; said: string } {
  switch (verdict.status) {
    case "shortlisted":
      return {
        glyph: <Icon name="check" size={11} weight="semibold" />,
        said: "you're in",
      };
    case "not_shortlisted":
      return { glyph: <Icon name="dash" size={11} />, said: "not this time" };
    case "undetermined":
      return { glyph: <Icon name="question" size={11} />, said: "can't tell" };
  }
}

function Step({
  step,
  current,
  onOpen,
}: {
  step: RoundStep;
  current: boolean;
  onOpen: (id: number) => void;
}) {
  const { glyph, said } = mark(step.verdict);
  const className = `round-step ${step.verdict.status}${current ? " current" : ""}`;
  const label = `${step.label}, ${said}`;
  return current ? (
    <span className={className} aria-current="step" aria-label={label}>
      {step.label}
      {glyph}
    </span>
  ) : (
    <button
      className={className}
      onClick={() => onOpen(step.drive_id)}
      aria-label={`${label} — open`}
    >
      {step.label}
      {glyph}
    </button>
  );
}

export function RoundTrail({
  progression,
  current,
  onOpen,
}: {
  progression: Progression;
  current: number;
  onOpen: (id: number) => void;
}) {
  return (
    <ol className="round-trail" aria-label="Rounds">
      {progression.steps.map((step, i) => (
        <li key={step.drive_id}>
          {i > 0 && (
            <span className="round-arrow" aria-hidden="true">
              →
            </span>
          )}
          <Step
            step={step}
            current={step.drive_id === current}
            onOpen={onOpen}
          />
        </li>
      ))}
      {progression.next_pending && (
        <li>
          <span className="round-arrow" aria-hidden="true">
            →
          </span>
          <span
            className="round-step next"
            aria-label="Next round, still to come"
          >
            Next
            <Icon name="question" size={11} />
          </span>
        </li>
      )}
    </ol>
  );
}

/** The round before this one, to compare against, or after it if it's first. */
export function neighbour(
  progression: Progression,
  current: number,
): { step: RoundStep; earlier: boolean } | null {
  const i = progression.steps.findIndex((s) => s.drive_id === current);
  if (i < 0) return null;
  const before = progression.steps[i - 1];
  if (before) return { step: before, earlier: true };
  const after = progression.steps[i + 1];
  return after ? { step: after, earlier: false } : null;
}
