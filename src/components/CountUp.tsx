/**
 * A number that arrives rather than appears.
 *
 * Only for figures that were just computed — a shortlist's size as the result
 * lands. Everywhere else a number is simply printed: a count that animates
 * every time it is rendered is a fidget, not feedback.
 *
 * Monospaced digits, so the text does not reflow as it climbs.
 */

import { useEffect, useRef, useState } from "react";

/** Fast at first, still at the end: the shape of something settling. */
function ease(t: number): number {
  return 1 - Math.pow(1 - t, 3);
}

export function CountUp({
  value,
  duration = 420,
}: {
  value: number;
  duration?: number;
}) {
  const [shown, setShown] = useState(value);
  const frame = useRef<number>(0);

  useEffect(() => {
    const reduced = window.matchMedia?.(
      "(prefers-reduced-motion: reduce)",
    ).matches;
    // Small numbers have nowhere to climb from, and someone who has asked for
    // less motion has asked for this too.
    if (reduced || value < 10) {
      setShown(value);
      return;
    }

    const start = performance.now();
    const step = (now: number) => {
      const t = Math.min(1, (now - start) / duration);
      setShown(Math.round(ease(t) * value));
      if (t < 1) frame.current = requestAnimationFrame(step);
    };
    frame.current = requestAnimationFrame(step);
    return () => cancelAnimationFrame(frame.current);
  }, [value, duration]);

  return <span className="tabular">{shown.toLocaleString()}</span>;
}
