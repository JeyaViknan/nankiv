/**
 * The lines of the file around yours, the way the file has them.
 *
 * "Why does it think I'm in?" is answered by showing, not telling: the column
 * as it was headed, the rows on either side by their numbers, and yours lit
 * the moment the excerpt opens. Nothing moves but that light. It is the same
 * place the student would find opening the file, which is the point: they can
 * check it.
 */

import type { Evidence } from "../lib/api";
import { PASTED_SHEET, hasExcerpt, keyName } from "../lib/evidence";
import { Icon } from "./Icon";

/** The excerpt behind a yes. Renders nothing without `hasExcerpt`. */
export function Excerpt({ evidence }: { evidence: Evidence }) {
  const at = evidence.found_at;
  if (!hasExcerpt(evidence) || !at || !evidence.key) return null;

  const key = keyName(evidence.key);
  const pasted = at.sheet === PASTED_SHEET;
  // Sheet names arrive as typed, invisible characters and stray spaces too.
  const sheet = at.sheet.replace(/[\u200b-\u200d\ufeff]/g, "").trim();
  const where = pasted
    ? "What you pasted"
    : [sheet, at.column && `Column ${at.column}`].filter(Boolean).join(" · ");
  const heading = at.header ?? key.charAt(0).toUpperCase() + key.slice(1);

  return (
    <div className="excerpt">
      <p className="excerpt-where">{where}</p>
      <div
        className="excerpt-sheet"
        role="table"
        aria-label={`${where}, around ${pasted ? "line" : "row"} ${at.row}`}
      >
        <div className="excerpt-line head" role="row">
          <span className="excerpt-n" role="columnheader">
            <span className="sr-only">{pasted ? "Line" : "Row"}</span>
          </span>
          <span role="columnheader">{heading}</span>
        </div>
        {evidence.excerpt.map((line) => {
          const yours = line.row === at.row && line.value === at.value;
          return (
            <div
              key={line.row}
              className={`excerpt-line${yours ? " you" : ""}`}
              role="row"
              aria-current={yours || undefined}
            >
              <span className="excerpt-n" role="cell">
                {line.row}
              </span>
              <span className="excerpt-id" role="cell">
                {line.value}
                {yours && <span className="excerpt-you">You</span>}
              </span>
            </div>
          );
        })}
      </div>
      <p className="excerpt-basis">
        <Icon name="check" size={12} weight="semibold" />
        Matched by {key}
      </p>
    </div>
  );
}
