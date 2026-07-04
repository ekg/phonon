# Phonon Feature-Gap Assessment — 2026-07

**Task:** `assess-feature-gaps`
**Date:** 2026-07-03
**Question:** With the stability campaign (wave 1 + wave 2) landed and the engine now
surviving randomized live sessions, **what is still MISSING for Phonon to reach full
feature strength as a live-coding platform** — the vision being *Tidal Cycles
expressiveness + embedded, modulatable synthesis in one environment*?

**Method:** Every claim below was checked against the **actual source tree** (not the
status docs, which are stale — see §1). Inventory commands are cited inline so the
reconciliation is reproducible.

**Inputs read:**
- `docs/audits/improvement-plan-2026-07.md` — deferred (non-wave-1) stability items
- `docs/SYNTHESIS_PARITY_PLAN.md`, `docs/UGEN_STATUS.md` — UGen roadmap (dated 2025-11-13)
- `docs/LIVECODE_COMPATIBILITY_TODO.md`, `docs/DSL_GAPS_DISCOVERED.md` — older gap notes
- `CLAUDE.md` "Next Priority Features"
- The codebase: `src/compositional_compiler.rs` (function table + `Transform` execution),
  `src/compositional_parser.rs` (`Transform` enum, `Statement` enum),
  `src/unified_graph.rs` (`SignalNode` enum — 120 variants), `src/midi_input.rs`,
  `src/pattern_ops*.rs`, `src/mini_notation_v3.rs`.

---

## 1. Reconciliation: CLAUDE.md / status-doc claims vs the actual code

**Headline: nearly every item on CLAUDE.md's "Next Priority Features" list is already
implemented and wired.** The status docs lag the code by months. This is itself a gap
(documentation — see Dimension 5) and the single most important thing to correct, because
future planning keeps re-deriving finished work.

| CLAUDE.md / status-doc says "next / missing" | Actual state (verified) | Evidence |
|---|---|---|
| Pattern DSP params `gain`/`pan`/`speed` | DONE — compiled as audio-node modifiers *and* sample params | `compositional_compiler.rs:2104-2106,2893-2895,3069-3072` |
| `hurry` | DONE — `Transform::Hurry`, executes | `compositional_compiler.rs:36,8399` |
| `chop` | DONE — `pattern.chop(n)` | `compositional_compiler.rs:75,8494` |
| `striate` | DONE — `pattern.striate(n)` | `compositional_compiler.rs:76,8498` |
| `loopAt` | DONE — `Transform::LoopAt` | `compositional_compiler.rs:67,8999` |
| Multi-output `out1:`/`out2:` | DONE — `Statement::OutputChannel`, parsed + compiled | `compositional_parser.rs:59,878`; `compositional_compiler.rs:774` |
| `hush` / `panic` | DONE — `Statement::Hush/Unhush/Panic`, parsed + compiled + TUI keybind (C-h) | `compositional_parser.rs:76-81,973-987`; `compositional_compiler.rs:844-858`; `modal_editor/mod.rs:2337,2346` |
| FM oscillator | DONE | `UGEN_STATUS.md:31`; `SignalNode` in `unified_graph.rs` |
| White noise | DONE | `UGEN_STATUS.md:32` |
| Pulse / PWM | DONE | `UGEN_STATUS.md:33` |
| Pan2 (stereo) | DONE — equal-power, stereo render | `UGEN_STATUS.md:144` |
| Limiter | DONE — brick-wall | `UGEN_STATUS.md:98` |
| Parametric EQ | DONE — 3-band peaking | `UGEN_STATUS.md:113` |
| `struct` (LIVECODE_TODO: "NOT IMPLEMENTED") | DONE — `Transform::Struct` | `compositional_parser.rs:160` enum |
| `stut` multi-repeat (LIVECODE_TODO: "PARTIAL") | DONE — `Transform::Stut` distinct from `Stutter` | `Transform` enum |
| `off` with transform (LIVECODE_TODO: "needs version") | DONE — `Transform::Off` | `Transform` enum |
| `foldEvery` (LIVECODE_TODO: "NOT IMPLEMENTED") | DONE — `Transform::FoldEvery` | `Transform` enum |
| `sew` (LIVECODE_TODO: "NOT IMPLEMENTED") | DONE — `compile_sew` | `compositional_compiler.rs:2516` |
| DAW-style block buffer passing | MOSTLY DONE — `process_buffer_dag` exists; `dag-scratch-arena` (P1) landed (commit `ed35cbc`). Design doc `DESIGN_DAW_STYLE_BUFFER_PASSING.md` remains partly aspirational | `unified_graph.rs`; wave-2 task done |
| Stability: C1 borrow race, render locks, voice pool, NaN, per-sample re-parse, live clock, `Box::leak`, DAG arena | ALL DONE — wave-1 + wave-2 tasks all `[x]` | `wg list`; commits `ed35cbc..0aa2984` |

