# Automation & Machine Learning

*Access date: 2026-09-21.*

**Policy, decided with the project owner:** local on-device models by default, with an optional
user-supplied API key for an external model where the hard cases justify it. Every feature must
work fully offline with zero cost; the key is strictly additive.

## 1. Where the labour actually is

Genealogical work, ranked by hours consumed:

1. **Reading handwriting.** Parish registers, revision lists, confession books. This dominates
   everything else.
2. **Finding the same person across records** under variant spellings, ages and places.
3. **Transcribing and structuring** what was read into a data model.
4. **Organising photographs** — who is in them, when, where.
5. **Writing up** conclusions with citations.

Automation should attack these in order. Most tools automate (3) — the part that was never the
bottleneck.

## 2. Handwritten text recognition (HTR)

### State of the art

Current engines evaluated in the literature: **PyLaia, HTR+, IDA, TrOCR-f**, and Transkribus'
proprietary Titan "supermodel". Transkribus reports a vendor-measured **6.7 % CER** for Text
Titan I across Latin scripts out of the box; models trained on a *specific* hand reach
**85–95 % word accuracy**. Transkribus offers 300+ public models across 100+ languages and
scripts. **[current]**

**The critical finding for us: support for non-Latin scripts, Cyrillic included, lags Latin
tooling by years.** Titan and TrOCR-f lead out of the box on Latin; **PyLaia, IDA and HTR+ do
better on specific non-Latin scripts when fine-tuned**. **[current]**

Open Cyrillic material does exist — `cyrillic-trocr/trocr-handwritten-cyrillic` on Hugging Face
is fine-tuned on Church Slavonic, Russian and Ukrainian handwriting. That is our starting point,
not our destination.

LLM-based transcription is an active and credible line (*Unlocking the Archives: Using Large
Language Models to Transcribe Handwritten Historical Documents*, arXiv:2411.03340) and is the
natural home for the optional API key.

### Our approach

A **pipeline**, not a single model, because the gains at our scale come from structure:

```
scan → deskew, dewarp, denoise
     → layout analysis (columns, ruled table cells, marginalia)
     → line segmentation
     → line recognition (ONNX, TrOCR-Cyrillic base, fine-tunable)
     → language model rescoring against a domain lexicon
          (given names, surnames, patronymics, place names, liturgical formulae,
           and the fixed phrasing of metric-book entries)
     → structured field extraction for known register forms
     → human verification UI, every field linked to its image region
     → corrections feed the fine-tuning set
```

Three things make this work where a bare model would not:

- **Metric books are forms.** A birth entry has a fixed skeleton: date, name, parents with
  estate and parish, godparents, officiating priest. Knowing the template converts free
  recognition into constrained field-filling, which is far more accurate.
- **The lexicon is small and known.** A closed vocabulary of Russian given names, patronymic
  forms, regional surnames and place names from the tree itself makes rescoring very effective.
- **Corrections compound.** Every human correction is training data for *this user's* hand.
  Per-collection fine-tuning is where the 85–95 % figures come from.

**Never auto-commit a transcription.** Recognised text enters as an **Assertion with low
confidence and a link to the image region**, awaiting human confirmation. This is the
assertion model (see [data-standards.md](./data-standards.md) §4) doing exactly the job it was
designed for.

## 3. Record linkage / duplicate detection

The theory is settled. **Fellegi–Sunter** (1967–69) remains the basis of most probabilistic
record linkage: for each candidate pair, decide *link*, *possible link*, or *non-link*, so as to
minimise the possible-link zone while controlling both error rates. The three model families in
current use are Fellegi–Sunter derivatives, machine-learning methods, and Bayesian methods.
Genealogy is named as one of the primary application domains. **[established]**

### Our pipeline

