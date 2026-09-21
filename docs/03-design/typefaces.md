# Typeface Audit

*Measured 2026-09-21 with `tools/audit-fonts.py`. Re-run it before changing any of these.*

The art direction named four voices. Naming a typeface is not the same as checking it can set
the text, and one of the four could not: **Archivo has no Cyrillic at all**. A Russian-speaking
user would have found that, which is the wrong way to find it. This audit exists so the next
substitution is decided by measurement.

## What was measured

Every candidate was checked for coverage of the character sets the product must set — ASCII,
Russian (А–я, Ё, ё), the German additions, and the typographic punctuation Russian text
requires («ёлочки», em and en dashes, №) — plus variable axes, tabular figures and a slashed
zero.

| Voice | Typeface | ASCII | Russian | German | Typographic | Axes | `tnum` | `zero` |
|---|---|---|---|---|---|---|---|---|
| **display** | **Prata** | full | full | full | 12/14 | static | no | no |
| display | Playfair Display | full | full | full | full | wght | no | no |
| display | Bodoni Moda | full | **0/66** | full | full | opsz, wght | yes | no |
| display | PT Serif | full | full | full | full | static | no | no |
| **text** | **Source Serif 4** | full | full | full | full | opsz, wght | yes | yes |
| text | Literata | full | full | full | full | opsz, wght | yes | yes |
| text | Noto Serif | full | full | full | full | wdth, wght | yes | yes |
| data | Archivo | full | **0/66** | full | full | wdth, wght | yes | yes |
| **data** | **Golos Text** | full | full | full | full | wght | yes | **no** |
| data | IBM Plex Sans | full | full | full | full | wdth, wght | no | yes |
| data | PT Sans | full | full | full | full | static | no | no |
| data | Inter | full | full | full | full | opsz, wght | yes | yes |
| **mono** | **JetBrains Mono** | full | full | full | full | wght | no | yes |
| mono | IBM Plex Mono | full | full | full | full | static | no | yes |
| **hand** | **Caveat** | full | full | full | full | wght | no | no |
| hand | Marck Script | full | full | full | 13/14 | static | no | no |

## The chosen set

| Voice | Typeface | Why |
|---|---|---|
| **Заголовок** | **Prata** | A genuine Didone with complete Cyrillic, by Cyreal. Distinctive in the way the brief demands, and single-weight — which costs nothing, because display type is set at one weight. |
| **Текст** | **Source Serif 4** | Complete, variable on optical size *and* weight, with tabular figures and a slashed zero. Optical size matters for a face that runs from a caption to a biography. |
| **Данные** | **Golos Text** | A contemporary Russian grotesque from ParaType. Complete coverage, variable weight, tabular figures. |
| **Рука** | **Caveat** | Complete Cyrillic — which the art direction had flagged as unverified and which is now confirmed. |
| **Шифр** | **JetBrains Mono** | Complete, variable, and — the point — a slashed zero, which is where it is needed. |

## The three decisions worth arguing about

### Prata over Playfair Display, despite Prata being incomplete

Prata is missing exactly two characters: **…** and **№**. Playfair Display is complete and
variable.

Prata wins anyway. Playfair Display is among the most-used serifs on the web, and the brief is
explicit that the work must not read as templated — a face that appears on every second
portfolio undercuts the one thing the art direction is for. Prata is a real Didone with real
Cyrillic, drawn by a foundry that specialises in it.

The two missing characters cost nothing in practice: the ellipsis and the numero sign belong to
running text and to reference strings, not to a plate title, and both fall back to Source Serif 4
and JetBrains Mono respectively. The CSS `font-family` stack makes that automatic and invisible.

### Golos Text over Inter, despite Inter being the only complete candidate

**Inter is the only data-voice candidate with everything** — full coverage, variable on optical
size and weight, tabular figures, and a slashed zero. On the checklist it wins outright.

It is not chosen, and the reason is in the research: Inter is named in
[design-language-2026.md](../01-research/design-language-2026.md) as one of the faces that
signals a generic, AI-default interface. Ubiquity is a real cost for a project whose stated
requirement is to be recognisable with the logo removed.

Golos Text gives up one thing: **the slashed zero**. That matters where `0` and `O` can be
confused, which in this product means archival shelf marks — `ГААО ф.29 оп.1 д.204 л.17об` —
and those are set in **JetBrains Mono**, which has one. The data voice sets dates and counts,
where a lone `0` among digits is unambiguous.

So the functional cost lands somewhere it was already covered, and the distinctiveness is kept.
**If a case appears where the data voice must disambiguate `0` from `O`, that case belongs in
the mono voice**, not in a different data face.

### Bodoni Moda and Archivo are excluded, not compromised

Both have **zero** Cyrillic coverage. There is no trade-off to weigh: a face that cannot set a
Russian name cannot be used in a Russian genealogy application. Recording them here is the
point — the audit should show what was considered and why it failed, or the next person
proposes Archivo again.

## Obligations

- The font stack always names a fallback that covers what the primary face does not. Prata
  falls back to Source Serif 4; the browser does the rest.
- **Re-run `tools/audit-fonts.py` before any typeface change.** The script downloads each
  candidate and measures it; it does not consult anybody's memory, including its author's.
- Every face is OFL. Vendored copies and their licences live in `assets/fonts/`.
- The MSDF atlas set in [ADR 0009](../02-architecture/adr/0009-msdf-atlas-tiers.md) is built
  from these faces. Changing one means re-running `tools/measure-msdf.sh` and re-checking the
  size budget.
