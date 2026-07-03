# Phonon Stability Improvement Plan — 2026-07

**Task:** `integrate-prioritized-stability`
**Date:** 2026-07-03
**Inputs synthesized:**

- `docs/audits/rt-safety-2026-07.md` (`audit-audio-engine`) — findings **F-1…F-11**
- `docs/audits/live-transition-2026-07.md` (`audit-live-coding`) — **D1–D4, U1, R1–R4**
- `docs/audits/pattern-timing-2026-07.md` (`audit-pattern-to`) — **F1–F9** (referred to below as **pt-F1…pt-F9**)
- `docs/audits/test-gap-analysis-2026-07.md` (`audit-test-suite`) — **P0-A…P2-B, RC-1…RC-6**
- Stress harness (`extend-glitch-harness`): `src/stress_harness.rs`, `src/bin/glitch_stress.rs`,
  `tests/glitch_stress_harness.rs` — the single-command regression driver
  `cargo run --release --bin glitch_stress -- --seed <N>`.

This document consolidates the four audits into **one deduplicated, severity-ranked
list**, explains the ranking, records the **wg execution tasks created**, and lists
**deferred items** with rationale.

Severity order (per the integration brief): **crashes > glitches/discontinuities >
timing correctness > performance > polish**.

---

## 0. What is already in flight (do not duplicate)

The `extend-glitch-harness` agent filed three defect tasks that are **already
in-progress**. They are part of this stability wave and are treated as upstream
dependencies of the `unified_graph.rs` chain (two of them edit that file):

| Task | Finding | Files | Status |
|------|---------|-------|--------|
| `fix-swap-boundary` | D3 — swap-boundary click (`prev_buffer_tail` not transferred, `disc≈0.330`) | `src/unified_graph.rs` | in-progress |
| `complete-fx-state` | D2 — pingpong/tapedelay/etc. FX tails reset on swap | `src/unified_graph.rs` | in-progress |
| `investigate-u1-swapping` | U1 — chunk without `out` jumps to ~0.7 RMS (loud), not silence | `src/modal_editor/mod.rs` | in-progress |

Because `fix-swap-boundary` and `complete-fx-state` both mutate `src/unified_graph.rs`,
**every new `unified_graph.rs` task in this plan is chained `--after` all three** to
avoid a multi-way conflict on that file (see §3, the golden rule).

---

## 1. Consolidated & Deduplicated Findings (master list)

Overlapping findings across the four audits are merged into a single row and cross-referenced.
`★` marks a finding scheduled in this wave; `▷` marks a deferred finding (§5).

### Tier 1 — CRASH (can panic/kill the audio thread → permanent silence)

| ID | Finding (merged sources) | Primary location | Files | Sched |
|----|--------------------------|------------------|-------|-------|
| **C1** | **Cross-thread `RefCell` borrow race → synth-thread panic.** Reload transfer holds `try_borrow_mut` on the old graph while the synth thread does **unconditional `borrow_mut()`** on the same `ArcSwap`-shared cell → panic → ring drains → audio stops permanently. *Also* the underlying data race on `RefCell`'s non-atomic borrow flag (`unsafe impl Sync` is unsound). Sources: rt **F-1**, test-gap **RC-1/G-1/P0-A**, live-transition **R1/R2** (partial). | `src/main.rs:1006`, `src/bin/phonon-audio.rs:288` (synth); `:1107`/`:531` (reload) | `main.rs`, `bin/phonon-audio.rs` | ★ `fix-synth-borrow-race` (symptom); ▷ render-owner model (root) |
| **C2** | **VST3 plugin load + per-sample `String` alloc + `lock().unwrap()` in render.** Loads/initialises a VST3 from disk on the render thread on first note; allocates a `Vec<(String,f32)>` every sample; `.unwrap()` on a poisoned lock panics. Source: rt **F-2**. | `src/unified_graph.rs:12122-12208`, `13788-13882` | `unified_graph.rs` | ★ `harden-render-locks` |
| **C3** | **Render-thread `Mutex.lock().unwrap()` poison → panic / priority inversion.** Sample&Hold stores per-node scalar state as `Mutex<f32>` locked **4× per sample**; GlobalClock + fundsp state also `.unwrap()`-locked on the hot path. Source: rt **F-3**. | `src/unified_graph.rs:14023-14033`, `13788+`; `src/bin/phonon-audio.rs:269` | `unified_graph.rs` (+ clock) | ★ `harden-render-locks` |
| **C4** | `src/live.rs` `LiveSession` latent **panicking** reload path (`borrow_mut`) + file-I/O in the CPAL error handler. Unreachable from the CLI today → latent. Source: rt **F-10**. | `src/live.rs:108,284,288,157-174` | `live.rs` | ▷ deferred (latent) — partly touched by `unify-live-clock` |

