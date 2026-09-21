# Visual & Motion Design — State of the Art, 2026

*Access date: 2026-09-21.*

## 1. The defining tension of 2026

Product design in 2026 is actively pulling away from **AI-generated sameness**. The reasoning
repeated across the design press is consistent and worth stating exactly: *AI has made imitation
effortless, so distinction now comes from decisions only a human would make.* Brands are moving
away from generic, interchangeable interfaces toward **ownable visual systems** — the test being
whether the work is recognisable with the logo removed, through its typographic rhythm,
photographic treatment, grid or restraint in motion. **[current]**

Two movements dominate the response:

**Neobrutalism** — sharp contrast, heavy borders, geometric shapes, deliberately raw layouts.
Explicitly described as what happens when designers refuse to look AI-generated. **We reject
it**: it is loud, it is now itself a template, and it is hostile to our primary persona.

**Tactile maximalism / materiality** — rich sensory texture (paper, grain, ink, glass) deployed
against digital sterility. The four textures named as leading in 2026 are **halftone**
(nostalgia), **risograph** (rebellion), **ink-spoiled** (authenticity) and **liquid grain**
(futurism). The key observation from the trade press: *texture reads as premium because it looks
slow and deliberate to produce.* **We take this**, selectively — halftone and ink, not
risograph or liquid grain. **[current]**

This is a fortunate convergence. The research on older users independently recommends
**skeuomorphic, high-contrast, materially legible** interfaces. The 2026 anti-AI-slop movement
recommends **texture, craft and material metaphor**. Our persona requirement and our originality
requirement point the same way, which almost never happens.

## 2. Typography

Typography has become the primary differentiator in 2026 precisely because it distinguishes
without asset weight: **variable fonts, fluid type scales, stronger headline systems, more
editorial hierarchy**. Type is increasingly treated as the interface itself rather than a layer
over it. **[current]**

### Constraints specific to us

The type system must cover **Cyrillic, extended Latin and German** to a professional standard,
be open-licensed, and ideally be variable. That eliminates most of the fashionable display faces.

Verified open-licensed candidates with genuine Cyrillic coverage:

| Face | Class | Note |
|---|---|---|
| **Source Serif 4** | Transitional serif | Variable, weight 200–900, Latin + Cyrillic + Greek. Reliable text serif. |
| **Theano Didot** | Didone | Alexey Kryukov, OFL 2007, a Didot revival — the historically correct Didone with Cyrillic. Static, display use only. |
| **PT Serif / PT Sans** | Transitional / humanist | Purpose-built for Cyrillic by ParaType; unmatched Cyrillic quality; not variable. |
| **Noto Serif / Noto Sans** | Neutral | Latin, Greek, Cyrillic; the safety net for scripts nothing else covers. |
| **Archivo** | Grotesque | Variable on weight *and* width, 200+ languages. Good for dense tabular UI. |
| **Inter** | Neo-grotesque | Excellent screen legibility. Ubiquitous — which is precisely the "AI-slop" signal we avoid for display, though it is defensible for dense data. |

**[judgement]** The pairing decision belongs in the art-direction document, not here. The
research conclusion is narrower: the botanical-plate direction is typographically achievable
without commissioning type.

**Corrected 2026-09-21 by measurement** — [typefaces.md](../03-design/typefaces.md). The list
above was assembled from secondary sources and two of its entries are unusable: **Archivo has
no Cyrillic at all**, and neither does Bodoni Moda. Theano Didot is real but not distributable
through the normal pipeline. The chosen set is Prata, Source Serif 4, Golos Text, Caveat and
JetBrains Mono, each verified against the actual character sets the product must set. The
lesson generalises: a font's marketing copy is not a coverage report.

## 3. Colour

**OKLCH is the practical perceptually-uniform space for design systems.** Equal lightness steps
in OKLCH look equal; in HSL they do not, because a 10 % lightness change reads very differently
across hues. Adjusting OKLCH's L channel gives the same perceived brightness change regardless
of hue or chroma, which is what makes accessible ramps tractable. The de-facto modern ramp is
**eleven stops (50–950)**, matching Tailwind, Radix Colors and shadcn/ui. **[established]**

