# Designing for Older Adults — Evidence and Obligations

*Access date: 2026-09-21. This note governs every interface decision in the project.*

## 1. The framing correction

The instinct when told "make it usable for an older person" is to make everything bigger. The
evidence says the problem is rarely acuity. A 2026 analysis of geriatric digital care
(arXiv:2601.17012) argues the barrier is **usability, not access** — the devices and connections
exist; the interfaces defeat people.

WCAG 2.2 (W3C Recommendation, 2023) reflects the same shift. Its new success criteria are about
**focus visibility, authentication burden, error prevention, pointer targets and consistency** —
that is, about cognitive load, time pressure and inconsistent interaction patterns, not about
sensory impairment. W3C states plainly that WCAG 2.2 makes content more usable by older
individuals with changing abilities. **[established]**

Our persona is specific and useful: *he sees fine, but he gets lost.* Getting lost is a
navigation and feedback problem.

## 2. What the evidence actually says about older users

- **High contrast is preferred**, and preference here tracks measured performance.
- **Skeuomorphic cues are preferred** — objects that look like the thing they act as. This is the
  finding that most contradicts the last decade of flat-design orthodoxy, and it is the reason
  our chosen art direction is materially textured rather than abstractly flat.
- **Icons must carry text labels.** Older users are less familiar with symbol conventions and are
  measurably more often confused by unlabelled icons. **No icon in DreViGen ships without a
  visible text label** — not a tooltip, a label.
- **Busy pages cause cognitive overload, anxiety and loss of focus.** W3C's COGA guidance
  (*Making Content Usable for People with Cognitive and Learning Disabilities*) advises keeping
  each view to a small number of important points; doing so calms the user, aids comprehension
  and supports memory.
- **Consistent navigation and reduced memory load** are the load-bearing principles: plain
  language, scannable structure, forgiving forms. **[established]**

## 3. WCAG 2.2 criteria that shape the build

We treat AA as a floor and take AAA where it is achievable. These specific criteria have direct
architectural consequences:

| Criterion | Consequence for DreViGen |
|---|---|
| **2.4.11 Focus Not Obscured** | Sticky headers, toolbars and the inspector panel must never cover the focused element. Scroll-into-view accounts for all fixed chrome. |
| **2.4.13 Focus Appearance** | A focus ring of at least 2 px, contrasting 3:1 against both the component and the background, on **every** interactive element including canvas nodes. |
| **2.5.7 Dragging Movements** | Every drag has a non-drag equivalent. Reparenting a person by dragging is a convenience; the menu path is the guarantee. |
| **2.5.8 Target Size (Minimum)** | The standard is 24×24 CSS px. **We adopt 44×44 as the project minimum**, 48×48 for primary actions, with a spacing floor so adjacent targets cannot be mis-hit. |
| **3.2.6 Consistent Help** | Help occupies the same position on every screen, always. |
| **3.3.7 Redundant Entry** | Never ask twice for information already given in a flow. Adding three siblings should not mean typing the surname three times. |
| **3.3.8 Accessible Authentication** | No cognitive-function test to log in. No puzzle CAPTCHAs, no "type the third letter of your password". Passkeys first, with an emailed magic link as the always-available fallback. |
| **1.4.3 / 1.4.11 Contrast** | Text and non-text contrast verified with **APCA** alongside WCAG 2 ratios (see below). |
| **2.2.1 Timing Adjustable** | No session timeouts that lose work. Drafts persist indefinitely. |
| **2.3.3 Animation from Interactions** | `prefers-reduced-motion` honoured throughout, with an in-app override that does not require finding an OS setting. |

## 4. Contrast: APCA alongside WCAG 2

WCAG 2's relative-luminance ratio is known to misjudge light-text-on-dark pairings, where APCA's
perceptual lightness contrast (Lc) is more accurate. Current design-system practice is to
compute both and use **APCA as the primary guide for text colour tokens**, with the WCAG ratio
retained for formal conformance reporting. **[current]**

**Obligation.** The token pipeline emits both values for every foreground/background pair and
fails the build on a violation. Contrast is a test, not a review comment.

## 5. Project-specific rules, beyond the standards

These come from the persona rather than from any guideline, and they are binding.

1. **Never lose the user's place.** Layout stability (see
   [layout-and-rendering.md](./layout-and-rendering.md) §4) is an accessibility requirement here,
   not a performance nicety. If the canvas reshuffles, he is lost.
2. **One visible way home, always.** A persistent control that returns to a known state — his
   own record, at a readable zoom. It never moves and never changes its label.
3. **No dead ends.** Every screen states what it is and offers a way back. Empty states explain
   what would fill them and how.
4. **Destructive actions are undoable, not confirmed.** A confirmation dialog is a question he
   has to evaluate under pressure; an undo is a mistake he can fix calmly. Where confirmation is
   unavoidable, it names the specific object ("Удалить Ивана Васильева?"), never "this item".
5. **Say what happened.** Every state change announces itself in words, in place — not only
   through motion or colour. Screen-reader announcements and visible text are the same message.
6. **Text floor 17 px**, body default 19 px, and a document-level scale control offering 100 /
   125 / 150 / 200 % that reflows rather than zooms.
7. **Modals are a last resort.** Editing happens in place or in a persistent side panel. Never a
   modal above a modal.
8. **The Hall never shows Workshop chrome.** No citation confidence pickers, no merge queues, no
   GEDCOM tags on his path. Reachable in one clearly labelled step; never present by default.
9. **Plain language, checked.** Interface copy is reviewed for reading level. "Синхронизировать
   репозиторий" is wrong; "Отправить мои изменения семье" is right.
10. **Test with the actual user.** Each phase ends with an unassisted session with the persona,
    recorded against a fixed task list. His failures are bugs with severity, not feedback.

## 6. How this is enforced

| Mechanism | Scope |
|---|---|
| Lint rule: no icon-only interactive element | Build-time, blocking |
| Token pipeline contrast check (APCA + WCAG 2) | Build-time, blocking |
| Minimum target size assertion in component tests | CI, blocking |
| `axe-core` automated audit on every route | CI, blocking on new violations |
| Keyboard-only traversal test of core flows | CI, Playwright |
| Screen-reader smoke test (NVDA / VoiceOver) | Per release, manual |
| Persona session with the fixed task list | Per phase, recorded |

Automated tooling catches perhaps a third of real accessibility problems. The persona session is
the part that matters.

## 7. Summary of obligations

- 44×44 minimum target, 48×48 for primary actions.
- Every icon labelled in text. No exceptions.
- APCA-validated contrast, enforced at build time.
- Layout stability treated as an accessibility requirement.
- Undo over confirmation; named objects in any unavoidable confirmation.
- Passkeys with magic-link fallback; no cognitive-test authentication.
- `prefers-reduced-motion` honoured, with an in-app override.
- Text scale control at 100/125/150/200 % that reflows.
- Hall and Workshop strictly separated by default, one labelled step apart.