### Tier 2 — GLITCH / DISCONTINUITY (audible dropout/click/retrigger)

| ID | Finding (merged sources) | Primary location | Files | Sched |
|----|--------------------------|------------------|-------|-------|
| **G1** | Swap-boundary click — `prev_buffer_tail` not transferred → Phase-4d crossfade skipped. live-transition **D3**. | `src/unified_graph.rs:7728-7764`, `5652` | `unified_graph.rs` | ✅ `fix-swap-boundary` (in flight) |
| **G2** | Partial FX-state transfer — 8 effect types counted-not-injected → tails snap to zero. live-transition **D2**. | `src/unified_graph.rs:8176-8210` | `unified_graph.rs` | ✅ `complete-fx-state` (in flight) |
| **G3** | C-x on a chunk without `out` → loud ~0.7-RMS jump (harness) / silence (audit). live-transition **U1**. | `src/modal_editor/mod.rs:2154,2112` | `modal_editor.rs` | ✅ `investigate-u1-swapping` (in flight) |
| **G4** | **Voice-pool heap growth + `eprintln!` on the synth thread** under dense triggers → alloc spike + stderr backpressure → underrun. rt **F-4**. | `src/voice_manager.rs:905-955,1044` | `voice_manager.rs` | ★ `preallocate-voice-pool` |
| **G5** | **NaN/Inf in internal node state → stuck silence** (output guard zeroes the sample but the node keeps emitting NaN). *And* the harness measures **post-sanitisation** output, so its NaN/clip gates are tautological. rt **F-6** + test-gap **RC-2/G-2/G-7/P0-C**. | `src/unified_graph.rs:7706-7726` (guard); stateful nodes | `unified_graph.rs`, harness, tests | ★ `sanitize-node-state` (folds the P0-C pre-sanitisation probe) |
| **G6** | **`Signal::Pattern` re-parses mini-notation every sample** (44.1 k parses/s/pattern) → allocation on the synth thread → CPU spike → underrun/timing glitch. pt-**F6**. | `src/unified_graph.rs:10181-10193` | `unified_graph.rs`, `compositional_compiler.rs` | ★ `compile-time-pattern-parse` |
| **G7** | Every swap **fades and kills active voices** (10 ms release, synth voices detached) → amplitude notch on every C-x + truncated long samples. live-transition **D1**. | `src/voice_manager.rs:2367-2388`, `src/unified_graph.rs:5629-5635` | `voice_manager.rs`, `unified_graph.rs` | ▷ deferred (design: needs voice→node identity + stealing policy) |
| **G8** | Modal I16 callback can `resize()` inside the callback (device buffer > 4096 frames). rt **F-9** / prior R4. | `src/modal_editor/mod.rs:434-435` | `modal_editor.rs` | ▷ deferred (rare, small) |
| **G9** | Stale-audio latency — ring not cleared on C-x → up to ~100 ms of old code still audible. live-transition **D4**. | `src/modal_editor/mod.rs:765-768` | `modal_editor.rs` | ▷ deferred (trade-off; couples with D3 fix) |

