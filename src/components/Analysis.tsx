/**
 * Analytics rendering.
 *
 * Coverage is shown before any statistic, and an insufficient sample renders an
 * explanation instead of a number. No chart is drawn from a sample the engine
 * declined to characterise.
 */

import type {
  DriveAnalysis,
  CutoffReport,
  BranchReport,
  Distribution,
} from "../lib/api";

function pct(x: number): string {
  return `${Math.round(x * 100)}%`;
}

/** Never dismissible: every figure below it is a sample, not a census. */
export function CoverageNotice({ analysis }: { analysis: DriveAnalysis }) {
  const { matched_students: m, total_students: t, coverage } = analysis;
  const tone = coverage >= 0.4 ? "accent" : coverage >= 0.15 ? "" : "warn";
  return (
    <div className={`notice ${tone}`}>
      Based on <strong>{m.toLocaleString()}</strong> of{" "}
      <strong>{t.toLocaleString()}</strong> students matched to CGPA data (
      {pct(coverage)}).{coverage < 0.4 && " More reference sheets raise this."}
    </div>
  );
}

export function InsufficientSample({ analysis }: { analysis: DriveAnalysis }) {
  return (
    <div className="card">
      <h2>Not enough data to analyse</h2>
      <div className="notice warn">
        Only <strong>{analysis.matched_students}</strong> of{" "}
        <strong>{analysis.total_students}</strong> matched to CGPA data (
        {pct(analysis.coverage)}) — too few to estimate a cutoff.
      </div>
      <p style={{ fontSize: 13, color: "var(--text-3)", margin: 0 }}>
        The results above are exact; they come straight from the file.
      </p>
    </div>
  );
}

