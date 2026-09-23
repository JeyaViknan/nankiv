/**
 * Verdict rendering.
 *
 * The four states — shortlisted, not shortlisted, undetermined, unreadable —
 * must never render alike. Each gets its own colour, its own icon and its own
 * wording, and the component switches exhaustively over the discriminated union
 * so a new state cannot be added without deciding how it looks.
 */

import type { Verdict, UndeterminedReason } from "../lib/api";
import { Icon } from "./Icon";
import { CountUp } from "./CountUp";

/** Plain-language explanation for why we can't answer. */
export function undeterminedText(r: UndeterminedReason): {
  title: string;
  detail: string;
} {
  switch (r.reason) {
    case "no_identity_configured":
      return {
        title: "We don't know who you are yet",
        detail: "Add your Neo ID in Settings and this will answer instantly.",
      };
    case "key_kind_not_configured":
      return r.file_key === "reg_no"
        ? {
            title: "Can't tell from this file",
            detail:
              "This shortlist is keyed by registration number, and you've only saved a Neo ID. Add your registration number to check files like this one.",
          }
        : {
            title: "Can't tell from this file",
            detail:
              "This shortlist is keyed by Neo ID, and you haven't saved one. Add it in Settings to check files like this one.",
          };
    case "file_not_understood":
      return {
        title: "Couldn't read this shortlist",
        detail:
          "It doesn't use Neo IDs or registration numbers, so there's nothing to match you against. This is not a result — it says nothing about whether you were shortlisted.",
      };
  }
}

interface BannerProps {
  verdict: Verdict;
  company: string;
  totalStudents: number;
  /** Opens Settings on the field that would answer the question. */
  onFix?: () => void;
}

/** The hero of the application: one unmistakable answer. */
export function VerdictBanner({
  verdict,
  company,
  totalStudents,
  onFix,
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
            {company} — <CountUp value={totalStudents} /> students shortlisted
          </p>
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
            You're not on the {company} shortlist of{" "}
            {totalStudents.toLocaleString()}. What it took is below.
          </p>
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
        <Icon name="check" size={24} /> In
      </span>
    );
  }
  if (verdict.status === "not_shortlisted") {
    return <span className="pill no">Not in</span>;
  }
  return <span className="pill unknown">Unknown</span>;
}
