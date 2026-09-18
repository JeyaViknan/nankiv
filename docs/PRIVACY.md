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

- **Names can be exported; records cannot.** This was originally "no bulk
  export at all". It now allows exactly one thing: downloading the list of who
  is on a shortlist, as Excel or CSV. The reasoning is that the shortlist itself
  already reaches every student in the batch, and turning its column of codes
  into names is the job people were doing by hand in the group chat. What the
  file may contain is deliberately narrow:
  - the identifier the shortlist already carried, and a name;
  - a name **only** where the identity link is strong enough to show a person —
    a probable match would attach the wrong student to a list that is about to
    be forwarded;
  - a row for **every** student, so an unidentified student stays visible
    rather than silently disappearing from what looks like a complete list;
  - **no** CGPA, branch, or registration number the file did not already have.

  Two injection hazards are handled in the writer: names are quoted so a comma
  cannot split a column, and a cell starting with `=`, `+`, `-` or `@` is
  prefixed so a spreadsheet opens it as text rather than running it as a
  formula. The academic data behind the analysis has no export path at all.
  Tests in `src-tauri/src/export.rs` assert all of the above.
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
dialog:allow-save
opener:allow-open-url
```

No HTTP capability. No shell capability. No filesystem capability of any kind
in the webview. A spreadsheet is read through a path the user chose in the
native open panel, parsed as data — no macros, no formula evaluation, no
external references — and the results are written to a local SQLite database.
The one file nankiv writes elsewhere is a names export, and only to the path the
user chose in the native save panel; the write is done by the Rust core, not the
webview.

## Where the data lives

| Platform | Path |
| --- | --- |
| macOS | `~/Library/Application Support/app.nankiv.desktop/nankiv.db` |
| Windows | `%APPDATA%\app.nankiv.desktop\nankiv.db` |
| Linux | `~/.local/share/app.nankiv.desktop/nankiv.db` |

Beside it, `widget/snapshot.json` — the only thing the desktop widget can see.
Delete both, or use **Settings → Delete all local data**, and nothing remains.

## The desktop widget

A widget runs in its own process, outside the app. That process is given the
least it can be given and still be useful.

**It cannot read the database.** The macOS extension is sandboxed with exactly
one exception — read-only access to the directory holding `snapshot.json`:

```
com.apple.security.app-sandbox
com.apple.security.temporary-exception.files.home-relative-path.read-only
  = /Library/Application Support/app.nankiv.desktop/widget/
```

Tested, not assumed: an earlier build of this extension could read
`snapshot.json` and was denied `nankiv.db` in the same directory tree, and
denied the home directory. The build in CI fails if that entitlement is missing,
if `get-task-allow` appears, or if broader file access is granted.

**The snapshot is display-ready and thin.** It holds what a glance needs and
nothing else: company names, counts, verdict words, coverage, the estimated
cutoff, bucket counts for the chart, and the labels the student typed into their
own circle. It holds **no Neo IDs, no registration numbers, no individual
CGPAs**, and no names from the reference data. The Rust tests assert this on
synthetic data, and the same check runs against every committed fixture.

**It carries no error text.** A failed import is recorded as a filename and a
time, not a message, because error messages can quote file paths.

**Links are navigation only.** A widget click opens `nankiv://drive/<id>`,
`nankiv://latest` or `nankiv://shortlists`. Any application or web page can open
a URL scheme, so no route imports, deletes, exports or changes a setting, and
anything that does not parse exactly is ignored. The Windows cards use
`Action.OpenUrl` and never `Action.Execute`, so a card cannot ask the provider
to do anything either.

**On screen means on screen.** The answer and your circle are marked
privacy-sensitive, so the system hides them where it hides private data. A
widget sits on a desktop that other people can see, which is the reason the
snapshot has no identifiers in it to show.

## What this does not protect against

Stated plainly, because a security claim that overreaches is worse than none:

- **The database is not encrypted at rest.** Anyone with access to the user's
  account can read it. Encrypting it with a key shipped in the binary would be
  obfuscation, not security — the key would be extractable — which is precisely
  why the real answer is minimisation: do not store what is not needed.
- **A widget is visible to whoever can see the screen.** Sharing a screen or
  sitting in a lab shows the latest result and your circle's labels to anyone
  looking. Remove the widget if that matters; the data it can show is already
  the minimum.
- **A student can still read what is on their own screen.** The controls above
  raise the effort of bulk misuse; they do not make individual lookups
  impossible, and they are not meant to.
- **Reference sheets a student imports are their own responsibility.** nankiv
  reduces what it keeps, but it cannot control what the student was sent.

## For maintainers

The bundled pack at `src-tauri/reference/pack.json` is generated from `Global/`
by `cargo run --example build_reference`, and is the one committed artefact that
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
