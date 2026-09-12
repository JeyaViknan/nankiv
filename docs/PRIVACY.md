# Privacy

nankiv handles students' academic records. This document explains what it does
with them and why, so the decisions can be challenged rather than trusted.

## The shape of the problem

The reference sheets circulating during placement season carry far more than the
tool needs: personal email addresses, phone numbers, gender, date of birth,
resume links and 10th/12th marks alongside the CGPA. A tool that reads those
sheets is one careless step away from becoming a distributable directory of a
few thousand classmates' personal data.

Three decisions keep that from happening.

## 1. The application ships with the cohort's academic data

This reverses an earlier decision, and the reasoning is worth recording.

The original design shipped empty: each student would import the reference
sheets they had already received, so whoever published the app redistributed
nothing. In practice that meant every student was asked to go and find a
spreadsheet before the app could tell them anything — two thousand separate
efforts to avoid one act of publication, imposed on people with no way to
anticipate it and no reason to accept it. The maintainer already holds the sheet.

So the pack ships. What ships is the *resolved* result rather than the source:
registration number, name, CGPA, branch, and the Neo ID links that name matching
could establish safely. Nothing else survives the generator.

The trade is real and should be stated plainly rather than buried. Publishing
the application now distributes roughly 2,500 classmates' names and CGPAs. That
is a materially different act from a file circulating inside the cohort, and it
carries exposure under India's DPDP Act 2023. Anyone deploying this should have
their placement cell's agreement first, and should decide deliberately whether
the repository holding `reference/pack.json` ought to be public.

## 2. Minimisation is structural, not procedural

The academic parser reads exactly four fields: registration number, name, CGPA,
branch. There is no code path that extracts anything else.

Behind it, the `academics` table has columns for `reg_no`, `cgpa`, `branch`,
`cohort` and `source` — and nothing else. A phone number has nowhere to be
written even if a future change tried. A test asserts this:

```
importing_never_stores_contact_details  (src-tauri/tests/import_flow.rs)
```

This is deliberate. Minimisation enforced by a schema survives refactors;
minimisation enforced by developer discipline does not.

## 3. The interface removes the payoff from misuse

- **No bulk export.** A student can export their own history. There is no
  "export all students" anywhere, which is the single most effective control
  available: it removes the reason to extract the data in the first place.
- **Search returns a person, not a table.** One deliberate query at a time. No
  browsable roster view exists.
- **Individual CGPA is opt-in and off by default.** Aggregate analysis needs no
  per-person disclosure; only the student themselves and explicitly added
  friends can show a CGPA, and only with the setting enabled.
- **Share summaries carry aggregates only** — never a list of named individuals,
  and never the student's own status, which is theirs to disclose.
- **A visible data inventory** in Settings, with a one-click wipe.

## What leaves the machine

Nothing. There is no network client in the binary. The only capabilities the
application declares are a window, a file-open dialog, and opening a URL in the
system browser:

```
core:default
core:window:allow-start-dragging
dialog:allow-open
opener:allow-open-url
```

No HTTP capability. No shell capability. No filesystem-write capability. A
spreadsheet is read through a file path the user chose, parsed as data — no
macros, no formula evaluation, no external references — and the results are
written to a local SQLite database.

## Where the data lives

| Platform | Path |
| --- | --- |
| macOS | `~/Library/Application Support/app.nankiv.desktop/nankiv.db` |
| Windows | `%APPDATA%\app.nankiv.desktop\nankiv.db` |
| Linux | `~/.local/share/app.nankiv.desktop/nankiv.db` |

Delete that file, or use **Settings → Delete all local data**, and nothing
remains.

## What this does not protect against

Stated plainly, because a security claim that overreaches is worse than none:

- **The database is not encrypted at rest.** Anyone with access to the user's
  account can read it. Encrypting it with a key shipped in the binary would be
  obfuscation, not security — the key would be extractable — which is precisely
  why the real answer is minimisation: do not store what is not needed.
- **A student can still read what is on their own screen.** The controls above
  raise the effort of bulk misuse; they do not make individual lookups
  impossible, and they are not meant to.
- **Reference sheets a student imports are their own responsibility.** nankiv
  reduces what it keeps, but it cannot control what the student was sent.

## For maintainers

The bundled pack at `src-tauri/reference/pack.json` is generated from `Global/`
by `cargo run --bin build_reference`, and is the one committed artefact that
carries real names and CGPAs. Regenerate it whenever the source sheets change.
A test asserts it contains no email address, phone number or document link.

Real spreadsheets must never be committed. `.gitignore` excludes `*.xlsx` and
the `Global/` directory, and CI fails if a spreadsheet appears outside the
fixtures directory.

Test fixtures are structure-preserving anonymisations: layout, headers, sheet
quirks and cross-file relationships intact, every identifier and name synthetic
and collision-checked against the real data. Before committing regenerated
fixtures:

```bash
python3 scripts/verify_fixtures.py ~/Downloads Global
```

It fails if any real identifier or name appears in a fixture.