### Tier 3 — TIMING CORRECTNESS

| ID | Finding (merged sources) | Primary location | Files | Sched |
|----|--------------------------|------------------|-------|-------|
| **T1** | **Live clock re-anchored to wall-clock every buffer** (startup/post-underrun onset clustering + steady-state jitter) **and `set_cps()` teleports cycle position** in wall-clock mode (no offset compensation). pt-**F1** + pt-**F2**. The correct `GlobalClock` model already exists in `phonon-audio.rs`. | `src/unified_graph.rs:18001,5060`; `src/live.rs:108`; `src/main.rs:1006` | `unified_graph.rs`, `live.rs`, `main.rs` | ★ `unify-live-clock` |
| **T2** | **`last_trigger_time` stored as `f32` absolute cycle position** → precision cliff (~4 ms @ 10 h) → onset jitter / doubled / dropped triggers; also weakens swap dedup. pt-**F3**. | `src/unified_graph.rs:1009,7275,18036` | `unified_graph.rs` | ▷ deferred (wave 2 — cheap; composes with T1) |
| **T3** | Continuous LFO patterns frozen to their buffer-start value (~86 Hz stairstep) → zipper noise; contradicts the sample-rate-modulation headline. pt-**F5**. | `src/unified_graph.rs:17806,10078` | `unified_graph.rs` | ▷ deferred |
| **T4** | `Fraction` is float-backed (fixed 1e6 denominator) → `1/3 ≠ 1/3`, `i64` overflow at extreme lengths. pt-**F4**. | `src/pattern.rs:27` | `pattern.rs` | ▷ deferred |
| **T5** | `fast`/`slow` sample the speed pattern once per cycle, in `f64`. pt-**F7**. | `src/pattern.rs:696` | `pattern.rs` | ▷ deferred |
| **T6** | Stepped control values: no slew, string-parsed per lookup. pt-**F8**. | `src/unified_graph.rs:10124` | `unified_graph.rs` | ▷ deferred |

### Tier 4 — PERFORMANCE / DEGRADATION (erodes headroom / leaks over a session)

| ID | Finding (merged sources) | Primary location | Files | Sched |
|----|--------------------------|------------------|-------|-------|
| **P1** | **Per-buffer allocation + `env::var("DEBUG_*")` in `process_buffer_dag`** (`HashMap::new()`/`vec!`/`.collect()` + a glibc global-lock env scan every buffer) → jitter → "underruns when the patch gets big"; plus swap-time `eprintln!`. rt **F-5** + pt-**F9**. | `src/unified_graph.rs:7248,7294,5670` | `unified_graph.rs` | ▷ **wave-2** `dag-scratch-arena` (deferred by the depth-8 limit — see §5) |
| **P2** | **`Box::leak` per parse** (two leaks per `parse_program`) → unbounded resident-memory growth over a live session. rt **F-7**. | `src/compositional_parser.rs:586,632` | `compositional_parser.rs` | ★ `parser-arena-no-leak` |
| **P3** | MIDI-monitoring `Mutex<VecDeque>` drained on the render thread → contention / priority inversion. rt **F-8**. | `src/unified_graph.rs:10809,14951,15099` | `unified_graph.rs` | ▷ deferred |
| **P4** | Noise nodes call `rand::thread_rng()` per sample (TLS lookup on the hot path). rt **F-11**. | `src/unified_graph.rs:10664,10740,10772` | `unified_graph.rs` | ▷ deferred |

### Tier 5 — POLISH / DOC / RESIDUAL RACES

