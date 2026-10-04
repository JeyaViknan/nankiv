/**
 * What a screen reader is told as it happens: a result arriving, a drop
 * refused. The result screen appears all at once, and VoiceOver does not read
 * a region just because it appeared; this says it in one sentence.
 *
 * Each announcement replaces the last, and is re-mounted so that the same
 * words twice are still said twice.
 */

import { useStore } from "../lib/store";

export function Announcer() {
  const announcement = useStore((s) => s.announcement);
  return (
    <div className="sr-only" role="status" aria-live="polite">
      {announcement && <span key={announcement.n}>{announcement.text}</span>}
    </div>
  );
}
