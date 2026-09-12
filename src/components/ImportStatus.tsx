/**
 * The moment between the drop and the answer.
 *
 * Two states earn a place on screen here, and each exists because the previous
 * build handled it badly:
 *
 *   reading  the file is acknowledged **by name** before anything is computed,
 *            so the student knows the gesture landed. Previously a spinner
 *            appeared inside the drop zone — the one element they had just
 *            stopped looking at — and then the view jumped.
 *
 *   failed   an explanation, not an alarm. A file nankiv cannot read is not a
 *            rejection, and must never be styled like one; the wrong colour
 *            here would read as "you are not shortlisted".
 */

import { useStore } from "../lib/store";

function AlertIcon() {
  return (
    <svg
      width="17"
      height="17"
      viewBox="0 0 20 20"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.6"
      strokeLinecap="round"
      aria-hidden="true"
    >
      <circle cx="10" cy="10" r="7.5" />
      <path d="M10 6.2v4.4" />
      <circle cx="10" cy="13.8" r="0.9" fill="currentColor" stroke="none" />
    </svg>
  );
}

export function ImportStatus() {
  const { importStage, dismissImport } = useStore();

  if (importStage.phase === "reading") {
    return (
      <div className="import-status reading" role="status" aria-live="polite">
        <span className="progress-bar" aria-hidden="true">
          <span className="progress-fill" />
        </span>
        <span className="import-text">
          Reading <span className="import-file">{importStage.filename}</span>
        </span>
      </div>
    );
  }

  if (importStage.phase === "failed") {
    const { error, filename } = importStage;
    return (
      <div className="import-status failed" role="alert">
        <span className="import-icon">
          <AlertIcon />
        </span>
        <div className="import-body">
          <p className="import-headline">{error.message}</p>
          <p className="import-file-line">{filename}</p>
          {error.detail && <p className="import-detail">{error.detail}</p>}
          {/* Said plainly, because silence here reads as a verdict. */}
          <p className="import-reassure">
            This isn't a result — it says nothing about whether you were
            shortlisted.
          </p>
        </div>
        <button className="btn small" onClick={dismissImport}>
          Dismiss
        </button>
      </div>
    );
  }

  return null;
}