```
blocking      → candidate pairs only, via Daitch–Mokotoff codes, birth-decade buckets,
                and place hierarchy — never O(n²)
comparison    → per-field agreement vectors:
                  name      Beider–Morse phonetic + Jaro–Winkler on the original script
                  dates     interval overlap, not equality
                  places    distance in the place hierarchy, then geographic distance
                  relations Jaccard over the set of linked relatives  ← the strongest signal
scoring       → Fellegi–Sunter weights, with m and u probabilities estimated by EM
                on the tree's own data
thresholds    → auto-link / review queue / reject, thresholds set per tree
explanation   → every score decomposed into its field contributions, always shown
```

**[judgement]** Relationship overlap is the feature generic entity-resolution libraries lack and
the one that matters most here. Two "Ivan Petrov, b. ~1871" records are ambiguous; two with the
same wife and three same-named children are the same man. Because of this, and because we need
score explanation in the UI rather than a black-box verdict, we implement this ourselves in
Rust rather than adopting a general ER library.

**Never auto-merge.** Detection proposes; a human disposes. A merge proposal is a Contribution
like any other.

## 4. Photographs

| Task | Approach | Model class |
|---|---|---|
| Face detection | On-device, every imported photo | RetinaFace / SCRFD class, ONNX |
| Face embedding + clustering | Group unknown faces, propose identities from already-tagged people | ArcFace-class embedding + HDBSCAN |
| Quality / duplicate detection | Perceptual hash + blur metric | Classical |
| Scan enhancement | Deskew, crop, colour-cast correction, dust and scratch removal | Classical + light CNN |
| Colourisation | Explicitly marked as an interpretation, never overwriting the original | Diffusion or CNN colouriser, ONNX |
| Restoration | Same rule: a derivative, never a replacement | Restoration model, ONNX |
| Date estimation | Suggest a decade from clothing, print format, photographer's mark | Classifier, low confidence by default |

**Inference runtime: ONNX Runtime**, with the native build using the platform execution provider
(DirectML on Windows, CoreML on Apple, NNAPI on Android) and the web build using WebGPU with a
WASM fallback. One model format, one calling convention, five platforms.

**Two hard rules.**

1. **The original file is immutable.** Every enhancement is a derivative with recorded
   provenance. The archival original is what gets preserved and exported.
2. **Generated content is labelled, permanently and visibly.** A colourised photograph carries
   the label in the UI, in exports, in prints and in its metadata. It never silently becomes
   the family's memory of what someone looked like.

### On synthetic animation

MyHeritage's Deep Nostalgia and LiveMemory animate still portraits of deceased people. It is the
single most viral feature in consumer genealogy and we are deliberately not making it a headline
feature. The reasoning is in [art-direction.md](../03-design/art-direction.md). Should it ever
ship, it is opt-in per photograph, per person, indelibly watermarked, and excluded from every
archival export.

## 5. Text and research assistance

Local, small models handle: entity extraction from typed transcripts, name and place
normalisation, translation of short archival phrases, and summarising a long research note.

The **optional user API key** unlocks: difficult-hand transcription assistance, interpretation
of unusual document types, drafting a proof argument from the assertions on record, and
free-text search over the whole archive. Configured per user, never required, and every
AI-produced statement enters as a low-confidence assertion attributed to the model — never as a
fact.

## 6. Summary of obligations

| Capability | Decision |
|---|---|
| HTR | Own pipeline: layout → segmentation → ONNX recognition → lexicon rescoring → form-aware extraction → verification UI |
| HTR base model | Cyrillic TrOCR fine-tune, with per-collection fine-tuning from user corrections |
| Record linkage | Own Rust implementation: DM-Soundex blocking, Fellegi–Sunter with EM, relationship-overlap feature, explained scores |
| Faces | On-device detection, embedding and clustering; proposals only |
| Photo enhancement | Derivatives only; originals immutable; generated content permanently labelled |
| Runtime | ONNX Runtime with per-platform execution providers; WebGPU on the web |
| External models | Optional user-supplied key; output always enters as low-confidence attributed assertions |
| Universal rule | **No model ever writes a fact. Models propose; humans decide.** |