**Net:** the "essential UGen" and "high-value quick win" lists in CLAUDE.md are **complete**.
The real frontier is elsewhere: **melodic/harmonic pattern support, resonant filters, a
handful of Tidal sampler ops, and one deferred timing bug that undermines the core USP.**

### 1a. Genuinely missing / partial (verified absent in the code)

> **⚠️ SUPERSEDED (2026-07-04, wave3-doc-status-refresh) — see
> [`wave3-scope-2026-07.md` §2](wave3-scope-2026-07.md).** Several rows below were
> accurate on 2026-07-03 but landed during wave 2 / wave 3 and are now DONE. The
> historical table is left intact for the audit trail; corrected status:
> - **Scale quantization / Note names / Chords** — DONE. `compile_scale_modifier`,
>   `compile_chord_modifier`, `compile_note_modifier` are wired
>   (`compositional_compiler.rs:3078-3080`); `test_note_modifier.rs`,
>   `test_chord_dsl.rs`, `test_scale_quantization_dsl.rs`.
> - **Resonant filters** RLPF / RHPF / Resonz / SVF / Allpass / Biquad — DONE.
>   `compile_rlpf`/`rhpf`/`resonz`, `svf_lp..notch`, `bq_lp..notch`,
>   `compile_allpass`; `test_rlpf.rs`, `test_rhpf.rs`, `test_resonz.rs`,
>   `test_svf.rs`, `test_biquad.rs`, `test_allpass.rs`.
> - **Effects: Gate / Expander** — DONE and DSL-wired. `compile_gate:10778`
>   (`gate "..."` → 0/1 signal), `compile_expander:6281` (`expander`/`expand`);
>   `test_noise_gate.rs`, `test_expander_buffer.rs`.
> - **Stereo-Width** — PARTIAL: node code-complete + unit-tested
>   (`src/nodes/stereo_widener.rs`, Mid/Side) but **not yet wired to a DSL name**
>   (usable programmatically only; follow-up filed).
> - **`splice` / `stitch`** — DONE since this audit: `compile_stitch` is wired
>   (`compositional_compiler.rs:2525`) and `splice` routes via the slice family.
>
> Still genuinely open (unchanged): T2/T3/G7 (deferred stability), Ableton Link,
> Median filter, and the heavy Tier-2 DSP listed in §5. See
> [`UGEN_STATUS.md`](../UGEN_STATUS.md) for the full recount (72/91 implemented).

| Feature | State | Evidence |
|---|---|---|
| **Scale quantization in the DSL** (`n "0 2 4" # scale "minor"`) | MISSING (machinery exists, NOT wired). A full `Scale` type with `major/minor/dorian/mixolydian/...` and `midi_to_note_name` lives in `src/midi_input.rs:90,949-958`, but there is **no `scale` function in the DSL function table** and no `compile_scale` | `grep compile_scale` -> none; `midi_input.rs:949` |
| **Note names in mini-notation** (`note "c e g"`, `n "c4 e4"`) | MISSING — `note`/`n` modifiers accept numbers only; no `parse_note_name` in the pattern path | `compositional_compiler.rs:3067-3068` |
| **Chords** (`n "c'maj e'min7"`, `chord`) | MISSING — no `compile_chord`, no chord table in the pattern path | `grep compile_chord` -> none |
| **Resonant filters** RLPF / RHPF / Resonz / SVF / Allpass / Biquad | MISSING — only LPF/HPF/BPF/Notch/Comb/Moog-Ladder implemented | `UGEN_STATUS.md:59-67` |
| **`splice`** (slice with speed-to-fit) | MISSING — only `Slice`/`Striate`/`Chop`/`Bite`/`Chew` | `Transform` enum has no `Splice` |
| **`stitch`** (boolean interleave of two patterns) | KEYWORD ONLY — listed in the known-name table (`compositional_compiler.rs:642`) but no `compile_stitch` (its sibling `sew` IS wired) | `grep compile_stitch` -> none |
| **Effects:** Gate / Expander / Stereo-Width | MISSING (Tier-2) | `UGEN_STATUS.md:110-115` |
| **Continuous signal-pattern modulation at sample rate** (`T3`) | BROKEN/frozen — LFO/signal patterns are sampled once per buffer (~86 Hz stairstep) -> zipper noise. **Directly contradicts the "patterns are sample-rate control signals" headline.** Deferred in the stability plan | `improvement-plan-2026-07.md:76` (pt-F5) |
| **`f32` trigger timekeeping** (`T2`) | PRECISION CLIFF — `last_trigger_time` is `f32` absolute cycle position -> ~4 ms jitter after ~10 h; onset jitter / doubled / dropped triggers | `improvement-plan-2026-07.md:75` (pt-F3) |
| **Voice preservation across swap** (`G7`) | Every `C-x` fades + kills active voices -> amplitude notch on every edit + truncated long samples | `improvement-plan-2026-07.md:66` (D1) |
| **Ableton Link / network tempo sync** | MISSING — MIDI in/out (`midi_input.rs`, `midi_output.rs`) and OSC (`osc_control.rs`, `osc_live_server.rs`) exist, but no Link clock for ensemble play | `ls src/*link*` -> none |

