/**
 * The student's own CGPA, as typed.
 *
 * It is the only CGPA nankiv ever shows anyone, so it is read strictly: a
 * figure out of ten, to two places. A percentage typed by mistake is refused
 * rather than quietly divided, because a wrong standing is worse than none.
 */

export type CgpaInput =
  { kind: "empty" } | { kind: "valid"; value: number } | { kind: "invalid" };

export function readCgpa(text: string): CgpaInput {
  const t = text.trim();
  if (t === "") return { kind: "empty" };
  if (!/^\d{1,2}(\.\d{1,2})?$/.test(t)) return { kind: "invalid" };
  const value = Number(t);
  return value > 0 && value <= 10
    ? { kind: "valid", value }
    : { kind: "invalid" };
}

/** "Updated 4 Oct", from the core's UTC timestamp. */
export function cgpaUpdatedLabel(stamp: string | null): string | null {
  if (!stamp) return null;
  const when = new Date(stamp.replace(" ", "T") + "Z");
  if (Number.isNaN(when.getTime())) return null;
  const sameYear = when.getFullYear() === new Date().getFullYear();
  return `Updated ${when.toLocaleDateString(undefined, {
    day: "numeric",
    month: "short",
    ...(sameYear ? {} : { year: "numeric" }),
  })}`;
}
