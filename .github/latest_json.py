"""Writes latest.json, which nankiv's Check for Updates reads.

    python3 .github/latest_json.py <signatures-dir> <release-notes.md>

Run by the release workflow once the installers are attached. Each platform
gets the update bundle to install and the signature it must carry; the notes
are the release notes' "What's new" section, which the app shows before
installing.
"""

import datetime
import json
import os
import re
import sys

PLATFORMS = {
    "darwin-aarch64": "nankiv-macos-apple-silicon.app.tar.gz",
    "darwin-x86_64": "nankiv-macos-intel.app.tar.gz",
    "windows-x86_64": "nankiv-windows-setup.exe",
}


def whats_new(markdown: str) -> str:
    match = re.search(r"^## What's new[^\n]*\n(.*?)(?=^## |\Z)", markdown, re.S | re.M)
    text = match.group(1) if match else ""
    # Plain text: the app shows it as written, not as Markdown.
    text = re.sub(r"\*\*(.+?)\*\*", r"\1", text)
    text = re.sub(r"\n(?!- |\n)\s*", " ", text)
    return text.strip()


def main() -> None:
    signatures, notes_path = sys.argv[1], sys.argv[2]
    tag = os.environ["TAG"]
    base = f"https://github.com/{os.environ['GH_REPO']}/releases/download/{tag}/"

    platforms = {}
    for key, asset in PLATFORMS.items():
        sig = os.path.join(signatures, asset + ".sig")
        if os.path.exists(sig):
            with open(sig) as f:
                platforms[key] = {"signature": f.read().strip(), "url": base + asset}
    if not platforms:
        sys.exit("no signatures found")

    with open(notes_path) as f:
        notes = whats_new(f.read())

    json.dump(
        {
            "version": tag.lstrip("v"),
            "notes": notes,
            "pub_date": datetime.datetime.now(datetime.timezone.utc)
            .replace(microsecond=0)
            .isoformat()
            .replace("+00:00", "Z"),
            "platforms": platforms,
        },
        sys.stdout,
        indent=2,
    )
    print()


if __name__ == "__main__":
    main()
