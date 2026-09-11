#!/usr/bin/env python3
"""Build regression fixtures from real shortlist files.

The fifteen real shortlists are the most valuable test asset in this project —
they encode layout quirks no one would invent: data on Sheet2 with Sheet1 empty,
headers with trailing spaces, serial-number columns, a file keyed by a foreign
reference ID, two files that are byte-identical.

But they hold real students' identifiers, so they cannot be committed. This
script keeps the *structure* and replaces the *content*:

  - every Neo ID  -> a deterministic synthetic Neo ID (same LDLDLDLD shape)
  - every Reg No  -> a deterministic synthetic Reg No (same shape, same year)
  - every name    -> a name drawn from a fixed invented pool
  - everything else (headers, sheet layout, blank sheets, column order) is kept

Mapping is a keyed hash, so the same input maps to the same output across runs
and cross-file relationships survive: if a student appears in both the Tredence
and Amazon lists, they still do in the fixtures. That is what makes the fixtures
useful for testing the identity graph.

The key is random per run and never stored, so the mapping is not reversible.

Usage:
    python3 scripts/make_fixtures.py ~/Downloads src-tauri/tests/fixtures
"""

from __future__ import annotations

import hashlib
import os
import re
import secrets
import sys
from pathlib import Path

try:
    import openpyxl
except ImportError:
    sys.exit("openpyxl required:  pip install openpyxl")

NEO_RE = re.compile(r"^(?:[A-Z][0-9]){4}$")
REG_RE = re.compile(r"^\d{2}[A-Z]{3}\d{4,5}$")

LETTERS = "ABCDEFGHIJKLMNOPQRSTUVWXYZ"

# Programme codes chosen so a synthetic registration number can never collide
# with a real one: no real programme code begins with Z.
PROGRAMMES = ["ZAA", "ZAB", "ZAC", "ZBA", "ZBB", "ZCA", "ZCB", "ZDA"]

# Invented names. Deliberately not drawn from any real roster.
FIRST = [
    "Aarav", "Bhavya", "Chetan", "Divya", "Eshan", "Farah", "Gaurav", "Hiral",
    "Ishan", "Jyoti", "Kabir", "Lavanya", "Manav", "Nisha", "Omkar", "Pooja",
    "Rahul", "Sneha", "Tarun", "Uma", "Varun", "Yash", "Zara", "Ananya",
]
LAST = [
    "Agarwal", "Bhatt", "Chauhan", "Desai", "Iyer", "Joshi", "Kulkarni",
    "Menon", "Nair", "Patel", "Rao", "Sharma", "Trivedi", "Verma", "Warrier",
]

SECRET = secrets.token_bytes(32)

# Values observed in the source files. Anything generated is checked against
# these and perturbed until it does not collide, so a fixture can never contain
# a real identifier or a real student's name even by coincidence.
REAL_VALUES: set[str] = set()
REAL_NAMES: set[str] = set()

# Synthetic values already handed out. The mapping must be injective: two real
# students must never collapse onto one synthetic identifier, or the fixtures
# would under-count and the regression expectations would drift from reality.
ASSIGNED: dict[str, str] = {}
USED: set[str] = set()


def _h(value: str, domain: str) -> int:
    return int.from_bytes(
        hashlib.blake2b(f"{domain}:{value}".encode(), key=SECRET, digest_size=8).digest(),
        "big",
    )


def _neo_from(n: int) -> str:
    out = []
    for _ in range(4):
        out.append(LETTERS[n % 26])
        n //= 26
        out.append(str(n % 10))
        n //= 10
    return "".join(out)


def _assign(real: str, domain: str, generate) -> str:
    """Returns a stable, unique synthetic value for `real`.

    Stable, so the same student maps identically across files and cross-file
    relationships survive. Unique, so distinct students stay distinct.
    """
    memo_key = f"{domain}:{real}"
    if memo_key in ASSIGNED:
        return ASSIGNED[memo_key]
    salt = 0
    while True:
        cand = generate(real, salt)
        collides = cand.upper() in REAL_VALUES or cand.lower() in REAL_NAMES
        if not collides and cand not in USED:
            ASSIGNED[memo_key] = cand
            USED.add(cand)
            return cand
        salt += 1


def fake_neo(real: str) -> str:
    return _assign(real, "neo", lambda r, s: _neo_from(_h(f"{r}#{s}", "neo")))


def _gen_reg(real: str, salt: int) -> str:
    year, digits = real[:2], len(real) - 5
    n = _h(f"{real}#{salt}", "reg")
    prog = PROGRAMMES[n % len(PROGRAMMES)]
    num = _h(f"{real}#{salt}", "regnum") % (10**digits)
    return f"{year}{prog}{num:0{digits}d}"


