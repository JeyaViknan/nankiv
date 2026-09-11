# nankiv

A desktop app that answers **"am I shortlisted?"** in under two seconds, then
tells you what the shortlist actually reveals about how the company chose.

Placement season means a shortlist a day, sometimes more. Each one arrives as a
spreadsheet keyed by an eight-character alphanumeric Neo ID that nobody can
remember — least of all their friends'. The current routine is: download the
attachment, wait for Excel, Ctrl+F your own ID, then message four people to ask
whether they made it, then collectively guess at the cutoff from whoever replies.

nankiv collapses that into: drop the file, read one line.

**Everything runs on your machine.** No account, no server, no telemetry, no
network calls. It works with the wifi off.

---

## What it does

**Always works** — exact, never inferred:

- Your verdict, the moment the file lands
- Everyone in your circle checked in the same pass
- Round-to-round diffs: who advanced, who dropped, who is new
- History of every drive you've imported, with duplicate detection

**Works where identity is known:**

- Names next to Neo IDs, so a shortlist reads as people rather than codes
- Search a classmate by name to find their Neo ID
- Reference-sheet import that permanently improves every past and future drive

**Statistical, and always labelled as such:**

- CGPA distribution against the batch baseline
- Cutoff estimation with the reasoning shown
- Branch over-representation, which is often the real filter
- Where you stand within the shortlist

---

## Two rules the code enforces

These are not guidelines. They are enforced by the type system and covered by
tests that fail loudly.

**1. "Unknown" is never rendered as "not shortlisted."**

Some shortlists are keyed by registration number rather than Neo ID. One in the
sample corpus uses a TCS-internal reference ID that matches no student identifier
at all. In those cases your absence from the file says *nothing* about you, and
nankiv says exactly that. `Verdict` is a three-way enum with no `Default` and no
boolean coercion, so collapsing the cases requires writing a deliberate match arm.

**2. An estimated cutoff is never presented as an official one.**

Every statistic is wrapped in an `Estimate` that cannot be constructed without
its sample size, and nothing statistical is published below 20 matched students
and 15% coverage. During development a single matched student produced a
confident "cutoff ≈ 9.0" — that case is now a regression test.

A related rule governs matching: **a wrong student match is far worse than an
unresolved one.** If the best name candidate doesn't clearly beat the runner-up,
the match is refused. `Shresth Kumar Gupta` and `Niraj Kumar Gupta` score
identically to `Sujal Chhajed` and `Sujal Sanjay Chhajed` — one pair is two
people, the other is one person, and no score separates them.

---

## Install

Download the installer for your platform from
[Releases](../../releases), open it, and drag nankiv to Applications.

Builds are currently **unsigned**, so the first launch shows a security warning:

- **macOS** — right-click the app and choose *Open*, then *Open* again. Once only.
- **Windows** — click *More info*, then *Run anyway*. Once only.

See [docs/RELEASING.md](docs/RELEASING.md) for how to enable signing.

---

## Using it

1. **Tell it who you are** — your Neo ID, and your registration number if you
   have it. Some companies key shortlists by the latter, and without it those
   files can't be answered.
2. **Add reference data** (optional) — drop in the roster or CGPA sheets you
   already received. nankiv ships with no student data in it.
3. **Add your circle** (optional) — a name and a Neo ID each, once.
4. **Drop shortlists.** That's the whole daily loop.

Every file that carries two identifier types at once permanently improves the
identity graph, so nankiv gets better at naming people the more you use it.

---

## Development

Requires [Rust](https://rustup.rs) and Node 20+.

```bash
npm install
npm run app:dev
```

| Command | What it does |
| --- | --- |
| `npm run app:dev` | Run the app with hot reload |
| `npm run app:build` | Build installers for the current platform |
| `npm run check:all` | Typecheck, lint and every test, both sides |
| `npm run test` | Frontend component tests |
| `npm run rust:test` | Rust unit, regression, integration and property tests |

### Layout

```
src/                  React interface — renders, holds no business logic
  components/         Verdict and analytics rendering
  screens/            One module per screen
  lib/api.ts          Typed bridge to the core
src-tauri/src/
  model.rs            Domain types; the reliability hierarchy lives here
  parse/              Spreadsheet reading, format-driven column election
  identity/           Normalisation, matching, the identifier graph
  analytics/          Baseline comparison, cutoff, branch lift
  store/              SQLite schema, migrations, repository
  engine.rs           Wires the above together
  commands.rs         The IPC surface
```

The interface holds no business logic. It renders what the core returns and
sends back user intent, which is what keeps personal data behind a single
boundary — and what would make a CLI a matter of adding a frontend.

### Testing

```bash
npm run check:all
```

- **Parser regression** against anonymised copies of all fifteen real-world
  shortlist formats, including the empty-sheet, foreign-ID and duplicate-file
  cases. Any change to a parse count fails.
- **Integration** covering import → harvest → verdict → analytics, with the
  negative paths (unreadable file, wrong key type) asserted explicitly.
- **Property tests** generating name collisions to prove the ambiguity gate
  holds for cases nobody thought to write down.
- **Component tests** proving the four verdict states never render alike.

### Fixtures

The fixtures in `src-tauri/tests/fixtures/` are structure-preserving
anonymisations of real shortlists: layout, headers, sheet quirks and cross-file
relationships are intact; every identifier and name is synthetic. Regenerate
with:

```bash
python3 scripts/make_fixtures.py ~/Downloads src-tauri/tests/fixtures
python3 scripts/verify_fixtures.py ~/Downloads Global
```

The second command is a privacy guard and must pass before committing. Real
spreadsheets are gitignored and CI fails if one is ever committed.

---

## Privacy

nankiv is built so that the careless path is also the safe one. See
[docs/PRIVACY.md](docs/PRIVACY.md) for the full reasoning.

- **Ships empty.** No student data is distributed with the application.
- **Minimises at ingestion.** Only registration number, name, CGPA and branch
  are read from a reference sheet. Phone numbers, email addresses, dates of
  birth and resume links are never extracted — and the database has no column
  for them, so they could not be stored even by mistake.
- **No bulk export.** You can export your own history and nothing else. This is
  the single most effective control available, because it removes the payoff
  from the obvious misuse.
- **Friends' CGPA is off by default.** The cutoff analysis needs aggregates, not
  per-person disclosure.
- **One-click wipe**, with a visible inventory of exactly what is stored.

If you are deploying this for your own batch, tell your placement cell first.
You are handling their data and your peers'.

---

## Licence

MIT
