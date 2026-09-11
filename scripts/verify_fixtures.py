#!/usr/bin/env python3
"""Privacy guard: assert that committed fixtures contain no real student data.

Run against the real source files on a maintainer's machine. It compares every
identifier and name in `src-tauri/tests/fixtures/` against everything present in
the real shortlists and reference sheets, and fails if anything overlaps.

Structural strings — column headers like `Neo ID`, `USN`, `Venue` — are
deliberately preserved in fixtures, because they are what the parser is being
tested against. They are not personal data and are excluded from the comparison.

Usage:
    python3 scripts/verify_fixtures.py ~/Downloads Global

Exits non-zero on any leak, so it can gate CI or a pre-commit hook.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

try:
    import openpyxl
except ImportError:
    sys.exit("openpyxl required:  pip install openpyxl")

NEO_RE = re.compile(r"^(?:[A-Z][0-9]){4}$")
REG_RE = re.compile(r"^\d{2}[A-Z]{3}\d{4,5}$")

FIXTURES = Path("src-tauri/tests/fixtures")

# Column headers and structural labels. Preserved on purpose — they carry the
# layout quirks the parser must survive, and identify no one.
STRUCTURAL = {
    "ac - title", "batch", "branch", "brach", "campus", "candidate id",
    "candidate name", "count", "degree", "email", "email id", "full name",
    "gender", "interview date", "lab no", "mobile number", "name", "neo id",
    "phone no", "reference_id", "reg.no", "register number", "registration number",
    "remarks", "resume link", "role", "s.no", "sno", "student name", "timestamp",
    "username", "usn", "venue",
}


def harvest(path: Path) -> tuple[set[str], set[str]]:
    """Returns (identifiers, names) found in a workbook."""
    ids: set[str] = set()
    names: set[str] = set()
    try:
        wb = openpyxl.load_workbook(path, read_only=True, data_only=True)
    except Exception as exc:  # noqa: BLE001 - a bad file should not abort the scan
        print(f"  ! could not read {path.name}: {exc}", file=sys.stderr)
        return ids, names

    for ws in wb.worksheets:
        for row in ws.iter_rows(values_only=True):
            for cell in row:
                if cell is None:
                    continue
                text = str(cell).strip()
                if not text:
                    continue
                upper = text.upper()
                if NEO_RE.match(upper) or REG_RE.match(upper):
                    ids.add(upper)
                    continue
                lower = text.lower()
                if lower in STRUCTURAL:
                    continue
                letters = sum(c.isalpha() for c in text)
                if (
                    3 <= len(text) <= 60
                    and not any(c.isdigit() for c in text)
                    and letters >= 3
                    and letters / len(text) > 0.7
                ):
                    names.add(lower)
    wb.close()
    return ids, names


def main() -> None:
    sources: list[Path] = []
    for arg in sys.argv[1:]:
        d = Path(arg).expanduser()
        if d.is_dir():
            sources += [p for p in d.glob("*.xlsx")]
        elif d.is_file():
            sources.append(d)

    if not FIXTURES.is_dir():
        sys.exit(f"no fixtures directory at {FIXTURES}")
    if not sources:
        print("No real source files supplied — nothing to compare against.")
        print("This is expected in CI, where the real data is absent. Skipping.")
        return

    real_ids: set[str] = set()
    real_names: set[str] = set()
    for p in sources:
        i, n = harvest(p)
        real_ids |= i
        real_names |= n

    fx_ids: set[str] = set()
    fx_names: set[str] = set()
    for p in sorted(FIXTURES.glob("*.xlsx")):
        i, n = harvest(p)
        fx_ids |= i
        fx_names |= n

    print(f"real data : {len(real_ids):>6} identifiers, {len(real_names):>5} names")
    print(f"fixtures  : {len(fx_ids):>6} identifiers, {len(fx_names):>5} names")

    leaked_ids = sorted(fx_ids & real_ids)
    leaked_names = sorted(fx_names & real_names)

    if leaked_ids or leaked_names:
        print(f"\nFAIL — fixtures contain real data")
        if leaked_ids:
            print(f"  {len(leaked_ids)} real identifiers: {leaked_ids[:10]}")
        if leaked_names:
            print(f"  {len(leaked_names)} real names: {leaked_names[:10]}")
        print("\nRegenerate with scripts/make_fixtures.py and re-run this check.")
        sys.exit(1)

    print("\nPASS — no real identifiers or names appear in the fixtures.")


if __name__ == "__main__":
    main()