export function CutoffCard({ report }: { report: CutoffReport }) {
  const v = report.verdict.value;
  const headline =
    v.kind === "hard_cutoff"
      ? `CGPA cutoff around ${v.threshold.toFixed(1)}`
      : v.kind === "soft_preference"
        ? "Skews high, no hard cutoff"
        : "No CGPA filter detected";
  // The headline says what was found; this, only what it rests on. The full
  // statement, which says both, is for the copied summary.
  const detail =
    v.kind === "hard_cutoff"
      ? `Lowest found ${v.observed_floor.toFixed(2)}. An estimate, not an official cutoff.`
      : v.kind === "soft_preference"
        ? `Lowest found ${v.observed_floor.toFixed(2)}. Likely a preference, not a hard bar.`
        : "It looks like the batch as a whole.";

  return (
    <div className="card">
      <div className="card-head">
        <h2>{headline}</h2>
        <span className="pill quiet">estimate</span>
      </div>
      <p style={{ fontSize: 13.5, color: "var(--text-2)", margin: "0 0 14px" }}>
        {detail}
      </p>

      <h3 style={{ marginTop: 18 }}>How this was worked out</h3>
      <p style={{ fontSize: 12.5, color: "var(--text-3)", margin: "0 0 10px" }}>
        A cutoff is a floor: few on the list fall below it, while many in the
        batch do.
      </p>
      <div className="table-wrap">
        <table>
          <thead>
            <tr>
              <th>Threshold</th>
              <th style={{ textAlign: "right" }}>Below, shortlist</th>
              <th style={{ textAlign: "right" }}>Below, batch</th>
              <th>Signal</th>
            </tr>
          </thead>
          <tbody>
            {report.comparison.map((row) => (
              <tr key={row.threshold} className={row.is_signal ? "signal" : ""}>
                <td className="mono">{row.threshold.toFixed(1)}</td>
                <td className="num">{pct(row.share_below_shortlist)}</td>
                <td className="num">{pct(row.share_below_batch)}</td>
                <td>{row.is_signal ? "cutoff here" : "—"}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}

export function DistributionCard({
  dist,
  yourCgpa,
}: {
  dist: Distribution;
  yourCgpa: number | null;
}) {
  const max = Math.max(1, ...dist.buckets.map((b) => b.count));

  // Show only the occupied range, padded by one empty bucket either side, so a
  // tight distribution isn't squashed into a corner of an empty 6-to-10 axis.
  const firstUsed = dist.buckets.findIndex((b) => b.count > 0);
  const lastUsed =
    dist.buckets.length -
    1 -
    [...dist.buckets].reverse().findIndex((b) => b.count > 0);
  const from = Math.max(0, (firstUsed < 0 ? 0 : firstUsed) - 1);
  const to = Math.min(
    dist.buckets.length - 1,
    (firstUsed < 0 ? dist.buckets.length - 1 : lastUsed) + 1,
  );
  const shown = dist.buckets.slice(from, to + 1);

  // Axis labels are derived from the buckets actually drawn. Hardcoding them
  // produced a "8.5, 8.0, 9.0" axis that ran backwards.
  const axisLo = shown[0]?.lower ?? 6.0;
  const axisHi = shown[shown.length - 1]?.upper ?? 10.0;
  const ticks = [0, 0.5, 1].map((f) => axisLo + (axisHi - axisLo) * f);

  return (
    <div className="card">
      <div className="card-head">
        <h2>CGPA spread</h2>
        <span
          className="mono"
          style={{ fontSize: 11.5, color: "var(--text-faint)" }}
        >
          n={dist.n}
        </span>
      </div>

      <div
        className="hist"
        role="img"
        aria-label={`CGPA distribution: median ${dist.median.toFixed(2)}, range ${dist.min.toFixed(2)} to ${dist.max.toFixed(2)}${yourCgpa !== null ? `; yours is ${yourCgpa.toFixed(2)}` : ""}`}
      >
        {shown.map((b, i) => (
          <div
            className="hist-col"
            key={b.lower}
            title={`${b.lower.toFixed(2)}–${b.upper.toFixed(2)}: ${b.count}`}
          >
            {/* Each bar grows from the baseline, a beat after the one to its
                left. A chart that draws itself reads as measured rather than
                decorated — and it is the shape of the data doing the moving. */}
            <div
              className="hist-bar"
              style={{
                height: `${(b.count / max) * 100}%`,
                animationDelay: `${Math.min(i * 22, 260)}ms`,
              }}
            />
          </div>
        ))}
      </div>
      <div className="hist-axis">
        {ticks.map((t, i) => (
          <span
            key={t}
            style={{
              textAlign: i === 0 ? "left" : i === 1 ? "center" : "right",
            }}
          >
            {t.toFixed(2)}
          </span>
        ))}
      </div>

      <div className="stats" style={{ marginTop: 16 }}>
        <div className="stat">
          <div className="stat-value">{dist.min.toFixed(2)}</div>
          <div className="stat-label">Lowest found</div>
        </div>
        <div className="stat">
          <div className="stat-value">{dist.median.toFixed(2)}</div>
          <div className="stat-label">Median</div>
        </div>
        <div className="stat">
          <div className="stat-value">{dist.max.toFixed(2)}</div>
          <div className="stat-label">Highest</div>
        </div>
        {yourCgpa !== null && (
          <div className="stat">
            <div className="stat-value" style={{ color: "var(--accent)" }}>
              {yourCgpa.toFixed(2)}
            </div>
            <div className="stat-label">Yours</div>
          </div>
        )}
      </div>
    </div>
  );
}

export function BranchCard({ report }: { report: BranchReport }) {
  const rows = report.rows.value;
  const max = Math.max(1, ...rows.map((r) => r.count));

  return (
    <div className="card">
      <div className="card-head">
        <h2>Branches</h2>
        <span className="pill quiet">estimate</span>
      </div>
      <p style={{ fontSize: 13.5, color: "var(--text-2)", margin: "0 0 14px" }}>
        {report.statement}
      </p>
      <div className="table-wrap">
        <table>
          <thead>
            <tr>
              <th>Branch</th>
              <th style={{ textAlign: "right" }}>On list</th>
              <th style={{ textAlign: "right" }}>Share</th>
              <th style={{ textAlign: "right" }}>vs batch</th>
            </tr>
          </thead>
          <tbody>
            {rows.map((r) => (
              <tr
                key={r.branch}
                className={
                  report.over_represented.includes(r.branch) ? "signal" : ""
                }
              >
                <td>{r.branch}</td>
                <td className="num">
                  <span
                    aria-hidden="true"
                    style={{
                      display: "inline-block",
                      width: `${(r.count / max) * 40}px`,
                      height: 7,
                      background: "var(--accent)",
                      opacity: 0.35,
                      borderRadius: 2,
                      marginRight: 7,
                      verticalAlign: "middle",
                    }}
                  />
                  {r.count}
                </td>
                <td className="num">{pct(r.share)}</td>
                <td className="num">
                  {r.lift === null ? "—" : `${r.lift.toFixed(1)}x`}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}

/**
 * Where "Where you stand" goes until you have said your CGPA. nankiv will not
 * borrow one from the reference data: the identity it would come from is only
 * what was typed at setup, and could be anyone's.
 */
export function StandingPrompt({ onAdd }: { onAdd: () => void }) {
  return (
    <div className="card card-row">
      <div>
        <h2>Where you stand</h2>
        <p className="card-text">See how your CGPA compares with this list.</p>
      </div>
      <button className="btn small" onClick={onAdd}>
        Add your CGPA
      </button>
    </div>
  );
}

export function StandingCard({
  percentile,
}: {
  percentile: { value: number; matched: number; total: number };
}) {
  return (
    <div className="card">
      <h2>Where you stand</h2>
      <p style={{ fontSize: 13.5, color: "var(--text-2)", margin: 0 }}>
        Your CGPA is above <strong>{pct(percentile.value)}</strong> of this list
        ({percentile.matched} of {percentile.total} matched).
      </p>
    </div>
  );
}