---

## 2. Assessment by dimension

### Dimension 1 — Pattern-language parity with Tidal
**Strong.** The `Transform` enum carries ~90 variants (`Fast/Slow/Rev/Every/FoldEvery/
Iter/Ply/Stut/Off/Jux/JuxBy/Struct/Mask/Sew/Chop/Striate/Slice/Bite/Chew/Chunk/Euclid/
Degrade*/Sometimes*/Within/Zoom/Compress/Palindrome/Swing/...`). The gaps are **melodic**,
not rhythmic:
- **No scale quantization / note names / chords** in the DSL. This is the biggest
  pattern-language gap: a live coder writing melodies reaches for `n "0 2 4 7" # scale
  "minor"` and note names, and finds neither — even though the scale tables already exist
  in `midi_input.rs`. **High value, medium effort** (wire existing machinery).
- **`splice`** (speed-to-fit slicing) and **`stitch`** (boolean pattern interleave) are the
  two commonly-reached sampler/combinator ops still missing. Medium/low value, low effort.

### Dimension 2 — Synthesis / UGen coverage
**53/90 UGens** (`UGEN_STATUS.md`), Tier-1 complete. What a live coder reaches for and
does **not** find, ranked by reach:

> **⚠️ SUPERSEDED (2026-07-04, wave3-doc-status-refresh).** The **53/90** count is
> stale — the 2026-07-04 recount is **72/91** (`UGEN_STATUS.md`). The three
> bulleted gaps below (resonant filters, Allpass/Biquad, Gate/Expander/Stereo-
> Width) have since landed; see §1a's banner for per-item evidence.

- **Resonant filters** (RLPF/RHPF/Resonz/SVF) — the resonant-sweep is a live-coding staple
  and the current LPF is non-resonant. **High value, medium effort.**
- **Allpass / Biquad** — building blocks; medium value.
- **Gate / Expander / Stereo-Width** — dynamics + width; medium value, low effort.
- Heavy/Tier-2 (deprioritized): pitch-shift, freq-shift, time-stretch, vocoder, FFT/PV_*,
  convolution/plate reverb, Pan4/ambisonics, pitch-track/beat-track.

### Dimension 3 — Performance ergonomics
**Good, with two holes.**
- hush / unhush / panic — wired end-to-end (parser -> compiler -> TUI keybind).
- Multi-output `out1:`/`out2:`, `outmix:` modes.
- MIDI input + output; OSC control + live server.
- **No network tempo sync (Ableton Link)** — blocks ensemble live coding.
- **Continuous-pattern zipper (T3)** and **trigger-timing precision (T2)** are the two
  perf/timing items whose absence a performer *hears* (zipper on modulated params; onset
  drift on long sets).

### Dimension 4 — Deferred stability/perf items worth promoting
From `improvement-plan-2026-07.md` §5, with a fresh value judgment now that wave-1/2 are in:
- **Promote `T3` (per-sample continuous patterns)** — highest, because it restores the
  headline USP. -> filed.
- **Promote `T2` (widen trigger timekeeping to f64)** — cheap, composes with T3. -> filed.
- **Promote `G7` (voice preservation on swap)** — removes the amplitude notch on every
  edit; design-heavy but high perceived quality. -> filed (behind a flag, 10 ms fade
  fallback).
- **Render-owner graph swap** — the biggest architectural win (removes C1's *root* data
  race, R1/R2/R3, unifies the three live paths). Too large to implement blind -> filed as a
  **design** task.
- Deprioritized (leave deferred): `T4` Fraction rational rework, `T5` fast/slow speed
  integration, `T6` control slew, `P3` MIDI event ring (lower interactive probability),
  `P4` per-graph noise RNG.