| ID | Finding | Sched |
|----|---------|-------|
| **X1** | Voiceless-old-graph window during `preload_samples` (take/store gap). live-transition **R3**. | ▷ deferred |
| **X2** | Rapid successive C-x → cumulative fades / skip windows (no crash). live-transition **R4**. | ▷ deferred |
| **X3** | Stale/wrong doc comments: `eval_chunk:2112` ("FULL session content"), `load_code:722-724` ("preserve active voices"), CLAUDE.md "instant C-x transitions". | ▷ deferred (fold into `investigate-u1-swapping` / docs) |

### Cross-cutting — TEST INFRASTRUCTURE (enablers)

| ID | Finding | Sched |
|----|---------|-------|
| **I1** | Pre-sanitisation invariant probe — assert on **raw** (pre-limiter/pre-flush) signal so NaN/clip gates stop being tautological. test-gap **P0-C**. | ★ folded into `sanitize-node-state` |
| **I2** | Concurrency loom + TSan/Miri stress (proves/refutes the `unsafe impl Sync` unsoundness, C1 root). test-gap **P0-A**. | ▷ deferred (pairs with render-owner model) |
| **I3** | Soak / long-run harness (leaks, voice/phase drift). test-gap **P1-A**. | ▷ deferred (validates P2/G4/T2 accumulation) |
| **I4** | DSL fuzzing (generative + libFuzzer). test-gap **P1-B**. | ▷ deferred |
| **I5** | Live-path unification conformance suite. test-gap **P2-A**. | ▷ deferred (lands with render-owner model) |
| **I6** | Real-time deadline / callback-budget harness. test-gap **P0-B**. | ✅ largely EXISTS in `src/stress_harness.rs` (concurrent session + budget/underrun detectors) |

---

## 2. Ranking rationale

1. **Crashes first, and the reachable ones before the latent ones.** **C1** is ranked #1
   across three of the four audits: it is triggered by the *core interactive action*
   (editing while playing) on the two most-used surfaces (`phonon live`,
   `phonon-audio`), the outcome is a hard permanent stop, and it is a *fresh partial
   regression* introduced by the recent reload-continuity fixes. The symptom fix
   (`try_borrow_mut`+skip, already proven in the modal editor) is cheap and independent
   of `unified_graph.rs`, so it leads the wave as its own branch. **C2/C3** are the
   next crashes and both live in `unified_graph.rs`, so they head the serial chain.
   **C4** is latent (unreachable `LiveSession`) → deferred.

2. **Glitches by reach × audibility.** The three in-flight tasks (**G1/G2/G3**) already
   cover the most-audible swap discontinuities. Among the rest, **G4** (voice-pool growth
   on the synth thread) fires on ordinary dense drum patterns; **G5** (stuck-silence from
   internal NaN) is both a real dropout class *and* the finding that most exposes the
   test blind spot (the harness measures sanitised output), so its fix is bundled with
   the **I1** pre-sanitisation probe that makes it — and future NaN work — actually
   testable. **G6** (per-sample mini-notation re-parse) is a glitch *and* an RT-safety
   allocation, so it sits in the glitch tier. **G7** (fade-every-swap) is audible on
   every C-x but requires real design work (voice→node identity + a stealing policy to
   bound accumulation) and is risky, so it is deferred rather than rushed.

3. **Timing correctness after glitches.** **T1** is the marquee timing fix and the
   pattern-timing audit's #1 recommendation: unify every live path on the sample-advancing
   `GlobalClock` that already exists, which removes the per-buffer wall-clock re-anchor
   (jitter/clustering) *and* the `set_cps` teleport in one move. **T2** (widen `f32`→`f64`)
   is cheap and composes with T1, but only bites after ~1 h, so it is the top wave-2 item.

4. **Performance/leaks last in the wave.** **P2** (`Box::leak`) is a clean single-file leak
   fix on an independent branch and is scheduled. **P1** (per-buffer alloc/`getenv`) is the most
   common "it underruns when the patch gets big" cause, but it is the lowest-severity of the
   `unified_graph.rs` chain and the depth-8 task limit admits only four `unified_graph.rs` fix
   tasks before the wave-1 verify gate — so P1 is deferred to **wave 2** (pre-seeded, serial
   after the chain). P3/P4 are lower-probability and deferred.

