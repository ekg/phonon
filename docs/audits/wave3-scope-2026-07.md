# Phonon Wave-3 Scope — 2026-07

**Task:** `scope-wave-3-remaining`
**Date:** 2026-07-04
**HEAD scoped against:** `d1d8a75` (== `main`; includes all wave-1, wave-2, and the 11 pre-existing-triage merges)

**Question:** Waves 1 (stability) and 2 (features) are verified complete. From the *full*
evidence now available — the reconciled red surface, the deferred roadmap in the three audit
docs, and the **actual current code** — what is left for wave 3, and what is deliberately out?

**Method:** Every "is it done / is it missing" claim below was checked against the **source
tree and the live test binaries at HEAD**, not against the status docs (which lag the code by
months — see §4). Commands are cited inline so the reconciliation is reproducible.

> **Headline:** waves 1+2 cleared far more than they were chartered to. The feature surface a
> live coder reaches for is now essentially complete (see §2 "already done — verified"). Wave 3
> is therefore **not another feature wave** — it is a **hardening + one-missing-capability +
> doc-accuracy** wave: endurance/soak testing, DSL fuzzing, the single genuinely-absent
> performer feature (network tempo sync), one cheap RT-safety hot-path polish, and a doc
> refresh to stop re-deriving finished work.

---

## 1. Red-surface reconciliation (every remaining red binary dispositioned)

The `full-test-suite` baseline (main HEAD `ed35cbc`) was **8490 pass / 212 fail / 101 ignored**
in debug (**81 failing binaries**; 78 reproducible + 3 load-flaky), dominated by *pattern-query
passes but Level-2/3 audio asserts fail* — a systemic e2e render/sample-trigger cluster, not 81
independent bugs. It was grouped into **11 triage tasks**. Their disposition, verified from
`wg show` logs + eval status:

| Triage task | Binaries claimed | Outcome | Nature of fixes |
|---|---:|---|---|
| `fix-pre-existing` | 1 (`test_lpf_square_lfo`) | done, eval 0.88 | real FFI/threading bug |
| `fix-pre-existing-2` | 1 (`test_filters_envelopes_utils`, 11 tests) | done | env applies to synth/osc signals |
| `fix-pre-existing-3` | 1 (`test_synthesis_complete`, 5 tests) | done | legacy `synth "notes" "wave"` arg-binding |
| `fix-pre-existing-4` | ~2 (`test_mix`,`test_stack_operation`) | done | **real bug**: `Mix` averaged instead of summed |
| `fix-pre-existing-5` | 3 (`test_sample_integration`,`_n_modifier`,`_note_modifier`) | done | sample onset trigger; filed `fix-parallel-render` (done) |
| `fix-pre-existing-6` | 1 (`test_vst3_plugins`, the crashed binary) | done | X-display gate for headless VST3 SIGSEGV |
| `triage-fix-pre` | 17 | done, eval 0.90 | pattern-transform audio characteristics |
| `triage-fix-pre-2` | 18 | done | 3 real src bugs (begin/end, sample-param, multi-out cache) + stale-test migration |
| `triage-fix-pre-3` | 7 | done | scale/note-pitch/MIDI-poly (test-only + doc; features already correct) |
| `triage-fix-pre-4` | 16 | done | osc/filter/noise/effects (3 real: pan2 cache, `_trig` numeric, TransientShaper) |
| `triage-fix-pre-5` | 17 | done | routing/multi-out/hush/stereo/voice/DSL; filed `fix-parse-dsl` (done) |

**Arithmetic:** 17+18+7+16+17 = **75** triage binaries + **~9** `fix-pre-existing` binaries ≈ the
**81** red binaries. Both follow-ups spawned during triage — `fix-parallel-render` (a genuine
parallel-render voice-truncation bug) and `fix-parse-dsl` (`parse_dsl` silently dropped
statements after `struct "pat" $ src`) — are **done**.

**Live re-verification (do not trust stale docs):** the four binaries the triage agents explicitly
flagged as *"left pre-existing / out of scope"* were run at HEAD `d1d8a75`, single-threaded:

```
test_stut_transform                 21 passed / 0 failed   (was flagged 4-fail)
test_sample_trigger_timing          24 passed / 0 failed   (was flagged 2-fail)
test_sample_pattern_operations       7 passed / 0 failed   (was flagged 6-fail)
test_multi_output_hush_integration  35 passed / 0 failed   (was flagged test_auto_routing_when_no_out)
```

