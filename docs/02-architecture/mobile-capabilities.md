# Mobile Capability Matrix

*Spike S5, 2026-09-21. Sources: the [official plugin support table](https://github.com/tauri-apps/plugins-workspace)
and crates.io metadata read on the same date.*

[ADR 0002's premise](./adr/0002-react-for-accessibility.md) and the platform choice in
[overview.md](./overview.md) commit us to shipping the same application to phones as to desktops.
This note answers the question that commitment raises: **what can Tauri 2 actually do on iOS and
Android, and what do we have to build ourselves?**

The pass criterion for S5 was a written matrix in which every gap has an owner and an estimate.

## 1. What we need, and whether it exists

Legend: ✅ supported · ⚠️ untested upstream · ❌ not supported · 🔨 we build it

| Capability | Why DreViGen needs it | Plugin | iOS | Android | Verdict |
|---|---|---|---|---|---|
| **File dialogs** | Import a GEDCOM; save a `.dvg` | `dialog` (official) | ✅ | ✅ | Ready |
| **Filesystem** | Read and write the project file | `fs` (official) | ⚠️ | ⚠️ | **Not needed** — see §2 |
| **Android document storage** | Save a `.dvg` where the user can find it under scoped storage | `tauri-plugin-android-fs` v29.0.0, updated 2026-07-22, 67 k downloads | n/a | ✅ | Adopt |
| **Camera** | Photograph a document or a print in an archive | — | 🔨 | 🔨 | **Gap — we build it** |
| **Photo library** | Import the shoebox of family photographs | — | 🔨 | 🔨 | **Gap — we build it** |
| **Share sheet** | Send a chart or a person link to a relative | — | 🔨 | 🔨 | **Gap — we build it** |
| **Biometrics** | Unlock a tree marked private | `biometric` (official) | ✅ | ✅ | Ready |
| **Haptics** | The tactile layer of the art direction | `haptics` (official) v2.3.3, updated 2026-09-13 | ✅ | ✅ | Ready |
| **Notifications** | A contribution needs review | `notification` (official) | ✅ | ✅ | Ready |
| **Deep links** | Open a person from a shared URL | `deep-link` (official) | ✅ | ✅ | Ready |
| **HTTP** | Sync with the server | `http` (official) | ✅ | ✅ | Ready |
| **WebSocket** | Presence and live notifications | `websocket` (official) | ⚠️ | ⚠️ | **Not needed** — see §2 |
| **Key–value store** | Settings, last-opened tree | `store` (official) | ✅ | ✅ | Ready |
| **Open external URL** | Follow an archive citation to its source | `opener` (official) | ✅ | ✅ | Ready |
| **Upload** | Send media to the sync server | `upload` (official) | ✅ | ✅ | Ready |
| **OS info, logging** | Diagnostics | `os`, `log` (official) | ✅ | ✅ | Ready |
| **Clipboard** | Copy a citation | `clipboard-manager` (official) | ✅ | ✅ | Ready |
| **SQL** | — | `sql` (official) | ✅ | ✅ | **Not used** — see §2 |
| **In-app update** | — | `updater` (official) | ❌ | ❌ | Correct: the stores update mobile apps |
| Autostart, CLI, global shortcuts, window state, single instance | Desktop conveniences | official | ❌ | ❌ | Desktop-only by design |

## 2. Three entries that look like gaps and are not

**`fs` is marked untested on mobile, and it does not matter.** That plugin exists to expose the
filesystem *to the JavaScript layer* with a scope model. DreViGen does its file I/O in the Rust
core ([ADR 0001](./adr/0001-rust-core-on-every-platform.md)), inside the application sandbox,
using `std::fs` — which works on both mobile platforms without any plugin. The plugin's status
is upstream's uncertainty, not ours. **Verification task P0-S5a** confirms sandbox read/write
on both platforms before Phase 1 relies on it.

**`websocket` is marked untested, and we will not use it.** The plugin wraps a Rust client for
the JS layer. Our sync lives in the core, so it uses a Rust WebSocket client directly, on the
same code path as desktop. Presence is an enhancement in any case; if a platform proves hostile,
notifications degrade to polling without touching the product.

**`sql` is supported and we still will not use it.** Our SQLite access goes through
`drevigen-store` in the Rust core, which owns the schema, the migrations and the FTS5 index.
Routing the same database through a second, JS-facing plugin would create two writers to one
file — the defect class we least want in a product whose entire value is not losing data.

## 3. The real gaps

Three capabilities have no maintained plugin, and all three are load-bearing for the
**«Хранитель»** persona — the family keeper with a shoebox of photographs. Without camera and
photo-library access, that persona cannot use the mobile app for the one thing they want it for.

### 3.1 Camera and photo library — **we build it**

What exists, and why none of it is adoptable:

| Candidate | Version | Last updated | Downloads | Assessment |
|---|---|---|---|---|
| `tauri-plugin-camera` (kessdev) | 0.1.4 | **2025-06-23** | 4 067 | Fifteen months stale, pre-1.0, thin usage |
| `tauri-plugin-native-camera` | 0.1.0 | 2026-01-16 | 249 | Android-only in practice; essentially unused |
| `nanderstabel/tauri-plugin-camera` | not published | — | — | A copy of a removed official plugin |

**[judgement]** Photo import is not a peripheral feature we can afford to have break on an OS
update. A dependency at 0.1.x with four thousand downloads and no commits in over a year is a
liability on the product's most emotionally important path, and the fix would land on us anyway
the first time it broke. We write it.

Scope: `ACTION_IMAGE_CAPTURE` and the Photo Picker on Android; `UIImagePickerController` /
`PHPickerViewController` on iOS. Permissions declared per platform. **EXIF must survive the
round trip** — capture date and camera model are genealogical evidence, and a picker that strips
them is worse than useless. Desktop falls back to the `dialog` file picker, which is the correct
behaviour there anyway.

**Owner:** core. **Estimate:** 7 days for both platforms including permissions, EXIF
preservation, and the Rust and TypeScript surface. **Scheduled:** Phase 1, alongside media
import.

### 3.2 Share sheet — **we build it**

`tauri-plugin-sharesheet` sits at 0.0.1, last touched **2024-08-29** — two years — despite
eleven thousand downloads, which says the ecosystem wants this and nobody is maintaining it.

Scope: `Intent.ACTION_SEND` on Android, `UIActivityViewController` on iOS, sharing a file (a
generated chart or book) or a URL (a deep link to a person). Desktop uses `opener` and the
platform's own mechanisms.

**Owner:** core. **Estimate:** 3 days. **Scheduled:** Phase 4, with the export pipeline that
gives it something to share.

### 3.3 Android scoped storage — **adopt**

`tauri-plugin-android-fs` v29.0.0 was updated two months ago and has 67 000 downloads: actively
maintained and widely used. It covers the Storage Access Framework, which is how a `.dvg` file
reaches a location the user can actually find. Adopt rather than build.

**Owner:** core. **Estimate:** 1 day to integrate. **Scheduled:** Phase 1.

## 4. Tasks this produces

| # | Task | Phase | Estimate |
|---|---|---|---|
| P0-S5a | Verify Rust `std::fs` read/write in the app sandbox on both mobile platforms | 0 | 0.5 d |
| P0-S5b | Verify the Rust WebSocket client connects from both mobile platforms | 0 | 0.5 d |
| P1-M1 | Write `drevigen-plugin-media`: camera and photo library, EXIF-preserving | 1 | 7 d |
| P1-M2 | Integrate `tauri-plugin-android-fs` for scoped storage | 1 | 1 d |
| P4-M3 | Write `drevigen-plugin-share`: share sheet on both platforms | 4 | 3 d |

**Total unplanned mobile work surfaced by this spike: 12 days.** That is the number the spike
existed to find, and finding it in week three is the point — the alternative was discovering it
in month eleven, which is how mobile ports die.

## 5. What this does not cover

- **Background sync.** Not investigated, because sync is user-initiated by design
  ([ADR 0003](./adr/0003-contribution-review-over-auto-merge.md)). If that ever changes, both
  platforms' background-execution limits become a research topic of their own.
- **Push notifications** as distinct from local ones. The `notification` plugin covers local
  notifications; server-driven push needs APNs and FCM, which is Phase 3 work with the server.
- **Store submission requirements** — privacy manifests, data-safety declarations, age ratings.
  Phase 6, and non-trivial for an application that handles personal data about living people.