### Dimension 5 — Documentation / examples for live performers
**Weak — and self-inflicting.** `UGEN_STATUS.md` (2025-11-13), `CLAUDE.md` "Next Priority",
`LIVECODE_COMPATIBILITY_TODO.md`, and `DSL_GAPS_DISCOVERED.md` all describe finished work as
"missing" (see §1). There is **no single, current "what actually works" reference** for a
performer. This wastes planning cycles and misleads newcomers. -> filed a doc-refresh task.

---

## 3. Ranked gaps (live-coding value / effort)

Rank = perceived live-coding value weighted against implementation effort/risk. `V`=value,
`E`=effort (both H/M/L). * = filed as a wg task this wave.

| # | Gap | V | E | Why it matters live | Sched |
|---|-----|---|---|---------------------|-------|
| 1 | **T3 — continuous patterns at sample rate** | H | M | Restores the core USP; kills zipper noise on every modulated param | * `promote-t3-continuous-patterns` |
| 2 | **Scale quantization + note names in DSL** | H | M | Melodic live coding is currently numbers-only; machinery already exists in `midi_input.rs` | * `feat-scale-quantization` |
| 3 | **Resonant filters (RLPF/RHPF/Resonz)** | H | M | Resonant sweep is a staple; current LPF is flat | * `feat-resonant-filters` |
| 4 | **T2 — f64 trigger timekeeping** | M | L | Removes onset drift/doubling on long sets; cheap; composes with #1 | * `promote-t2-trigger-f64` |
| 5 | **Chords in mini-notation** | M+ | M | `n "c'maj"` unlocks harmony; builds on #2's note-name work | * `feat-chord-support` |
| 6 | **`splice` (+ `stitch`)** | M | L | Common sampler op; low-risk pattern-layer add | * `feat-splice-stitch` |
| 7 | **Voice preservation on swap (G7)** | M+ | H | Removes amplitude notch on every `C-x`; design-heavy/risky | * `feat-voice-preservation-swap` |
| 8 | **Live-performer feature reference (doc refresh)** | M | L | Stops re-deriving finished work; onboards performers | * `doc-refresh-livecoder-reference` |
| 9 | **Render-owner graph swap (design)** | H | H | Biggest architectural win; removes C1 root + unifies live paths — needs a design pass first | * `design-render-owner-swap` |
| 10 | Gate / Expander / Stereo-Width effects | M | L | Dynamics + width | deferred -> wave-3 (§5) |
| 11 | Ableton Link / network tempo sync | M | H | Ensemble play; external crate integration | deferred -> wave-3 |
| 12 | Pitch-shift / freq-shift / time-stretch / FFT-PV | L | H | Heavy Tier-2 DSP | deferred |
| 13 | T4/T5/T6 (Fraction rational, fast-slow speed, control slew) | L | M | Polish; long-session only | deferred |

Filed this wave: **#1–#9** (9 feature/design tasks) **+ a final verify** = 10 tasks
(within the per-session subtask cap). #10–#13 are deferred with rationale in §5.

---

## 4. Filed wg tasks — DAG and golden-rule compliance

**Golden rule:** same file ⇒ sequential `--after`; disjoint files ⇒ parallel; all branches
join a final verify.

The two chokepoint files are `src/unified_graph.rs` (the `SignalNode` eval engine) and
`src/compositional_compiler.rs` (the function table + `Transform` execution). Several
feature tasks touch one or both, so they are ordered into two serial chains that merge at
the tasks touching *both* files.

```
 docs (independent roots, distinct files):
   doc-refresh-livecoder-reference ------------------------------------------------+
   design-render-owner-swap -------------------------------------------------------+
                                                                                    |
 unified_graph.rs chain:                                                            |
   promote-t3-continuous-patterns -> promote-t2-trigger-f64 --+                     |
                                                              +-> feat-resonant-    |
 compositional_compiler.rs chain:                             |    filters -> feat- |
   feat-scale-quantization -> feat-chord-support -> feat------+    voice-preserv-   |
                                            splice-stitch          ation-swap ------+
                                                                                    v
 JOIN:                                                             verify-feature-wave2
```

- `feat-resonant-filters` touches **both** chokepoint files -> `--after promote-t2-trigger-f64,feat-splice-stitch` (tip of *both* chains).
- `feat-voice-preservation-swap` touches `unified_graph.rs` (+ `voice_manager.rs`) -> `--after feat-resonant-filters` (last prior `unified_graph.rs` writer).
- `verify-feature-wave2` -> `--after` every leaf: `feat-voice-preservation-swap` (transitively covers the whole feature chain), `doc-refresh-livecoder-reference`, `design-render-owner-swap`.