All green — the later, broader triage waves swept up the residuals the earlier ones left behind.

**Disposition of the red surface: CLEARED.**
- Every one of the 81 binaries is accounted for by a **done + eval-passed** triage task.
- No open triage/follow-up tasks remain (`wg list` shows only this scoping task + its FLIP open).
- The residual reds observed *under heavy multi-agent load* are **environmental** (sample-loading
  contention via the deprecated paren `s(...)` syntax, plus the 3 wall-clock load-flaky tests
  `test_tempo_doubling_bug` / `test_no_infinite_loops_on_swap` / `test_graph_swap_performance`
  which pass at `--test-threads=1`). These are tracked in memory
  (`stabilize-load-flaky`, `stress-budget-overrun-env-fragile`) and are **not code defects** — no
  wave-3 task is filed to "fix" them; the correct remediation (run affected wall-clock tests
  serialized) is already documented. **No red binary is being declared-obsolete-and-deleted**; all
  were repaired, not removed.

**Wave-3 filing on the red surface: NONE.** It is done.

---

## 2. What is already done — verified against the code (OUT of wave 3)

The single most important scoping finding: **almost every item on the wave-3 candidate list in
the task brief is already implemented.** Checked at HEAD:

| Candidate dimension (task brief) | Actual state (verified) | Evidence |
|---|---|---|
| Multi-output `out1:`/`out2:`, `hush`/`panic` | DONE | `triage-fix-pre-5`; `test_multi_output_hush_integration` 35/0 |
| UGens: white noise, pulse/PWM, Pan2/stereo, limiter, parametric EQ, FM | DONE | `feature-gap-2026-07.md §1`; `UGEN_STATUS.md` |
| **Gate / Expander / Stereo-Width** *(feature-gap listed these MISSING)* | **DONE — stale doc** | `compile_gate` `compositional_compiler.rs:3102`, `compile_expander:3033`, `stereo_width` node `unified_graph.rs:1548`; tests `test_noise_gate.rs`, `test_expander_buffer.rs`, `test_effects_characteristics.rs`; landed in commit `f5a7bce` "Wave 10 production essentials" |
| Pattern DSP params (gain/pan/speed), hurry/chop/striate/loopAt | DONE | `feature-gap-2026-07.md §1` |
| Scale quantization, note names, chords, splice, stitch, resonant filters | DONE (wave 2) | `verify-feature-wave2` |
| T3 continuous patterns, T2 f64 triggers, G7 voice-preservation | DONE (wave 2) | `verify-feature-wave2` |
| DAW-style block-buffer passing | DONE | `process_buffer_dag` + `dag-scratch-arena` (wave 2) |
| Render-owner graph swap (C1 root race) | DONE | `src/render_swap.rs` (28 KB); `verify-render-owner-swap` done; `UnifiedSignalGraph` now `Send`-only |
| I2 loom / TSan / Miri concurrency verification | DONE | `tests/loom_graph_swap.rs`; `Cargo.toml:115` `cfg(loom)` dev-dep |
| I5 live-path conformance suite | DONE | `tests/live_path_conformance.rs` |
| Performer feature reference doc | DONE (wave 2) | `docs/LIVE_CODING_FEATURE_REFERENCE.md` |
| Wave-1 accumulation fixes (P2 `Box::leak`, G4 voice-pool) | DONE | `parser-arena-no-leak`, `preallocate-voice-pool` both done; `compositional_parser.rs:592` "no `Box::leak`" |

There is **no remaining Tidal pattern op or Tier-1 UGen** a live coder commonly reaches for that
is absent. The frontier has moved from "add features" to "keep the big surface robust over long
sessions and under adversarial input."

---

## 3. Wave-3 scope — ranked by live-coding value vs effort

`V` = perceived live-coding value, `E` = effort/risk (H/M/L). ★ = filed this wave. ▷ = deferred (§5).