5. **The `unified_graph.rs` chokepoint dictates structure.** ~10 findings touch
   `src/unified_graph.rs`. The golden rule (same file ⇒ sequential) forces them into one
   serial chain; independent-file work (C1, G4, P2) fans out in parallel. Because the chain
   already starts deep (after the three in-flight tasks) and the graph enforces a max
   user-visible depth of 8, the wave-1 chain is capped at **four** `unified_graph.rs` fix tasks
   (C2/C3 → G5 → G6 → T1) so the verify join lands at depth 8; the next-severity item (P1) leads
   wave 2. A deeper win — splitting `unified_graph.rs` and moving to a render-thread-owned graph
   — is the biggest deferred item because it simultaneously removes C1's root data race (I2),
   R1/R2/R3, and unifies the three live paths (I5).

---

## 3. Execution structure (the golden rule)

**Same file ⇒ sequential `--after`. Independent files ⇒ parallel. All branches join at a
final verify task.**

```
 in-flight (unified_graph.rs):  fix-swap-boundary ─┐
                                complete-fx-state ─┤ (both edit unified_graph.rs)
                                investigate-u1-swapping ─┤ (modal_editor.rs; chained for safety)
                                                   │
 unified_graph.rs SERIAL CHAIN: ───────────────────┴─► harden-render-locks (C2,C3 CRASH)
                                                          └─► sanitize-node-state (G5+I1 GLITCH)
                                                                └─► compile-time-pattern-parse (G6 GLITCH/RT)
                                                                      └─► unify-live-clock (T1 TIMING) ─┬─► verify-stability-wave1
                                                                                                       │      (JOIN)
                                                                                                       └─► dag-scratch-arena (P1) [WAVE-2, serial, not gated by verify]
                                                                                                              │
 PARALLEL BRANCH (main.rs, phonon-audio.rs): fix-synth-borrow-race (C1 CRASH) ─────────────────────────────────┤
   └─ note: unify-live-clock also edits main.rs, so it is chained --after fix-synth-borrow-race                 │
 PARALLEL BRANCH (voice_manager.rs):         preallocate-voice-pool (G4 GLITCH) ───────────────────────────────┤
 PARALLEL BRANCH (compositional_parser.rs):  parser-arena-no-leak (P2 leak) ──────────────────────────────────►┘
                                                                                                              ▼
 JOIN:                                        verify-stability-wave1  (full cargo test + glitch_stress seeded)
```

The wave-1 `unified_graph.rs` chain is **four** tasks deep (C2/C3 → G5 → G6 → T1). `dag-scratch-arena`
(P1, wave-2) is pre-seeded serial `--after unify-live-clock` (golden-rule safe on `unified_graph.rs`)
but is **not** part of the wave-1 verify gate — it gets a wave-2 verify of its own.

**File-scope matrix (proof that no two *parallel* tasks share a file):**

| Task | `unified_graph.rs` | `main.rs` | `bin/phonon-audio.rs` | `live.rs` | `voice_manager.rs` | `compositional_parser.rs` | `compositional_compiler.rs` | `modal_editor.rs` | harness/tests |
|------|:--:|:--:|:--:|:--:|:--:|:--:|:--:|:--:|:--:|
| `fix-synth-borrow-race` | | ✎ | ✎ | | | | | | ✎ |
| `preallocate-voice-pool` | | | | | ✎ | | | | ✎ |
| `parser-arena-no-leak` | | | | | | ✎ | | | ✎ |
| `harden-render-locks` | ✎ | | | | | | | | ✎ |
| `sanitize-node-state` | ✎ | | | | | | | | ✎ |
| `compile-time-pattern-parse` | ✎ | | | | | | ✎ | | ✎ |
| `unify-live-clock` | ✎ | ✎ | | ✎ | | | | | ✎ |
| `dag-scratch-arena` *(wave-2)* | ✎ | | | | | | | | ✎ |

