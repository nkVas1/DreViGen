# Competitive Landscape

*Access date: 2026-09-21.*

## 1. The reference products

### MyHeritage — the consumer benchmark

**What it does well.** Onboarding is close to frictionless. **Smart Matches** cross-reference
your people against other users' trees; **Record Matches** do the same against a licensed
archive corpus spanning vital records from 66 countries, censuses, immigration and burial
records. The photo suite — Colorize, Enhance, **Deep Nostalgia** (portrait animation),
**LiveMemory** (still → short video), **AI Time Machine** — is the reason most casual users stay.
**AI Record Finder** is a chat interface over the record corpus. **[current]**

**Where it fails us.** Everything meaningful is a subscription. The research apparatus is thin:
no serious conflicting-evidence model, no proof arguments, weak citation discipline. Your data
is on their server. Export is lossy.

**What we take.** The onboarding curve. The idea that photographs — not boxes and lines — are
what makes a family tree emotionally legible. The matching *concept*, re-implemented against
your own corpus rather than a licensed one.

**What we reject.** Synthetic animation of deceased people as a headline feature. See the ethics
note in [art-direction.md](../03-design/art-direction.md).

### MobileFamilyTree 11 / MacFamilyTree 11 (Synium) — the craft benchmark

Shares an engine with MacFamilyTree 11. Ships an unusually deep chart catalogue: **Ancestor
Chart, Genealogical Chart, Hourglass Chart, Kinship Chart, Double Ancestor Chart, Statistics,
Timeline, Genogram**, plus an **Interactive Family Tree** and a **Virtual Tree 3D** view.
Version 11 rebuilt places management end to end: **Landmarks and Place Details, geographic
coordinate and alternative-name search, Wikipedia integration for places**, an **Influential
People** feature, automatic backups, and **Web Export** for generating a family-history
site. **[current]**

**What it does well.** Breadth and polish of visual output. Place handling is the best in class.
The product understands that genealogy output is often a *gift* — a chart, a book, a site.

**Where it fails us.** Apple-only. Collaboration is file-shaped, not change-shaped: no
attribution of who asserted what, no review workflow. Single-researcher assumptions throughout.

**What we take.** The chart catalogue as a minimum bar. First-class places with hierarchy,
coordinates, alternative names and external enrichment. Export-as-artefact (book, poster, site)
treated as a core feature rather than an afterthought.

## 2. The open-source field

| Project | Stack | Strengths | Limits |
|---|---|---|---|
| **Gramps / Gramps Web** | Python desktop + web companion | The deepest free data model; strong sourcing; huge report library; extensible; FamilySearch integration in beta for 6.1 | Steep learning curve by its own admission; desktop UI shows its age; web companion is a separate product |
| **webtrees** | PHP + MySQL/Postgres/SQLite | Genuinely collaborative, multi-user, web-native, GEDCOM-faithful | Server-first (no offline), dated interaction model, PHP hosting requirement |
| **Ancestris** | Java | Closely respects GEDCOM; many views — tree, geographic, chronological, tables, reports | Java desktop UX; anchored to GEDCOM 5.5 rather than 7 |
| **TNG** | PHP, paid licence | Mature site-builder | Not open source |

**The crucial borrowing — the Gramps object model.** Gramps separates People, Families, Events,
Places, Sources, Citations, Repositories, Notes, Media and Tags into independent first-class
objects that reference one another. Two structural decisions are worth copying outright:

1. **Events are not fields of a person.** A birth, death or marriage is an object in its own
   right with a type, date, place and description. People attach to events *through a role* —
   primary subject, witness, officiant, informant. This is what makes a marriage record
   naturally hold six participants instead of two.
2. **Citation sits between Source and fact.** A *Source* is the document; a *Citation* is a
   specific place within it (page, entry, image) carrying a confidence assessment. Multiple
   objects cite the same source at different points.

Gramps also nests Places hierarchically, so a village is defined once and referenced by every
event that happened there. **[established]**

**What we improve.** Gramps stores a researcher's *conclusions*. It has no native notion of
"source A says 1871, source B says 1873, here is my reasoning for preferring A". We make
conflicting assertions first-class — see [data-standards.md](./data-standards.md), section 4.

## 3. Collaborative-tree prior art

**WikiTree** runs a single shared world tree with a human merge workflow: when two profiles
describe the same person they must be merged; the merging user picks which fields and which
parents survive; biographies, sources, photos and memories are combined. Disagreements can be
**rejected** or parked as an **Unmerged Match**, and disputes move to a discussion thread
attached to the profile plus a Research Notes section summarising the issue. **[current]**

**Lesson.** A merge UI that forces a field-by-field decision is *correct* but exhausting. Parking
a disagreement ("unmerged match") is a genuinely good primitive — it lets honest uncertainty
persist instead of forcing a false resolution. We adopt both: field-level merge resolution, and
a first-class **Открытый вопрос** (open question) state.

**Lesson two.** WikiTree's governance problem — strangers merging your ancestors — comes from the
single-world-tree premise. Per-family canonical trees avoid it entirely. This is why our
non-goals rule out a universal tree.

## 4. The academic read

Siu et al. (arXiv:2411.07869), semi-structured interviews with 20 genealogists of mixed
expertise, is the most current HCI study of the field. Its findings that bear on our design:

- The field is **professionalising and standardising**; tools that ignore research standards are
  increasingly seen as toys by serious users.
- Software plays a **critical role in education** — people learn genealogical method *from* their
  tools. A tool that lets you record an unsourced guess without friction is teaching a bad habit.
- The **amateur/expert experience gap** is the central unsolved problem.

**Obligation.** Our two-profile design is not a marketing split; it is a direct response to the
documented gap. The Hall must teach, gently, by making the Workshop's discipline visible — a
fact without a source should *look* different from a fact with one, even to a casual viewer.

## 5. Summary of obligations

| Finding | What we must build |
|---|---|
| Gramps event/role and source/citation model is right | Adopt it, extend it with assertion-level provenance |
| MFT 11 chart catalogue is the visual bar | Ancestor, descendant, hourglass, fan, kinship, genogram, timeline, statistics — plus our own |
| MFT 11 places model is best in class | Hierarchical places with coordinates, alternative names, external enrichment |
| WikiTree merges are correct but exhausting | Field-level resolution UI + a parked-disagreement state |
| MyHeritage owns onboarding | First run must reach a recognisable face in under two minutes |
| Research standards are professionalising | GPS-aware research log, proof arguments, citation discipline surfaced in the UI |
| Nobody serves amateurs and experts in one document | Hall / Workshop duality as the core product architecture |
