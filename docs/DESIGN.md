# Interaction design review

Three maps: the product as built, the product as it should be, and the
transformation between them. Written before any code changed.

---

## Map 1 — the current product

### Entry and onboarding

A two-step wizard. Step one asks for a Neo ID and, optionally, a registration
number. Step two offers reference-data import.

- The wizard is a **wall before the door**. It demands identity before the
  student has seen the app do anything. Nothing can be tried first.
- The registration number is labelled *optional* but is load-bearing: two of the
  fifteen real shortlist formats are keyed by it, and without it those files can
  never be answered. The label understates it.
- Step two asks for a "reference sheet" with no context for what that is or why
  it helps. It is abstract at exactly the moment the student has least patience.

### Home

Heading, drop zone, a conditional notice, then "Recent drives".

- **The affordance lies.** The drag listener is registered on the whole webview,
  so a file dropped anywhere in the window works — but only a small dashed
  rectangle reacts. The app accepts more than it admits to.
- "Recent drives" sits at the same visual weight as the primary action.
- Nothing tells the student what will happen after the drop.

### Import

Drop → one IPC call → the view switches to the result.

- **It is a black box with a jump at the end.** No filename echo, no statement of
  what was detected, no sense of progress. The spinner is inside the drop zone,
  which is the thing the user just stopped looking at.
- The company name is inferred from the filename and **saved silently**, with no
  chance to see or correct it. There is no rename anywhere in the app, so a bad
  guess is permanent.
- A duplicate import renders as a **red error banner**. Re-dropping yesterday's
  file is ordinary behaviour, not a failure.
- An unreadable file renders in the same red banner style, which reads as
  something having gone wrong rather than as an explanation.

### Drive result

Verdict banner → "learned N links" notice → circle → a lone copy button → three
tabs → analytics.

- Verdict first, circle second is **correct and stays**.
- The "learned N identity links" notice occupies the second-most prominent slot
  on the page and is the least important thing on it.
- "Copy summary for the group chat" floats between two cards with no visual
  home.
- The tabs — Overview / CGPA / Branches — are an arbitrary split. Overview is
  mostly the cutoff card, which is a CGPA finding. Tabs hide content that would
  read perfectly well as one scroll.
- No title, no back affordance. Which drive you are looking at is only legible
  from inside the verdict sentence.

### Circle

An add form requiring a Neo ID, then a list with Remove buttons.

- **The product's own premise is unsolved in its own UI.** Adding a friend
  requires already knowing their eight-character alphanumeric ID — the exact
  thing nankiv exists because nobody can remember. The app has a working name
  search and does not use it here.
- Remove is immediate, with no undo.

### Search

An input, a button, and two differently shaped result blocks.

- Requires pressing a button for something that should be live.
- Finding a person by name is a **dead end** — the one thing you would want to do
  next, add them to your circle, is not offered.

### History

A list where every row carries three controls.

- **The compare interaction is the worst thing in the app.** A tri-state button
  whose label cycles "Compare" → "Selected" → "vs this", changing across every
  row simultaneously. The user has to decode a stateful control replicated N
  times to perform one action.
- Delete is immediate and destructive, with no confirmation and no undo.
- Three controls per row means control density scales with history length.

### Settings

Appearance, identity, privacy, reference data, inventory, wipe. Broadly right.

### Global

Only one keyboard shortcut exists (Cmd+,). No Cmd+O, no Cmd+F, no Escape, no
arrow navigation. Errors push content down rather than attaching to what failed.

---

## Map 2 — the ideal

### The mental model

The student's model is not *Home / Circle / Search / History*. That is the
database schema wearing a navigation bar. Their model is:

> "A file arrives. I need to know. Sometimes I look back."

One recurring act, and a small number of occasional side tasks. The navigation
should reflect that ratio.

### Structure

**One surface, one detail view, two sheets.**

- **Shortlists** — the drop surface *is* the timeline. Importing prepends to it.
  Home and History were never two things; they were one thing split in half.
- **Drive** — pushed when a drive is opened, with a real back affordance.
- **Circle** and **Settings** — sheets. Both are configured occasionally and
  left alone; neither deserves a permanent navigation slot.