Every `✎` in the `unified_graph.rs` column belongs to the **serial chain** (never two in
parallel). `main.rs` appears in `fix-synth-borrow-race` and `unify-live-clock` — the latter
is chained `--after` the former. `bin/phonon-audio.rs` is exclusive to
`fix-synth-borrow-race`. `compositional_compiler.rs` is exclusive to
`compile-time-pattern-parse`. `voice_manager.rs`, `compositional_parser.rs`, `live.rs` each
appear once. **No two non-chained tasks share a file.** ✔

Each fix task carries a `## Validation` section that **requires a regression test**, routed
through the stress harness (`glitch_stress` / `stress_harness.rs`) wherever the defect is
interactive.

---

## 4. Created wg tasks (this wave)

Initial wave = **7 fix tasks + 1 verify** (top-impact, per the ~8–10 cap; the depth-8 limit
capped the `unified_graph.rs` chain at four, moving P1 to wave 2). IDs below are the exact `wg`
task IDs.

| # | Task ID | Tier / finding | Files | `--after` |
|---|---------|----------------|-------|-----------|
| 1 | `fix-synth-borrow-race` | CRASH · C1 (rt F-1) | `main.rs`, `bin/phonon-audio.rs` | *(root)* |
| 2 | `harden-render-locks` | CRASH · C2+C3 (rt F-2,F-3) | `unified_graph.rs` | `fix-swap-boundary, complete-fx-state, investigate-u1-swapping` |
| 3 | `preallocate-voice-pool` | GLITCH · G4 (rt F-4) | `voice_manager.rs` | *(root)* |
| 4 | `sanitize-node-state` | GLITCH+enabler · G5+I1 (rt F-6, test-gap P0-C) | `unified_graph.rs`, harness, tests | `harden-render-locks` |
| 5 | `compile-time-pattern-parse` | GLITCH/RT · G6 (pt-F6) | `unified_graph.rs`, `compositional_compiler.rs` | `sanitize-node-state` |
| 6 | `unify-live-clock` | TIMING · T1 (pt-F1,F2) | `unified_graph.rs`, `main.rs`, `live.rs` | `compile-time-pattern-parse, fix-synth-borrow-race` |
| 7 | `parser-arena-no-leak` | PERF/leak · P2 (rt F-7) | `compositional_parser.rs` | *(root)* |
| 8 | `verify-stability-wave1` | VERIFY (join) | tests only | `unify-live-clock, preallocate-voice-pool, parser-arena-no-leak, fix-synth-borrow-race, fix-swap-boundary, complete-fx-state, investigate-u1-swapping` |
| — | `dag-scratch-arena` *(wave-2, pre-seeded)* | PERF/glitch · P1 (rt F-5, pt-F9) | `unified_graph.rs` | `unify-live-clock` |