### 4a. File-scope matrix (proof no two *parallel* tasks share a file)

| Task | `unified_graph.rs` | `compositional_compiler.rs` | new/other files |
|------|:--:|:--:|--|
| `promote-t3-continuous-patterns` | X | | tests |
| `promote-t2-trigger-f64` | X | | tests |
| `feat-scale-quantization` | | X | `src/scale_dsl.rs` (new), tests |
| `feat-chord-support` | | X | `src/mini_notation_v3.rs`, tests |
| `feat-splice-stitch` | | X | `src/pattern_ops_extended.rs`, tests |
| `feat-resonant-filters` | X | X | tests |
| `feat-voice-preservation-swap` | X | | `src/voice_manager.rs`, tests |
| `doc-refresh-livecoder-reference` | | | `docs/LIVE_CODING_FEATURE_REFERENCE.md` (new) |
| `design-render-owner-swap` | | | `docs/audits/design-render-owner-swap-2026-07.md` (new) |
| `verify-feature-wave2` | | | tests only |

- Every X in `unified_graph.rs` belongs to the serial chain
  `t3 -> t2 -> resonant-filters -> voice-preservation` — never two in parallel.
- Every X in `compositional_compiler.rs` belongs to the serial chain
  `scale -> chord -> splice -> resonant-filters` — never two in parallel.
- `mini_notation_v3.rs`, `pattern_ops_extended.rs`, `voice_manager.rs`, `scale_dsl.rs`
  each appear once. The two doc tasks write **distinct** files.
- **No two non-chained tasks share a file.** OK

Each code task carries a `## Validation` section requiring a **failing-test-first** flow and
the **three-level audio testing** methodology (pattern-query -> onset-detection -> audio-
characteristics) per `CLAUDE.md`. Doc/design tasks require the artifact to exist and its
claims to be code-verified.

---

## 5. Deferred (next waves) with rationale

> **⚠️ SUPERSEDED IN PART (2026-07-04, wave3-doc-status-refresh) — see
> [`wave3-scope-2026-07.md` §2](wave3-scope-2026-07.md).** The **Gate / Expander /
> Stereo-Width** bullet below is no longer deferred: Gate & Expander are DSL-wired
> + tested and Stereo-Width has a code-complete node (DSL wiring still pending —
> follow-up filed). See §1a's banner and `UGEN_STATUS.md` for evidence. The
> remaining bullets (Link, heavy Tier-2 DSP, T4/T5/T6, P3/P4, I3/I4) stand.

- **Gate / Expander / Stereo-Width** (wave-3) — low effort, medium value; deferred only to
  keep this wave at the ~10-task cap. Good first wave-3 batch (all `unified_graph.rs` +
  `compositional_compiler.rs`, so serial after wave-2's chain).
- **Ableton Link / network tempo sync** (wave-3) — medium value for ensemble play, but
  pulls an external crate and a clock-model change; wants its own design pass.
- **Heavy Tier-2 DSP** — pitch/freq shift, time-stretch, vocoder, FFT/PV_*, convolution/
  plate reverb, Pan4/ambisonics, pitch/beat-track. High effort, specialist value; deferred.
- **T4** Fraction rational rework, **T5** fast/slow speed integration, **T6** control-signal
  slew — polish; long-session-only impact; deferred (see improvement-plan §5).
- **P3** dedicate MIDI event ring, **P4** per-graph noise RNG — lower interactive
  probability; deferred.
- **Test infra I3/I4** — soak/long-run harness (validates T2/G7 accumulation) and DSL
  fuzzing; fold into wave-3 once the feature surface is bigger.

---

## 6. Validation of THIS task

- [x] `docs/audits/feature-gap-2026-07.md` exists with **ranked gaps** (§3) and an explicit
      **verify-vs-CLAUDE.md-claims reconciliation** (§1, with file:line evidence for every
      claim, including the finding that the "Next Priority" list is already complete).
- [x] wg tasks filed for the top ~10 items (§3 #1–#9 + verify) with **golden-rule-compliant
      dependencies** — two serial chains on the chokepoint files, merged at the both-files
      tasks; file-scope matrix proves no parallel task shares a file (§4a).
- [x] A **final verification task** (`verify-feature-wave2`) depends on all filed feature
      branches (§4).
- [x] **Every filed task has a `## Validation` section** requiring tests (code tasks: TDD +
      three-level audio testing; doc/design tasks: artifact-exists + code-verified claims).
