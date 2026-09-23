# Releasing

CI builds installers for macOS (Apple Silicon and Intel), Windows x64 and Linux
x64 on every push. Linux is built to keep the codebase portable; it is not a
release target yet.

## Cutting a release

1. Bump the version in `package.json`, `src-tauri/Cargo.toml` and
   `src-tauri/tauri.conf.json` — all three must agree.
2. Tag and push:

```bash
git tag v0.1.0 && git push origin master --tags
```

3. `.github/workflows/release.yml` builds macOS (Apple Silicon and Intel),
   Windows and Linux, and attaches the installers to a **draft** release.
4. Check the downloads, write the notes, and publish it from the Releases page.
   Until it is published, the links are not public.

The `ci.yml` workflow also builds installers on every push, but those are
workflow artifacts: they need a GitHub login and expire after 14 days. A
release is what produces a link anyone can use.

## Code signing

Builds are unsigned by default, and unsigned builds cost real adoption: macOS
reports that the developer cannot be verified, and Windows SmartScreen warns of
an unrecognised app. Neither is a bug that can be coded around.

Until signing is configured, the download page must carry the workaround:
right-click → *Open* on macOS, *More info* → *Run anyway* on Windows.

### macOS — Apple Developer Program, $99/year

The highest-leverage spend in this project. Removes the warning completely.
Set these repository secrets:

| Secret | What it is |
| --- | --- |
| `APPLE_CERTIFICATE` | Base64 of the exported Developer ID `.p12` |
| `APPLE_CERTIFICATE_PASSWORD` | Password for that `.p12` |
| `APPLE_SIGNING_IDENTITY` | e.g. `Developer ID Application: Name (TEAMID)` |
| `APPLE_ID` | Apple ID email |
| `APPLE_PASSWORD` | An app-specific password, not the account password |
| `APPLE_TEAM_ID` | Ten-character team identifier |

```bash
base64 -i certificate.p12 | pbcopy
```

Notarisation must run in CI or every release becomes a manual step.

**The widget signs with the same identity.** `widgets/macos/build.sh` signs the
extension ad hoc (`CODE_SIGN_IDENTITY=-`) and Tauri then signs the app bundle
with `bundle.macOS.signingIdentity`, currently `"-"` as well. With a Developer
ID, set both:

```bash
CODE_SIGN_IDENTITY="Developer ID Application: Name (TEAMID)" widgets/macos/build.sh
```

and change `signingIdentity` in `src-tauri/tauri.conf.json` to the same string.
Two things to check afterwards, because both have failed elsewhere and neither
is loud about it:

- `codesign -d --entitlements - nankiv.app/Contents/PlugIns/NankivWidget.appex`
  still lists `app-sandbox` and the read-only exception. Re-signing a nested
  bundle without its entitlements produces a widget that installs and never
  loads.
- `codesign --verify --deep --strict nankiv.app` passes, and the widget appears
  in `pluginkit -m -v -p com.apple.widgetkit-extension`.

Notarisation needs the hardened runtime on the extension, which
`widgets/macos/project.yml` already sets.

### Windows — the widget needs a package, not just a signature

Windows widgets can only be provided by an MSIX-packaged app, so shipping the
Windows widget means adding an MSIX artifact alongside the MSI and NSIS
installers, signed with a certificate Windows accepts. The provider and manifest
are written and waiting in `widgets/windows`; see that directory's README for
the remaining steps and what has not been verified.

### Windows — lower priority

An OV certificate is roughly $200–400/year and SmartScreen reputation still
builds over time, so the return is weaker. Ship unsigned with clear instructions
and revisit if usage justifies it.

## Update signing

Separate from OS code signing, and free. It proves an update came from you.

```bash
npx tauri signer generate -w ~/.tauri/nankiv.key
```

Set `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` as
repository secrets. Never commit the private key — `.gitignore` already excludes
`tauri-signing-key*`.

## Release checklist

- [ ] `npm run check:all` passes
- [ ] `python3 scripts/verify_fixtures.py ~/Downloads Global` passes
- [ ] Versions agree across the three files
- [ ] Installed from the built artifact on a clean machine, both platforms —
      including the Gatekeeper and SmartScreen path, which is the most commonly
      skipped and most adoption-critical test here
- [ ] Dropped a real shortlist and got the right verdict
- [ ] Added the widget from the desktop gallery at each size, and a click opened
      the right shortlist — the one part of the widget no test can check
