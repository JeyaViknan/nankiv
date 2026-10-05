/**
 * Verdict rendering.
 *
 * The four states — shortlisted, not shortlisted, undetermined, unreadable —
 * must never render alike. Each gets its own colour, its own icon and its own
 * wording, and the component switches exhaustively over the discriminated union
 * so a new state cannot be added without deciding how it looks.
 */

import { useState } from "react";
import type { Evidence, Verdict, UndeterminedReason } from "../lib/api";
import { explainVerdict, hasExcerpt } from "../lib/evidence";
import { Excerpt } from "./Excerpt";
import { Icon } from "./Icon";
import { CountUp } from "./CountUp";

/**
 * The working behind an answer, one click away. Closed by default: the answer
 * is the news, and the evidence is for whoever wants to check it. A yes shows
 * the file around your line; anything else says it in a sentence.
 *
 * A yes from before positions were kept has no lines to show until its file
 * is found again, so opening it asks for them once, and says the sentence
 * only if the file is gone.
 */
function Why({
  question,
  verdict,
  evidence: given,
  findEvidence,
}: {
  question: string;
  verdict: Verdict;
  evidence?: Evidence;
  findEvidence?: () => Promise<Evidence>;
}) {
  const [found, setFound] = useState<Evidence | null>(null);
  const [looking, setLooking] = useState(false);
  const [looked, setLooked] = useState(false);
  const evidence = found ?? given;
  if (!evidence) return null;

  const yes = verdict.status === "shortlisted";
  const shown = yes && hasExcerpt(evidence);
  const text = shown ? null : explainVerdict(verdict, evidence);
  if (!shown && !text) return null;

  function onToggle(e: React.SyntheticEvent<HTMLDetailsElement>) {
    if (!e.currentTarget.open || shown || !yes || looked || !findEvidence) {
      return;
    }
    setLooked(true);
    setLooking(true);
    findEvidence()
      .then(setFound)
      .catch(() => {})
      .finally(() => setLooking(false));
  }

  return (
    <details className="why" onToggle={onToggle}>
      <summary>
        {question}
        <Icon name="chevronRight" size={11} className="why-chevron" />
      </summary>
      {shown ? (
        <Excerpt evidence={evidence} />
      ) : looking ? (
        <p className="why-looking" role="status">
          Finding your row…
        </p>
      ) : (
        <p>{text}</p>
      )}
    </details>
  );
}

/** Plain-language explanation for why we can't answer. */
export function undeterminedText(r: UndeterminedReason): {
  title: string;
  detail: string;
} {
  switch (r.reason) {
    case "no_identity_configured":
      return {
        title: "We don't know who you are yet",
        detail: "Add your Neo ID to get an answer.",
      };
    case "key_kind_not_configured":
      return r.file_key === "reg_no"
        ? {
            title: "Can't tell from this file",
            detail:
              "This list uses registration numbers. Add yours to check it.",
          }
        : {
            title: "Can't tell from this file",
            detail: "This list uses Neo IDs. Add yours to check it.",
          };
    case "file_not_understood":
      return {
        title: "Couldn't read this shortlist",
        detail:
          "It has no Neo IDs or registration numbers to match. Not a result — this says nothing about whether you're in.",
      };
  }
}

interface BannerProps {
  verdict: Verdict;
  company: string;
  totalStudents: number;
  /** Opens Settings on the field that would answer the question. */
  onFix?: () => void;
  /** What the answer rests on. */
  evidence?: Evidence;
  /** Finds the file again for a yes from before positions were kept. */
  findEvidence?: () => Promise<Evidence>;
  /** Offered only for a yes: makes an "I'm in" card. */
  onShare?: () => void;
}

/** The hero of the application: one unmistakable answer. */
export function VerdictBanner({
  verdict,
  company,
  totalStudents,
  onFix,
  evidence,
  findEvidence,
  onShare,
}: BannerProps) {
  // The answer people hope for gets the room, the motion and the one flourish
  // in the application. The other two are deliberately quieter: a rejection
  // that arrives with a fanfare is cruel, and "can't tell" is a task, not news.
  if (verdict.status === "shortlisted") {
    return (
      <div className="verdict yes land" role="status">
        <div className="verdict-icon">
          <Icon name="check" size={30} draw />
        </div>
        <div>
          <p className="verdict-title">You're in</p>
          <p className="verdict-detail">
            {company} — <CountUp value={totalStudents} /> shortlisted
          </p>
          <div className="verdict-links">
            <Why
              question="Why does it think I'm in?"
              verdict={verdict}
              evidence={evidence}
              findEvidence={findEvidence}
            />
            {onShare && (
              <button className="link-button share-link" onClick={onShare}>
                Share
              </button>
            )}
          </div>
        </div>
      </div>
    );
  }

  if (verdict.status === "not_shortlisted") {
    return (
      <div className="verdict no land" role="status">
        <div className="verdict-icon">
          <Icon name="dash" size={20} />
        </div>
        <div>
          <p className="verdict-title">Not this time</p>
          <p className="verdict-detail">
            {company} — {totalStudents.toLocaleString()} shortlisted
          </p>
          <Why question="Why not?" verdict={verdict} evidence={evidence} />
        </div>
      </div>
    );
  }

  const { title, detail } = undeterminedText(verdict);
  // Every reason here is something the student can fix in one place, so the
  // banner carries the way to fix it rather than describing it.
  const fixable = verdict.reason !== "file_not_understood";
  return (
    <div className="verdict unknown land" role="status">
      <div className="verdict-icon">
        <Icon name="question" size={22} />
      </div>
      <div>
        <p className="verdict-title">{title}</p>
        <p className="verdict-detail">{detail}</p>
        {fixable && onFix && (
          <button className="btn small verdict-fix" onClick={onFix}>
            {verdict.reason === "key_kind_not_configured" &&
            verdict.file_key === "reg_no"
              ? "Add registration number"
              : "Add Neo ID"}
          </button>
        )}
      </div>
    </div>
  );
}

/**
 * Compact verdict pill for friend rows.
 *
 * Text is never colour-only: each pill states its status in words as well.
 */
export function VerdictPill({ verdict }: { verdict: Verdict }) {
  if (verdict.status === "shortlisted") {
    return (
      <span className="pill yes">
        <Icon name="check" size={12} weight="semibold" /> In
      </span>
    );
  }
  if (verdict.status === "not_shortlisted") {
    return <span className="pill no">Not in</span>;
  }
  return <span className="pill unknown">Unknown</span>;
}
