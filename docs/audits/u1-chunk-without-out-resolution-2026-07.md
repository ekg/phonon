# U1 resolution — C-x on a chunk without `out` (investigate-u1-swapping)

**Status:** Resolved (2026-07-03).
**Related:** `docs/audits/live-transition-2026-07.md` §5 (U1), `src/stress_harness.rs`
scenario `U1-chunk-without-out`.

## Finding

The `live-transition-2026-07` audit §5 predicted that live-coding C-x on a block
lacking `out` would **silence** output. The stress harness measured the opposite:
swapping from a mixed patch to `~bass $ sine 55` (no `out`) produced
`post_rms=0.6909` — a near-full-scale **blast**, not silence.

```
cargo run --release --bin glitch_stress -- --scripted | grep U1-chunk
# before fix: pre_rms=0.1800 post_rms=0.6909 post_silent=false
```

## Root cause

C-x (`ModalEditor::eval_chunk`, `src/modal_editor/mod.rs`) compiles and swaps in
**only the evaluated chunk** via `load_code` → `compile_program`
(`src/compositional_compiler.rs`). The chunk becomes a *complete replacement*
graph.

`compile_program`'s auto-routing (`src/compositional_compiler.rs`) had a
**Priority-4 "mix all buses" fallback**: when a program had no explicit `out`,
no `~master`, and no Tidal-style `dN`/`outN` bus, it summed **every** plain
`~name` bus and routed the result to the speakers at **unity gain**. So a lone
`~bass $ sine 55` became the output at full scale → RMS ≈ 0.707 (matches the
measured 0.6909). The audit's silence prediction missed this fallback.

This is worse than silence: an unexpected ~0.7 RMS blast when evaluating a
bus-only chunk (and multiple plain buses summed at unity gain is a clipping
hazard — the two-bus case measured RMS ≈ 0.797).

## Decision

Plain `~name` buses are **intermediate** named buses normally referenced by an
explicit `out $ ...` statement (which sets its own gains). But the auto-sum
fallback is a genuine live-coding convenience: a quick multi-bus sketch with no
explicit `out` still makes sound. We keep that convenience — but **bound it**.

**The Priority-4 auto-sum fallback now applies a documented headroom gain**
(`AUTO_ROUTE_HEADROOM_GAIN = 0.25`, −12 dB) instead of routing to the speakers at
unity. A lone `~bass $ sine 55` (≈ 0.707 RMS raw) now lands at ≈ 0.177 RMS —
audible but bounded, and in the U1 scenario ≈ the pre-swap level (0.18), so C-x on
a bus-only chunk is a **clean, seamless transition** instead of a blast or a
sudden dropout to silence. The status line still warns `out: NO!`; add an explicit
`out $ ~bass` to control the level precisely.

The task (`investigate-u1-swapping`) accepted *either* intentional silence *or*
"attenuated / last-bus with a documented gain". Attenuation was chosen over
silence because:

- It preserves the quick-sketch convenience — evaluate a bus-only chunk and you
  still hear it, rather than everything cutting out.
- It makes the U1 swap a *seamless* level transition (pre 0.18 → post 0.18)
  rather than a jarring full dropout mid-set.
- −12 dB is a **documented headroom constant** with a clear rationale (a "you
  forgot your output gains" safety level), not a silent surprise or an arbitrary
  magic number — the constant carries its own doc comment.

Tidal-style speaker routes (`dN`/`outN`) and explicit `out`/`~master` are
**unaffected** — they still reach the DAC at exactly the gain the user asked for.

## Changes

- `src/compositional_compiler.rs` — the Priority-4 "mix all buses" fallback now
  sums the plain `~name` buses and multiplies by `AUTO_ROUTE_HEADROOM_GAIN`
  (−12 dB) before setting output, instead of routing at unity. Priority-3
  `dN`/`outN` auto-route buses (and `~master`/`out`) keep unity gain. Extracted a
  `sum_nodes` helper shared by both paths. The `AUTO_ROUTE_HEADROOM_GAIN` constant
  carries a doc comment explaining the level.
- `src/unified_graph.rs` (`process_buffer_dag`) — `buffer.fill(0.0)` before the
  output-mix phase. The synth thread reuses one buffer across blocks, so a graph
  with **no** output node at all (e.g. C-x'ing a chunk that is only a `tempo:`
  line, or only additive numbered outputs) must actively zero it — otherwise it
  would replay the previous block instead of going silent. No-op for the common
  output-present case (the main output overwrites every sample).
- `src/stress_harness.rs` — `U1-chunk-without-out` expectation
  `Documented("U1")` → `Clean` (a regression back to the ~0.7 blast, or a
  catastrophic boundary click, is now a hard fail; unexpected silence would also
  fail `Clean`).
- `src/modal_editor/mod.rs` — fixed the stale `eval_chunk` doc comment ("We send
  the FULL session content" was false — it sends only the chunk) and documented
  the bounded auto-sum (−12 dB headroom) semantics in both `eval_chunk` and
  `load_code`.
- Tests: `tests/test_chunk_without_out.rs` (new; U1 core "bounded not blast, not
  silent" + `out`/`dN`/`~master` unity guardrails), `tests/glitch_stress_harness.rs`
  (U1 `post_rms < 0.35` and not-silent guard), `tests/test_compositional_compiler_unit.rs`
  (auto-sum still sets an output node + a `dN` auto-route guardrail).

## Scope note

The older `DslCompiler`/`parse_dsl` path (`src/unified_graph_parser.rs`) keeps its
own "sum all buses" fallback. It is **test-only** — no user-facing entry point
(`main.rs`, `modal_editor`, `live.rs`, `osc_live_server`) uses it; they all use
`compile_program`. Leaving it unchanged avoids churning many `render_dsl` test
helpers while fully fixing the live-coding/render behavior that U1 is about.

## Verification

```
cargo run --release --bin glitch_stress -- --scripted | grep U1-chunk
# after fix: pre_rms=0.1800 post_rms=0.1750 post_silent=false bnd_delta=0.010 => ok (PASS, exit 0)

cargo test --test test_chunk_without_out          # U1 core + guardrails
cargo test --test glitch_stress_harness           # scripted scenarios incl. U1 post-RMS guard
cargo test --test test_compositional_compiler_unit
```

## Possible follow-up (not done here)

A richer Tidal-style block model would *merge* the evaluated chunk into the live
graph (update only the named bus, keep the existing `out` playing) instead of
replacing the whole graph. That is a larger architectural change; the bounded
auto-sum is the safe, defined semantics until then.