- **Search** — a field that is always visible in the toolbar. Not a destination,
  not hidden behind a shortcut. Discoverable *and* fast.

Five destinations become one, and the sidebar disappears with them.

### The drag interaction

This is the defining interaction and deserves to be designed as a sequence
rather than a widget.

| Phase | What must be true |
| --- | --- |
| Before | The window looks receptive. The student knows a file can come here without being told twice. |
| Enter | **The whole window responds**, because the whole window is the target. Content recedes; a single clear invitation appears. |
| Over | Steady, calm, no flicker. |
| Invalid | Rejected **during** the drag, not after the drop — with the reason. |
| Drop | Instant acknowledgement by name. The student sees the file was received before anything is computed. |
| Reading | What was detected, as soon as it is known. |
| Result | The answer, in the same continuous surface. |
| Failure | An explanation, not an alarm. Explicitly not a verdict. |

### Confirmation, and its absence

Confirmation is friction with a good reputation. The right defaults:

- **Import: never confirm.** Do it, show the answer, and make the inferred
  company name inline-editable in the result header. Transparency and
  correctability replace a modal.
- **Duplicate: never an error.** Open the drive that already exists and say so
  quietly. Re-dropping a file is a normal thing to do.
- **Delete: never confirm, always undo.** A toast with Undo beats a dialog,
  because the common case costs nothing and the rare case is still recoverable.
- **Wipe everything: confirm.** This one is genuinely irreversible.

### Trust

Certainty levels must be legible at a glance and never overstated:

- *Shortlisted* — full colour, unambiguous.
- *Not shortlisted* — plain, neutral, not alarming. A rejection is not an error.
- *Cannot determine* — visibly a third thing, with the reason and the fix.
- *Estimated* — always labelled, always carrying its sample.

Visual polish must never imply more confidence than the data supports.

---

## Map 3 — transformation

| Area | Current | Ideal | Change | Priority |
| --- | --- | --- | --- | --- |
| Navigation | Sidebar, 5 destinations | Toolbar, 1 surface + detail + 2 sheets | Redesign | High |
| Home / History | Two screens | One timeline that is also the drop target | Merge | High |
| Drag and drop | Small rectangle, whole-window listener | Whole-window target, enter/over/invalid/drop states | Redesign | High |
| Import feedback | Black box then jump | Named acknowledgement → detection → answer | Introduce | High |
| Duplicate import | Red error | Opens the existing drive with a quiet note | Redesign | High |
| Unreadable file | Red error | Calm explanation, explicitly not a verdict | Refine | High |
| Company name | Silent guess, permanent | Inline-editable in the result header | Introduce | High |
| History compare | Tri-state button per row | Contextual "compare with…" from the drive | Redesign | High |
| Add a friend | Requires knowing the Neo ID | Search by name, add from the result | Redesign | High |
| Search | A page with a button | Always-visible toolbar field, live | Redesign | Medium |
| Delete | Instant, no recourse | Instant, with Undo | Refine | Medium |
| Analytics tabs | Arbitrary three-way split | One sequence, progressively disclosed | Refine | Medium |
| Keyboard | Cmd+, only | Cmd+O, Cmd+F, Escape, arrows, Enter | Introduce | Medium |
| Verdict hierarchy | Verdict → links notice → circle | Verdict → circle → everything else | Refine | Medium |
| Onboarding | Wall before the door | Single step, honest about the reg number | Refine | Medium |
| Settings | Sheet at Cmd+, | Unchanged | Keep | — |
| Verdict components | Four distinct states, tested | Unchanged in logic, refined in form | Keep | — |
| Analytics engine | Coverage-gated, honest | Unchanged | Keep | — |
| Rust core | Parsing, matching, storage | Unchanged | Keep | — |

### Architectural changes, and why

Three additions to the command surface. Nothing is rewritten.

1. **`rename_drive`** — the UI cannot offer a correctable company name without
   it. The alternative is a permanent wrong guess.
2. **`delete_drive` returns a snapshot** and **`restore_drive` accepts one** —
   required to offer Undo instead of a confirmation dialog.
3. **`set_drive_round`** — links two drives as rounds of one process so the
   comparison can be offered contextually rather than through manual selection.

The parsing, matching, analytics, storage and privacy models are untouched.
