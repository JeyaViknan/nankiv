/**
 * Small surprises, kept small.
 *
 * Each one is rare, says one line, and gets out of the way: none blocks
 * anything, none appears twice in a row, and none ever jokes about a
 * rejection. They respect reduced motion like everything else.
 */

/** "nankiv" is "viknan" backwards. Searching the name turns the title round. */
export function isTheName(query: string): boolean {
  return query.trim().toLowerCase() === "viknan";
}

const PHOTO = /\.(png|jpe?g|heic|heif|gif|webp|tiff?|bmp|avif)$/i;
const RESUME = /(r[eé]sum[eé]|curriculum|(^|[^a-z])cv([^a-z]|$))/i;

/** What to say about a file that isn't a spreadsheet, when there's a better line. */
export function wrongFileReply(
  name: string,
): { title: string; reason: string | null } | null {
  if (/\.pdf$/i.test(name) && RESUME.test(name)) {
    return {
      title: "That's a résumé, not a shortlist.",
      reason: "Good luck with it though.",
    };
  }
  if (PHOTO.test(name)) {
    return {
      title: "Nice picture. Still not a shortlist.",
      reason: "nankiv reads spreadsheets — .xlsx, .xls or .csv.",
    };
  }
  return null;
}

/** Between 1 and 5 in the morning, a gentle word under a fresh result. */
export function lateNightLine(at: Date): string | null {
  const h = at.getHours();
  if (h < 1 || h >= 5) return null;
  const mm = String(at.getMinutes()).padStart(2, "0");
  return `It's ${h}:${mm} AM. The list will say the same thing in the morning.`;
}
