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
}

/** The hero of the application: one unmistakable answer. */
export function VerdictBanner({
  verdict,
  company,
  totalStudents,
}: BannerProps) {
  if (verdict.status === "shortlisted") {
    return (
      <div className="verdict yes" role="status">
        <div className="verdict-icon">
          <Icon name="check" size={24} />
        </div>
        <div>
          <p className="verdict-title">You're in</p>
          <p className="verdict-detail">
            {company} — {totalStudents.toLocaleString()} students shortlisted
          </p>
        </div>
      </div>
    );
  }

  if (verdict.status === "not_shortlisted") {
    return (
      <div className="verdict no" role="status">
        <div className="verdict-icon">
          <Icon name="dash" size={24} />
        </div>
        <div>
          <p className="verdict-title">Not this time</p>
          <p className="verdict-detail">
            You're not on the {company} shortlist of{" "}
            {totalStudents.toLocaleString()}.
          </p>
        </div>
      </div>
    );
  }

  const { title, detail } = undeterminedText(verdict);
  return (
    <div className="verdict unknown" role="status">
      <div className="verdict-icon">
        <Icon name="question" size={24} />
      </div>
      <div>
        <p className="verdict-title">{title}</p>
        <p className="verdict-detail">{detail}</p>
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