| # | Item | V | E | Why it matters now | Sched |
|---|------|---|---|--------------------|-------|
| 1 | **DSL fuzzing (I4)** — generative + `cargo-fuzz` on **both** front-ends (`parse_dsl`, `parse_program`) | H | M | The parser is the front door and has **two divergent front-ends**; `fix-parse-dsl` *just* found a silent statement-drop that a fuzzer catches trivially. A live coder mistypes constantly — a panic or silent-drop mid-set is a show-stopper | ★ `wave3-dsl-fuzzing` |
| 2 | **Soak / endurance harness (I3)** — multi-hour simulated live session | H | M | The feature surface is now large; waves 1+2 fixed a whole class of *accumulation* bugs (P2 leak, G4 pool growth, T2 f32 trigger drift). Nothing currently *guards* those over a real multi-hour set. Regression net + validates the noise-RNG fix (#4) | ★ `wave3-soak-harness` |
| 3 | **Network tempo sync (Ableton Link)** — design pass | M+ | H | The **only** genuinely-missing performer capability. Unlocks ensemble live coding. External crate + clock-model change → design-first, then a scoped implement sub-graph (mirrors the render-owner precedent) | ★ `wave3-design-ableton-link` |
| 4 | **Noise-RNG hot-path polish (P4)** — per-node seeded PRNG | L+ | L | `rand::thread_rng()` is called **per sample** in `WhiteNoise`/pink/etc. (`unified_graph.rs:12446` + 4 more) → TLS lookup + reseed check on the audio hot path → jitter on noise-heavy patches. Cheap swap to a per-node `SmallRng` seeded once | ★ `wave3-noise-rng-hotpath` |
| 5 | **Doc-accuracy refresh** — `UGEN_STATUS.md` + stale audit claims + showcase examples | M | L | `UGEN_STATUS.md` says **53/90** (dated 2025-11-13) — an undercount; resonant/gate/expander/width/transient-shaper all landed. `feature-gap-2026-07.md §1a/§5` lists shipped effects as MISSING. Stale docs make every future planner re-derive finished work (this very scoping task had to) | ★ `wave3-doc-status-refresh` |
| 6 | **Verify gate** | — | — | Full suite green + soak short-run clean + fuzz smoke clean + P4 regression + docs match code | ★ `verify-wave3` |
| — | P3 MIDI-event ring (render-thread `Mutex<VecDeque>`) | L | M | Lower interactive probability; only bites with heavy MIDI monitoring | ▷ |
| — | Heavy Tier-2 DSP (pitch/freq-shift, time-stretch, vocoder, FFT/PV, convolution/plate reverb, Pan4/ambisonics) | L | H | Specialist value, high effort | ▷ |
| — | T4 Fraction-rational, T5 fast/slow speed, T6 control slew | L | M | Long-session-only polish | ▷ |
| — | G8 modal resize-in-callback, G9 clear-crossfade ring, X1 voiceless window, X2 rapid-C-x, C4 retire `LiveSession` | L | L–M | Rare / latent / trade-off polish | ▷ |

**Filed this wave: #1–#6** (4 work tasks + 1 dependent + 1 verify = 6 tasks, within the session
cap). Deferred items carry rationale in §5.

---

## 4. Filed wg tasks — DAG and golden-rule compliance

**Golden rule:** same file ⇒ sequential `--after`; disjoint files ⇒ parallel; all branches join a
final verify. Wave 3 is unusual: it edits **almost no shared source** — it is mostly *new* test
harnesses, a design doc, and one isolated code fix. That makes the DAG wide and shallow.

```
 roots (implicit --after scope-wave-3-remaining):
   wave3-noise-rng-hotpath (P4, unified_graph.rs) ──┐
                                                     └──► wave3-soak-harness (new files) ──┐
   wave3-dsl-fuzzing      (new files + Cargo dev-dep) ───────────────────────────────────┤
   wave3-design-ableton-link (new design doc) ──────────────────────────────────────────┤
   wave3-doc-status-refresh  (docs + new examples) ─────────────────────────────────────┤
                                                                                          ▼
 JOIN:                                                                              verify-wave3
```

- `wave3-soak-harness` is chained `--after wave3-noise-rng-hotpath` for a **logical** reason
  (its noise-heavy long-run scenario doubles as the P4 regression assertion), **not** a file
  conflict — they share no file, so it is golden-rule-safe either way.
- `verify-wave3` depends on the four leaves (`wave3-soak-harness` transitively covers
  `wave3-noise-rng-hotpath`).