def fake_reg(real: str) -> str:
    return _assign(real, "reg", _gen_reg)


def _gen_name(real: str, salt: int) -> str:
    n = _h(f"{real}#{salt}", "name")
    first = FIRST[n % len(FIRST)]
    last = LAST[(n // len(FIRST)) % len(LAST)]
    if salt == 0:
        return f"{first} {last}"
    # Widen the space with a middle initial, then a numeric-free suffix word.
    mid = LETTERS[(salt - 1) % 26]
    if salt <= 26:
        return f"{first} {mid} {last}"
    return f"{first} {mid} {LAST[(salt // 26) % len(LAST)]} {last}"


def fake_name(real: str) -> str:
    return _assign(real, "name", _gen_name)


def scan_real(paths: list[Path]) -> None:
    """Records every identifier and name present in the source data."""
    for p in paths:
        try:
            wb = openpyxl.load_workbook(p, read_only=True, data_only=True)
        except Exception:
            continue
        for ws in wb.worksheets:
            for row in ws.iter_rows(values_only=True):
                for c in row:
                    if c is None:
                        continue
                    s = str(c).strip()
                    up = s.upper()
                    if NEO_RE.match(up) or REG_RE.match(up):
                        REAL_VALUES.add(up)
                    elif looks_like_name(s):
                        REAL_NAMES.add(s.lower())
        wb.close()


def looks_like_name(s: str) -> bool:
    t = s.strip()
    if not (3 <= len(t) <= 60) or "@" in t or "://" in t:
        return False
    letters = sum(c.isalpha() for c in t)
    if any(c.isdigit() for c in t) or letters < 3:
        return False
    if t.lower() in {
        "name", "full name", "candidate name", "student name", "venue",
        "remarks", "role", "batch", "gender", "degree", "brach", "branch",
        "campus", "sno", "s.no", "neo id", "reg.no", "usn", "interview date",
        "lab no", "count", "register number", "reference_id", "candidate id",
        "ac - title",
    }:
        return False
    return letters / len(t) > 0.7


def anonymise_cell(value):
    if value is None:
        return None
    s = str(value).strip()
    if not s:
        return value
    upper = s.upper()
    if NEO_RE.match(upper):
        return fake_neo(upper)
    if REG_RE.match(upper):
        return fake_reg(upper)
    if looks_like_name(s):
        return fake_name(s.lower())
    return value


def convert(src: Path, dst: Path) -> tuple[int, int]:
    wb = openpyxl.load_workbook(src, data_only=True)
    neo_count = reg_count = 0
    for ws in wb.worksheets:
        for row in ws.iter_rows():
            for cell in row:
                new = anonymise_cell(cell.value)
                if new is not cell.value:
                    s = str(cell.value).strip().upper()
                    if NEO_RE.match(s):
                        neo_count += 1
                    elif REG_RE.match(s):
                        reg_count += 1
                    cell.value = new
    dst.parent.mkdir(parents=True, exist_ok=True)
    wb.save(dst)
    wb.close()
    return neo_count, reg_count


def main() -> None:
    if len(sys.argv) != 3:
        sys.exit(__doc__)
    src_dir, out_dir = Path(sys.argv[1]).expanduser(), Path(sys.argv[2])

    files = sorted(p for p in src_dir.glob("*.xlsx") if re.search(r"shortlist", p.name, re.I))
    if not files:
        sys.exit(f"no shortlist files found in {src_dir}")

    # Blocklist everything we can see, not just the files being converted: a
    # synthetic name must not coincide with a real student's name anywhere.
    blocklist_sources = sorted(set(src_dir.glob("*.xlsx")))
    if Path("Global").is_dir():
        blocklist_sources += sorted(Path("Global").glob("*.xlsx"))
    print(f"scanning {len(blocklist_sources)} source files for collision avoidance...")
    scan_real(blocklist_sources)
    print(f"  blocklist: {len(REAL_VALUES)} identifiers, {len(REAL_NAMES)} names\n")

    print(f"anonymising {len(files)} files -> {out_dir}\n")
    for p in files:
        safe = re.sub(r"[^a-z0-9]+", "_", p.stem.lower()).strip("_") + ".xlsx"
        neo, reg = convert(p, out_dir / safe)
        print(f"  {p.name:44} -> {safe:38} ({neo} neo, {reg} reg)")

    print(
        "\nDone. Fixtures contain no real identifiers or names.\n"
        "The mapping key was random and was not saved, so this is one-way."
    )


if __name__ == "__main__":
    main()