The verify task (#8) depends transitively on the entire chain via `unify-live-clock`, plus the
parallel leaves and the three in-flight defect tasks, so it runs only once **every** wave-1
stability fix has landed. `dag-scratch-arena` is created now (serial after the chain, golden-rule
safe) but is the lead **wave-2** task and is gated by a future `verify-stability-wave2`, not by
#8.

---

## 5. Deferred items (next waves) with rationale

Deferred deliberately — either lower interactive probability, design-heavy/risky, or
best sequenced after the wave-1 fixes land.

**Wave 2 (next, high value, mostly cheap):**

- **P1 `dag-scratch-arena`** (rt F-5, pt-F9) — **already created and pre-seeded** in the graph,
  chained `--after unify-live-clock` (serial on `unified_graph.rs`, golden-rule safe). Deferred
  from wave 1 only because the depth-8 limit caps the pre-verify `unified_graph.rs` chain at four
  tasks; it is the highest-priority wave-2 item and needs a `verify-stability-wave2` gate.
- **T2 `widen-trigger-timing-f64`** (pt-F3) — cheap `f32`→`f64` widening of trigger
  bookkeeping; composes with `unify-live-clock`; do immediately after it lands. Long-session
  only, hence not wave 1.
- **P3 `dedicate-midi-event-ring`** (rt F-8) — replace the render-thread `Mutex<VecDeque>`
  with a lock-free SPSC ring. `unified_graph.rs` → must chain after wave-1's chain.
- **T3 `per-sample-continuous-patterns`** (pt-F5) — evaluate signal patterns per sample to
  restore the sample-rate-modulation headline; `unified_graph.rs` → chain after wave 1.

**Design-heavy / architectural (biggest wins, highest risk — do with a dedicated design task):**

- **Render-owner graph swap** (live-transition Rank 5) — move `transfer_*` + pointer swap
  onto the synth thread at a buffer boundary. **Removes C1's root data race** (not just the
  panic symptom), R1 (beat jump), R2 (starvation), R3 (voiceless window), and unifies the
  three live paths. Pair with **I2** (loom/TSan/Miri, test-gap P0-A) and **I5** (live-path
  conformance suite, test-gap P2-A) to prove it. Touches the threading model of every
  frontend — schedule as its own design→implement→verify sub-graph after wave 1.
- **G7 `voice-preservation-on-swap`** (D1) — policy-driven voice continuation needs stable
  voice→node identity across compiles and a stealing policy to bound accumulation; ship
  behind a flag with the 10 ms fade as fallback.
- **C4 `retire-or-harden-livesession`** (rt F-10) — delete `LiveSession`/`LiveRepl` or bring
  them to the `try_borrow_mut`+skip pattern and move the error-log write off the callback.
  Latent (unreachable) so low urgency; `unify-live-clock` will already touch `live.rs`.

**Lower-probability polish:**

- **T4** Fraction rational rework (pt-F4), **T5** `fast/slow` speed integration (pt-F7),
  **T6** control-signal slew + compile-time numeric parse (pt-F8), **P4** per-graph noise RNG
  (rt F-11), **G8** modal I16 resize-in-callback (rt F-9), **G9** clear-and-crossfade ring on
  C-x (D4), **X1** voiceless-old-graph window (R3), **X2** rapid-C-x choppiness (R4).

**Test infrastructure (beyond I1, which ships in wave 1):**

- **I3** soak/long-run harness (P1-A) — validates P2/G4/T2 accumulation; some scaffolding
  already exists in `stress_harness.rs`.
- **I4** DSL fuzzing (P1-B) — generative + `cargo-fuzz` on `parse_program`.
- **I6/P2-B** real-device callback golden test — `#[ignore]`d nightly; advisory.

---

## 6. Validation of this integration task

- [x] `docs/audits/improvement-plan-2026-07.md` exists with ranked findings (§1),
      rationale (§2), and the list of created task IDs (§4).
- [x] A wg task exists for every critical/high finding not already in flight
      (C1→`fix-synth-borrow-race`, C2/C3→`harden-render-locks`, G4→`preallocate-voice-pool`,
      G5→`sanitize-node-state`, G6→`compile-time-pattern-parse`, T1→`unify-live-clock`,
      P2→`parser-arena-no-leak`; P1→`dag-scratch-arena` pre-seeded as wave-2 due to the depth
      limit); C4 latent + design-heavy items deferred with rationale (§5).
- [x] No two parallel (non-chained) tasks modify the same file — file-scope matrix, §3.
- [x] Every created fix task has a `## Validation` section requiring a regression test,
      routed through the stress harness where the defect is interactive.
- [x] A final `verify-stability-wave1` task depends on all fix tasks and requires: full test
      suite green + `glitch_stress` seeded run clean.