### 4a. File-scope matrix (proof no two parallel tasks share a file)

| Task | `src/unified_graph.rs` | `Cargo.toml` | new / other files |
|------|:--:|:--:|--|
| `wave3-noise-rng-hotpath` | ✎ | | tests only |
| `wave3-soak-harness` | | | `src/bin/soak_endurance.rs`, `tests/soak_endurance.rs` (new) |
| `wave3-dsl-fuzzing` | | ✎ (add `proptest` dev-dep) | `tests/dsl_fuzz.rs`, `fuzz/` (new) |
| `wave3-design-ableton-link` | | | `docs/audits/design-ableton-link-2026-07.md` (new) |
| `wave3-doc-status-refresh` | | | `docs/UGEN_STATUS.md`, `examples/wave3_showcase.ph` (new) + adds a "stale/superseded" banner note pointing here |
| `verify-wave3` | | | tests only |

- `unified_graph.rs` is touched by **exactly one** task (`wave3-noise-rng-hotpath`). ✔
- `Cargo.toml` is touched by **exactly one** task (`wave3-dsl-fuzzing`, adding the `proptest`
  dev-dep; the soak bin is auto-discovered from `src/bin/` and needs **no** manifest edit). ✔
- Soak, fuzz, design, and doc-refresh each write **disjoint** new files. ✔
- **The parser source is edited by NO wave-3 task.** `wave3-dsl-fuzzing` only *exercises*
  `parse_dsl` / `parse_program`; if it discovers a bug, its `## Validation` requires the **fix**
  to be filed as a follow-up task that chains **sequentially** on the parser file (golden rule),
  never edited in parallel.

Every code/infra task carries a `## Validation` section requiring **failing-test-first** and, for
anything that renders audio, the **three-level methodology** (pattern-query → onset-detection →
audio-characteristics) per `CLAUDE.md`. Doc/design tasks require the artifact to exist with
code-verified claims.

---

## 5. Deferred (next waves) with rationale

- **P3 — dedicate MIDI-event ring** (rt F-8): the render thread drains a `Mutex<VecDeque>` for
  MIDI monitoring. Real RT-safety item but **lower interactive probability** (only under heavy
  MIDI-in monitoring); would chain sequentially after P4 on `unified_graph.rs`. Deferred to keep
  wave 3 value-dense.
- **Heavy Tier-2 DSP** — pitch/freq-shift, time-stretch, vocoder, FFT/PV_*, convolution/plate
  reverb, Pan4/ambisonics, pitch/beat-track. High effort, specialist value.
- **T4** Fraction rational rework, **T5** `fast/slow` speed integration, **T6** control-signal
  slew: polish; measurable only on very long sets.
- **G8/G9/X1/X2** modal-editor polish, **C4** retire/harden latent `LiveSession`: rare, latent, or
  trade-off-laden; low urgency.
- **Missing input doc:** `docs/audits/test-gap-analysis-2026-07.md` is referenced by
  `improvement-plan-2026-07.md §1` and by this task's brief but **does not exist on disk** (it was
  never committed, or was removed). Its findings survive transitively via `improvement-plan §5`
  (I2–I6) and `feature-gap §5`, all of which are reconciled here — I2 and I5 are **done**, I3/I4
  are **filed this wave**, I6 largely **exists** (`stress_harness.rs`). No information was lost;
  the missing file is noted for the record, not re-created.

---

## 6. Validation of THIS task

- [x] `docs/audits/wave3-scope-2026-07.md` exists with **ranked scope** (§3) and **explicit
      in/out decisions** (§2 out, §3 in, §5 deferred).
- [x] **Triage results reconciled** (§1): all 81 red binaries mapped to 11 done+eval-passed
      triage tasks; 4 flagged residuals **live-re-verified green** at HEAD; environmental/load-flaky
      reds identified as non-code and left to their documented remediation; **no bulk
      declare-obsolete-and-delete** was needed (everything was repaired). Every remaining red is
      dispositioned.
- [x] wg tasks filed with **golden-rule-compliant dependencies** — one code task per chokepoint
      file, disjoint new-file harnesses in parallel, file-scope matrix proving no parallel task
      shares a file (§4a), and a `## Validation` (TDD + three-level audio) on each.
- [x] A **`verify-wave3` gate task** depends on all filed wave-3 tasks (§4).
