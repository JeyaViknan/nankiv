# nankiv on the Windows widgets board

The same glance as on macOS, built on the mechanism Windows actually provides:
a **widget provider** for the Windows 11 widgets board, answering with
**Adaptive Cards**.

## Status, plainly

| Part | State |
| --- | --- |
| Adaptive Card templates (small, medium, large) | **Written and checked.** `widgets/windows/cards.test.ts` expands every template for every state using Microsoft's own `adaptivecards-templating`, and runs in CI on every platform. |
| Card content for each state (`CardData`, `Glance`, `Copy`) | **Written in C#, not yet compiled.** The expected output for every state is committed in `cards/examples/`, and `Nankiv.Widgets.Tests` holds the C# to it. |
| Provider (`IWidgetProvider`, COM server, snapshot watching) | **Written, not yet compiled or run.** Needs Windows. |
| MSIX packaging | **Not done.** This is the blocker — see below. |

Everything here was written on a Mac, where neither the Windows App SDK nor a
.NET SDK can build or run it. Nothing in this directory has been executed on
Windows. It is not shipped in any release, and `npm run app:build` does not
build it.

## Why MSIX is the blocker

Only a **packaged** app can provide a Windows widget: Microsoft's documentation
is explicit that a widget provider must be a packaged Win32 app (MSIX) or a
PWA, and the COM class the widgets board activates is declared in
`Package.appxmanifest`. There is no registry-only or unpackaged path.

nankiv currently ships an MSI and an NSIS installer from Tauri. Shipping the
widget therefore means adding an MSIX package as a third Windows artifact:

1. Build the provider (`provider/Nankiv.Widgets.csproj`, `win-x64` and `win-arm64`).
2. Produce an MSIX containing `nankiv.exe`, the provider executable, the card
   templates, icons and screenshots, using `provider/Package.appxmanifest`.
3. Sign it. Windows will not install an unsigned MSIX, so this needs a
   certificate — the same decision as macOS notarisation, and the same reason
   it is not done here: no signing credentials have been provided.
4. Verify on Windows 11: pin each size, confirm the card matches the examples,
   and confirm a click opens nankiv on the right shortlist.

Until then, Windows users get the app without a widget, and nothing pretends
otherwise.

## How it works

```
app writes snapshot.json ──► FileSystemWatcher ──► Glance.Make ──► CardData.Build
                                                                        │
                       Adaptive Card template (small|medium|large) ◄─────┘
                                      │
                       WidgetManager.UpdateWidget(template, data)
```

- **No polling.** The provider watches the snapshot file — the app renames a new
  one into place — and updates only widgets the board says are active.
- **No logic of its own.** Verdicts, counts and cutoffs come from the snapshot,
  which the Rust core computed. `Glance.cs` and `Copy.cs` mirror the Swift files
  of the same name, decision for decision and word for word.
- **Nothing to invoke.** The cards use `Action.OpenUrl` only, never
  `Action.Execute`, so a card can open the app and nothing else. The
  `windows.protocol` extension registers `nankiv://`, the same scheme macOS
  registers, and the core already accepts it on the command line — which is how
  Windows hands a URL to an app.
- **Configuration.** macOS lets you pin the widget to one shortlist. On Windows
  the provider starts zero-configuration (always the latest); pinning would use
  the board's customization flow and `CustomState`, which is the natural place
  for it when the package exists.

## Adapted to Fluent, not ported from Apple

| macOS | Windows | Why |
| --- | --- | --- |
| Four sizes (small → extra large) | Three (small, medium, large) | The board offers three. The extra-large layout's content maps onto large. |
| SF Symbols for each verdict | The status word plus Adaptive Cards' `Good` / `Default` / `Warning` colour | The widget host has no icon element that works offline; words carry the meaning, colour only reinforces it. |
| Drawn CGPA histogram with the cutoff marked | "8.5 – 10.0 · median 8.95" | Adaptive Cards has no chart element in the widget host. A sentence is honest; a fake chart is not. |
| System font, Apple's type scale | Segoe UI via the Fluent type ramp (`Small/Lighter` … `ExtraLarge/Bolder`) | Microsoft's widget design guidance specifies exactly these Adaptive Cards formulas. |
| 16pt margins, `containerBackground` | The host supplies margins, background and the 48px attribution header | Cards must not draw their own header — the board owns it. |

## Working on it

```bash
npm test -- widgets            # card templates against every state, anywhere
dotnet test widgets/windows/provider/tests   # the provider's own checks, on Windows
```

After changing the snapshot contract, regenerate the shared fixtures, then
update `cards/examples/` to match and run both.
