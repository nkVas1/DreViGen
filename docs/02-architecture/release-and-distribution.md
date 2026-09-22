# Release and Distribution

*Written 2026-09-22, when the release pipeline was built. What is true here is what the
workflow does; what is missing is named as missing.*

Six artefacts come out of one source tree. This note says how, what each platform refuses to do
until someone pays for a certificate, and what we do about that in the meantime.

## 1. What is built

`.github/workflows/release.yml` runs on a `v*` tag and produces a **draft** GitHub release, so
the notes can be written and the downloads opened before anyone is told the release exists.

| Target | Runner | Artefacts | Signed |
|---|---|---|---|
| Windows x64 | `windows-latest` | `.msi`, `.exe` (NSIS) | **No** |
| macOS universal | `macos-15` | `.dmg`, `.app` | **No** |
| Linux x64 | `ubuntu-24.04` | `.deb`, `.rpm`, `.AppImage` | n/a |
| Android | `ubuntu-24.04` | `.apk`, `.aab` | **No** |
| iOS | — | — | Not built |
| Web | — | Served from the VDS | n/a |

The Android row is the one that has been run end to end outside CI: 2026-09-22, NDK
30.0.16248370, a 7.6 MB APK carrying `libdrevigen_shell_lib.so` for arm64-v8a. Tauri embeds the
front end in the Rust library rather than in the APK's asset directory, which is why there is
no `assets/` tree to inspect.

The NSIS installer offers Russian, English and German with a language selector, which are the
launch locales. The MSI does not: Windows Installer picks one language per package, and
building three packages to gain a translated progress bar is not worth the confusion of three
downloads.

## 2. What is not signed, and what that costs the user

This is the part that is usually left out of a release document, so it is the part written
first.

**Windows.** An unsigned installer triggers SmartScreen: *"Windows protected your PC"*, with the
Run button behind *More info*. For the persona this project is built for — see
[00-vision.md](../00-vision.md) — that dialogue is where the installation ends. An OV
code-signing certificate is roughly €200–400 a year, an EV one more, and reputation accrues to
the certificate over time rather than immediately. Azure Trusted Signing is cheaper but requires
a verified organisation.

**macOS.** Without a Developer ID certificate and notarisation, Gatekeeper refuses to open the
application at all on first launch; the user has to right-click → Open, or clear the quarantine
attribute from a terminal. Apple charges $99 a year for the Developer Program, which also covers
iOS.

**Android.** `tauri android build` produces an unsigned release APK and AAB. The APK can be
sideloaded after the user permits installation from unknown sources; the AAB cannot be published
to Google Play without a signing key. A Play Console account is a one-off $25.

**Linux.** Nothing is signed there either, and nothing expects it to be. A `.deb` from a project's
own release page is ordinary.

### The interim position

The project's constraint is zero mandatory recurring cost, so no certificate is bought until
there is a reason. Until then:

- The release notes say plainly that the artefacts are unsigned and how to get past each
  platform's warning. Telling someone to click through a security dialogue is only acceptable
  when the instruction comes from the same place as the download.
- Every artefact's SHA-256 is published with it, so at least the bytes can be checked.
- The **web build is the recommended path for a non-technical relative**. An installed PWA needs
  no certificate, no dialogue and no administrator, and it is the same application. The native
  builds are for people who want a real window and the file associations.

When the project has users beyond the family, Windows signing is the first certificate to buy,
because SmartScreen is the one warning that stops an ordinary person completely.

## 3. Updating

The Tauri updater is **off**. `createUpdaterArtifacts` is not set, no update manifest is
produced, and the application does not check for one.

This is deliberate rather than unfinished. An updater needs a signing key, a manifest endpoint
and a rollback story, and it belongs with the sync service rather than before it — the same
server, the same accounts, the same release channel. `TAURI_SIGNING_PRIVATE_KEY` is already
wired through the workflow so that switching it on is a one-line configuration change and not a
pipeline rewrite.

Mobile is different and will stay different: the stores update mobile applications, and an
in-app updater is not permitted. See the last row of
[mobile-capabilities.md](./mobile-capabilities.md).

## 4. Versions

The application version lives in `apps/shell/src-tauri/tauri.conf.json`. It is `0.0.1`, which
is the *lowest value Android accepts*: `versionCode` is derived from it, and Tauri refuses to
build an Android package at `0.0.0`. The Rust crates stay at the workspace's `0.0.0` because
they are not published.

Windows Installer requires `major.minor.patch` with each part inside its own narrow range, and
macOS requires the bundle version to increase monotonically, so the scheme has to be plain
semver — no build metadata, no pre-release suffixes in the bundled version. A pre-release is
marked on the GitHub release instead.

## 5. Identifier

`io.github.nkvas1.drevigen`.

It is ugly and it is honest: it is derived from a namespace the project verifiably controls
today. An Android `applicationId` cannot be changed after the first Play Store publication, so
**this has to be settled before the first store submission** — if the project acquires a domain,
the identifier should move to it while moving is still free.

## 6. What is missing

| Gap | Blocker | When |
|---|---|---|
| iOS build | An Apple Developer Program membership; also a macOS machine or a CI runner for the simulator tests | With the first paid certificate |
| Code signing, all platforms | Certificates | §2 |
| SHA-256 in the release notes | Nothing; a workflow step | Before the first tagged release |
| Reproducible builds | Rust and Tauri both embed paths and timestamps | Not soon, and honestly stated |
| Updater | The sync service | Phase 5 |