**Obligation.** Every colour is authored in OKLCH, gamut-mapped to sRGB for output, with both
APCA (Lc) and WCAG 2 contrast computed for every foreground/background pair in the token
pipeline. Colour-blindness simulation runs in the same pass.

## 4. Texture and material

Implementation matters as much as the decision. The performance guidance is clear: CSS and SVG
noise avoid the processor cost of WebGL approaches, and **a ~50 px repeating transparent noise
tile can add grain across an entire page at negligible weight**. **[current]**

**Obligation.**

- Paper grain: one tiled PNG/AVIF noise texture plus a CSS blend mode. Not a per-frame shader.
- Never animate the grain. Animated film grain is a per-frame full-screen composite and will
  cost the canvas its frame budget.
- Texture belongs to **surfaces**, never to **text**. Grain behind a label reduces legibility,
  which is the one thing we will not trade.
- All texture is suppressed in high-contrast mode and at text scale ≥ 150 %.

## 5. Motion

The 2026 consensus, which is unusually specific and well-quantified:

### Duration

- **200–300 ms** for most UI transitions. Over 500 ms reads as sluggish; under 100 ms reads as
  jarring.
- Material Design 3's scale, useful as calibration: **300–700 ms** for large transitions with
  emphasised easing, **100–300 ms** for component animation with standard easing, **50–150 ms**
  for state-layer feedback. **[established]**

### Easing

- Entering: **ease-out** — fast start, gentle stop.
- Exiting: **ease-in** — gentle start, accelerating away.
- Moving between states of the same element: **ease-in-out**. **[established]**

### Spring physics

Springs respond correctly to interruption: reverse a gesture mid-flight and a spring reverses
from its current velocity instead of jumping, which Bézier easing cannot do. Published parameter
guidance: **damping 20–30 for a professional feel**, 8–12 for playful bounce; **stiffness 500
with damping 30** settles quickly. **[current]**

**[judgement]** Everything a user directly manipulates — canvas pan and zoom, drawer and panel
motion, card expansion — is spring-driven, damping 26–30. Nothing in DreViGen bounces. A family
archive that boings is a family archive nobody trusts.

### Choreography

Motion hierarchy mirrors visual hierarchy: **not everything moves equally**. Lead the eye with
the most important element, let secondary elements follow in a stagger. And — the point most
often missed — **the same interaction must always use the same animation, timing and easing**,
because consistency is part of the product's language. **[established]**

### Platform capability

The **View Transitions API** is stable in Chromium and broadly supported; `animation-timeline`,
`scroll()` and `view()` have been consistent across the major engines since mid-2026.
**Motion 13.4.0** wraps view transitions with spring support and proper interruption handling
via `animateView`. CSS `linear()` allows spring curves without JS. **[current]**

**Obligation.** Native CSS and View Transitions for anything they can express — they run off the
main thread and cost nothing. Motion (the library) only where interruption handling or layout
animation genuinely requires it. The canvas has its own spring integrator in the render loop,
sharing the same parameter constants so canvas motion and DOM motion feel identical.

## 6. The synthesis for DreViGen

| Layer | 2026 principle applied |
|---|---|
| **Material** | Tactile materiality — paper, ink, engraved line. Selective halftone and ink texture. No risograph, no liquid grain, no glassmorphism. |
| **Type** | Type as interface: a Didone display voice with real Cyrillic, a variable text serif, a tabular-figure grotesque for data. Fluid editorial scale. |
| **Colour** | OKLCH-authored, APCA-validated, eleven-stop ramps. Warm archival base, restrained accents. |
| **Motion** | Spring-driven direct manipulation at damping 26–30; 200–300 ms standard transitions; strict consistency; nothing bounces. |
| **Composition** | Editorial rigour — deliberate visual weight, rhythm and pause. The one thing taken wholesale from the neo-grotesque school. |
| **Ownability test** | Remove the logo. If a screenshot is not recognisably DreViGen, the design has failed. |
