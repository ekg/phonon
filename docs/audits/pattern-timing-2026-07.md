# Audit: Pattern-to-Audio Timing Correctness

**Task:** `audit-pattern-to`
**Date:** 2026-07-03
**Scope:** Investigation only — the boundary between the pattern system
(`src/pattern.rs`, `src/mini_notation_v3.rs`, `src/pattern_ops*.rs`) and the
audio engine (`src/unified_graph.rs`, `src/live.rs`, `src/main.rs`,
`src/bin/phonon-audio.rs`). No production code was changed.

---

## 1. Executive Summary

The pattern→audio timing path is **functional but has several structural
timing hazards**, three of which can produce audible artifacts today and one
of which is a latent footgun for live tempo control:

| # | Severity | Finding | Primary location |
|---|----------|---------|------------------|
| F1 | **HIGH** | Live clock is re-anchored to wall-clock **at render time** every buffer, not advanced by samples emitted. Causes startup clustering, post-underrun clustering, and per-buffer onset jitter. | `unified_graph.rs:18001`, `live.rs:108`, `main.rs:1006` |
| F2 | **HIGH** | `set_cps()` changes tempo with **no offset compensation**; in wall-clock mode this teleports the cycle position by `elapsed·Δcps`. | `unified_graph.rs:5060` |
| F3 | **HIGH (long session)** | Trigger bookkeeping (`last_trigger_time`, sample-node timing) stored as **`f32` absolute cycle position**; loses sub-cycle resolution after ~1 h of play. | `unified_graph.rs:1009`, `:7275`, `:18036` |
| F4 | **MEDIUM** | `Fraction` is **float-backed** (`from_float` with fixed 1e6 denominator). The rational type is decorative; every op round-trips through `f64`. Integer overflow at extreme session lengths. | `pattern.rs:27` |
| F5 | **MEDIUM** | Continuous "signal" patterns (sine/saw/tri LFO) are **frozen to their buffer-start value** for the whole 512-sample buffer by the event cache. | `unified_graph.rs:17806`, `:10078` |
| F6 | **MEDIUM** | `Signal::Pattern` **re-parses the mini-notation string every sample** (44 100 parses/s/pattern) → CPU spike → underrun → timing glitch. | `unified_graph.rs:10181` |
| F7 | LOW–MED | `fast`/`slow` sample the speed pattern **once per cycle** and do all time math in `f64`. | `pattern.rs:692` |
| F8 | LOW–MED | Stepped pattern control values have **no slew/smoothing** and are re-parsed from strings per lookup. | `unified_graph.rs:10124` |
| F9 | LOW | `eprintln!` in the graph-swap hot path and `std::env::var` lookups inside per-sample loops. | `unified_graph.rs:5670`, `:7908`, `:10103` |

**The good news:** because live timing is slaved to a wall-clock, the beat
does **not** exhibit unbounded free-running drift, and graph swaps preserve
cycle position coherently (see §13). The risks below are about *jitter*,
*discontinuities*, and *precision*, not slow tempo drift.

**The core recommendation:** adopt the `GlobalClock` model that already exists
in `src/bin/phonon-audio.rs:98` for *all* live paths. It advances by
sample-count between buffers and only rebases on wall-clock at explicit tempo
changes — exactly the fix for F1 and F2. `LiveSession` (`live.rs`) and the
`main.rs` live loop still call the self-timing `process_buffer()` instead of
`process_buffer_at()`, and this is the root of most findings here.

---

## 2. End-to-End Path: Pattern Query → Event Scheduling → Audio

This is the full trace for the default (DAG) live render path.

```
                        +------------------------------------------+
  wall clock ---------->| buffer_start_cycle = elapsed*cps+offset  |  (F1: render-time anchor)
  (Instant::elapsed)    |  unified_graph.rs:18001-18006            |
                        +--------------------+---------------------+
                                             | buffer_start_cycle, sample_increment
                                             v
                        process_buffer_internal (18014)
                                             |
                                             v
                        process_buffer_dag (7240)
                          |  self.cached_cycle_position = buffer_start_cycle   (7257)
                          |  precompute_pattern_events(buffer_size)            (7305)
                          |      +- query each Pattern/Sample node ONCE over
                          |         [start_cycle, end_cycle]  (17806-17815)
                          |         -> pattern_event_cache : Vec<Hap<String>>  (17867)
                          v
             for i in 0..buffer_size:                              (7860)
                 cycle_pos = buffer_start_cycle + i*sample_increment  (7861)   <- correct spacing WITHIN a buffer
                 self.cached_cycle_position = cycle_pos
                 eval_node(output)                                 (7900)
                     +- Pattern node: filter cached events by
                        cycle_pos in [part.begin, part.end)        (10078-10088)
                     +- Sample node: trigger voice if new onset,
                        dedup via last_trigger_time (f32)          (F3)
                 output[i] = sample
             cached_cycle_position = buffer_start_cycle + N*incr   (7962)  <- sample-accurate, but DISCARDED next buffer
                                             |
                                             v
             ring buffer (1 s in live.rs, 2 s in phonon-audio.rs)  <- render-ahead latency
                                             |
                                             v
             cpal audio callback: copy-only (live.rs:181)          <- plays ~1-2 s after render
```

**Query mechanics.** A `Pattern<T>` is a closure `Fn(&State) -> Vec<Hap<T>>`
(`pattern.rs:171`). `State.span` is a `TimeSpan { begin, end: Fraction }`
(`pattern.rs:106`). The audio engine queries with a one-sample-wide window
`[cycle_pos, cycle_pos + 1/sr/cps)` (`unified_graph.rs:10091`) or, on the fast
path, queries the whole buffer once and filters per sample from the cache.

**Two clocks that disagree.** Within a buffer, time advances by sample count
(`cycle_pos = buffer_start_cycle + i*increment`, line 7861 — correct). Between
buffers, `buffer_start_cycle` is recomputed from wall-clock (line 18002). The
sample-accurate "next start" is computed at line 7962 but **thrown away**
because the next `process_buffer()` call re-derives the anchor from wall-clock.
This split is the source of F1.

---

## 3. Finding F1 — Live clock re-anchored to wall-clock per buffer (HIGH)

**Where:** `src/unified_graph.rs:17999-18010` (`process_buffer`), invoked by
`src/live.rs:108` and `src/main.rs:1006`.

```rust
// unified_graph.rs:18001
let buffer_start_cycle = if self.use_wall_clock {
    let elapsed = self.session_start_time.elapsed().as_secs_f64();
    elapsed * self.cps as f64 + self.cycle_offset      // <- time NOW, at render
} else {
    self.cached_cycle_position
};
```

The background synthesis thread (`live.rs:93-152`) renders buffers **as fast as
the ring buffer has space** and queues up to 1 s of audio ahead
(`ring_buffer_size = sample_rate`, `live.rs:69`; 2 s in `phonon-audio.rs:244`).
`buffer_start_cycle` is the cycle position at the instant the buffer is
*rendered*, but that audio is *played* up to a full ring-buffer later.

**Why it bites:**

- **Startup clustering (audible).** The ring starts empty and is filled as fast
  as the CPU allows. Rendering ~1 s of audio (~86 buffers of 512) may take only
  a few tens of ms of wall time, but `buffer_start_cycle` only advances by that
  few tens of ms * cps. So the first second of output is rendered from a tiny,
  heavily-overlapping band of cycle positions -> repeated/stuttered onsets at
  startup until the pipeline reaches steady state.
- **Post-underrun / CPU-spike clustering.** Any time the ring drains low and the
  synth "catches up" by bursting many buffers, the same clustering recurs -> a
  timing glitch precisely when the system is already stressed.
- **Steady-state jitter.** Even when full, each buffer's anchor is a fresh
  `Instant::elapsed()` read, so onset timing carries the scheduling jitter of
  the cpal callback cadence (several ms) rather than being locked to the sample
  grid.
- **No long-term drift, though.** Because the anchor is wall-clock, average
  tempo is correct and cannot free-run — this is the one property the design
  buys, at the cost of the jitter above.

**Suggested fix:** Advance the clock by **samples emitted**, not wall-clock, and
consult wall-clock only to *rebase* (startup, explicit resync, post-underrun).
The sample-accurate next-start already exists at `unified_graph.rs:7962` — feed
it back in instead of discarding it. Concretely, route **all** live rendering
through `process_buffer_at()` (`unified_graph.rs:17982`) driven by the
`GlobalClock` in `phonon-audio.rs:98`, whose `get_buffer_timing()`
(`:155`) is the single source of truth. `LiveSession::start` (`live.rs`) and the
`main.rs:1006` loop should be migrated off `process_buffer()`; the doc comment
at `unified_graph.rs:17997` already says *"use process_buffer_at() for live
rendering."* If a wall-clock feel is still wanted for underrun resilience,
implement a PLL that nudges the sample clock toward wall-clock slowly (a few
ppm/buffer) instead of hard-reanchoring every buffer.

---

## 4. Finding F2 — `set_cps()` teleports cycle position in wall-clock mode (HIGH, latent)

**Where:** `src/unified_graph.rs:5060-5062`.

```rust
pub fn set_cps(&mut self, cps: f32) {
    self.cps = cps;                 // <- no offset compensation
}
```

In wall-clock mode the position is `elapsed*cps + offset` (`:8299`). Changing
`cps` without adjusting `offset` produces an **instantaneous jump of
`elapsed*(cps_new - cps_old)`**. `elapsed` grows unbounded across a session, so a
tempo tweak an hour in (`elapsed = 3600 s`) from cps 2.0->2.5 jumps the position
by `3600*0.5 = 1800 cycles` — the beat teleports.

**Contrast — the correct implementation already exists:**
`GlobalClock::set_cps` (`src/bin/phonon-audio.rs:135-144`) rebases first:

```rust
let current_pos = self.get_position();
self.base_cycle_position = current_pos;   // save position
self.base_time = Instant::now();          // reset elapsed origin
self.cps = new_cps;                        // then change tempo
```

**Current blast radius.** The live *reload* path avoids the bug by accident:
compile builds a fresh graph (`elapsed ~ 0`), `set_cps` runs there, then
`transfer_session_timing` (`:5652`) recomputes `cycle_offset` from the new cps
(§13). So today `set_cps` is only safe because a later step overwrites the
offset. Any *direct* live tempo change on a running wall-clock graph — a future
OSC `cps` message, MIDI-clock follow, or a `tempo:`/`bpm:` hot-edit that does not
go through a full graph swap — will trigger the jump. Callers to audit:
`compositional_compiler.rs:404,761,781`, `unified_graph_parser.rs:1822`,
`osc_live_server.rs:198,209`.

**Suggested fix:** Make `UnifiedSignalGraph::set_cps` rebase exactly like
`GlobalClock::set_cps`: when `use_wall_clock`, capture
`let p = elapsed*cps_old + offset; session_start_time = Instant::now();
cycle_offset = p; cps = new;`. Better: delete graph-owned timing and let
`GlobalClock` own cps for all paths (removes the two-sources-of-truth split
that F1 and F2 share).

---

## 5. Finding F3 — Absolute cycle position stored in `f32` (HIGH for long sessions)

**Where:** field decls `src/unified_graph.rs:1009` (`last_trigger_time: f32`),
`:1024` (`last_cycle: i32`), and the casts that populate them,
`:7275` and `:18036`:

```rust
*last_trigger_time = buffer_start_cycle as f32 - 0.001;   // f64 -> f32 at large magnitude
```

`last_trigger_time` holds an **absolute** cycle position (it grows with the
session), but it is an `f32` with a 24-bit mantissa (~7 significant digits).

**Quantified precision loss.** ULP(f32) at magnitude `x` ~ `x * 2^-23`:

| Session (cps = 2) | Cycle position | f32 ULP (cycles) | f32 ULP (ms) | Samples @ 44.1 kHz |
|---|---|---|---|---|
| 1 min | 120 | 1.4e-5 | 0.007 | 0.3 |
| 1 hour | 7 200 | 8.6e-4 | 0.43 | 19 |
| 10 hours | 72 000 | 8.6e-3 | 4.3 | 190 |
| 24 hours | 172 800 | 2.1e-2 | 10.3 | 455 |

Onset dedup compares an `f64` `cycle_pos` against this `f32` `last_trigger_time`.
Once the f32 grid (right column) exceeds the inter-onset spacing or the
one-sample query width, the comparison becomes unreliable -> **onset jitter,
missed triggers, or double triggers** that worsen monotonically over a long set.
By ~10 h the trigger time cannot even be resolved to finer than ~190 samples.

**Suggested fix:** Store trigger/timing bookkeeping as `f64` (cheap, matches the
rest of the path which is already `f64`). Better still, store timing **relative
to the current cycle** (fractional `[0,1)` phase + integer cycle) so magnitude
never grows, keeping precision constant regardless of session length. This also
composes with the F4 fix.

---

## 6. Finding F4 — `Fraction` is float-backed; rational time is decorative (MEDIUM)

**Where:** `src/pattern.rs:27-32`.

```rust
pub fn from_float(f: f64) -> Self {
    // Simple conversion - could be improved
    let denominator = 1000000;
    let numerator = (f * denominator as f64) as i64;
    Self::new(numerator, denominator)
}
```

The system *presents* rational time (`Fraction` with `Add/Sub/Mul/Div`,
`pattern.rs:47-89`) but constructs fractions from floats with a **fixed 1e6
denominator**, and nearly every transform round-trips through `f64`. Examples:

- `fast` (`pattern.rs:716-735`): `Fraction::from_float(begin.to_float()*factor)`
  — the scale is done in `f64`, then re-wrapped.
- `mini_notation_v3.rs:1379,1392,1396,1401` and pattern-cat at `:1363-1404`:
  positions computed as `f64` (`cycle_f + i/n`) then `from_float`.

Consequences:

1. **1/3 is not 1/3.** `from_float(1.0/3.0) = 333333/1000000`. Triplets and any
   `n`-not-dividing-1e6 subdivision are off by up to 5e-7 cycle. Negligible
   acoustically (~0.25 us @ cps 2) but it means the "exact rational" guarantee
   does not hold, and errors are *fixed-grid*, not exact.
2. **Integer overflow at extreme lengths.** `Mul` does
   `self.numerator * other.numerator` (`pattern.rs:73`); `cmp`/`duration`/
   `midpoint` cross-multiply (`:99`, `:118`, `:126`). With the 1e6 denominator, a
   cycle position of `C` yields numerator ~ `C*1e6`. `cmp`'s single product
   `num*den ~ C*1e6*1e6 = C*1e12` overflows `i64` (9.2e18) at `C ~ 9.2e6`
   cycles ~ **~1 300 h @ cps 2**; `Mul` of two large fractions overflows far
   sooner (`(C*1e6)^2 > i64::MAX` at `C ~ 3 000` cycles ~ **~25 min**) — though
   in practice the hot path uses the `f64` round-trip and rarely calls
   `Fraction::mul` on absolute positions, so this is latent, not routinely hit.
   In release builds the overflow wraps silently to garbage time.

**Long-session drift verdict (quantified):** The dominant long-session risk is
**F3 (f32 trigger times, ~4 ms error at 10 h)**, not F4. `f64` `cycle_pos`
itself is fine (ULP at 172 800 cycles ~ 4e-11 cycle). The 1e6 fixed grid adds a
constant <=5e-7-cycle quantization that does **not** accumulate. Fraction integer
overflow is only reachable at pathological lengths or via `Fraction::mul` on
absolute positions.

**Suggested fix:** Either (a) make `from_float` a proper rational approximation
(continued fractions / `f64->ratio`) and keep numerators/denominators reduced and
bounded, or (b) drop the rational pretense and standardize on `f64` phase with
per-cycle-relative representation (§5 fix), which removes both the quantization
and the overflow. Add `checked_mul`/`i128` intermediates to `Fraction` ops if
the rational path is retained.

---

## 7. Finding F5 — Continuous LFO patterns frozen per buffer (MEDIUM)

**Where:** `src/unified_graph.rs:17806-17815` (whole-buffer query) and
`:10078-10088` (per-sample cache filter).

`precompute_pattern_events` queries each pattern **once** over the whole buffer
span `[start_cycle, end_cycle]`. Continuous "signal" patterns —
`Pattern::sine_wave/saw_wave/tri_wave` (`pattern.rs:511,547,562`) — return a
**single hap spanning the query**, with the value computed at
`state.span.begin` only (`pattern.rs:516`: `phase = span.begin.to_float() % 1.0`).
The per-sample filter then matches that one hap for **every** sample in the
buffer and returns the same value:

```rust
// unified_graph.rs:10080
cached_events.iter().filter(|event| {
    cycle_pos >= event.part.begin.to_float() && cycle_pos < event.part.end.to_float()
})   // one buffer-wide hap => constant value for all 512 samples
```

So a pattern-as-LFO used as a control (routed through `SignalNode::Pattern`)
updates only once per 512-sample buffer — a **~86 Hz stairstep** at 44.1 kHz.
Audibly: zipper noise on filter cutoffs, quantized vibrato. This contradicts the
project's headline feature ("patterns evaluated at sample rate", CLAUDE.md).
Note: native UGen LFOs (`sine 2` compiled to an oscillator node) are unaffected;
this hits patterns that reach `SignalNode::Pattern`.

**Suggested fix:** Detect continuous/analog patterns and evaluate them
**per-sample** (the fallback at `:10091` already does the right thing — query
`[cycle_pos, cycle_pos+width)`), bypassing the whole-buffer cache. Or have
signal patterns emit a hap whose value is a function of query time rather than a
constant, and interpolate in the cache lookup. Cache should hold *discrete*
step events only.

---

## 8. Finding F6 — `Signal::Pattern` re-parses mini-notation every sample (MEDIUM–HIGH)

**Where:** `src/unified_graph.rs:10181-10193`.

```rust
Signal::Pattern(pattern_str) => {
    let pattern = parse_mini_notation(pattern_str);   // <- per-sample parse + alloc
    let sample_width = 1.0 / self.sample_rate as f64 / self.cps as f64;
    let state = State { span: TimeSpan::new(
        Fraction::from_float(cycle_pos),
        Fraction::from_float(cycle_pos + sample_width)), controls: HashMap::new() };
    let events = pattern.query(&state);
```

For any inline `Signal::Pattern`, the mini-notation string is **parsed from
scratch on every sample** — 44 100 parse+allocate cycles per second per such
signal. This is a real-time-safety violation (allocation in the synth thread)
and a large CPU cost that directly increases underrun probability, which in turn
causes the timing glitches described in F1. `Signal::Bus`/`SignalNode::Pattern`
avoid this via the event cache; the inline branch does not.

**Suggested fix:** Parse once at graph-compile time and store the
`Pattern<String>` (as `SignalNode::Pattern` already does), or memoize
`parse_mini_notation` results keyed by the string. No parsing should occur in
`eval_signal_at_time`.

---

## 9. Finding F7 — `fast`/`slow` sample speed once per cycle, in float (LOW–MED)

**Where:** `src/pattern.rs:696-712`.

`fast` queries its `speed` pattern only at `cycle_start = floor(span.begin)`
(`:698`) and applies a single scalar factor for the whole query. A
pattern-controlled `fast "2 4"` therefore cannot change speed sub-cycle, and the
time scaling is done in `f64` (`:716`) then re-wrapped as `Fraction` — i.e. this
op cannot deliver the "modulate any time parameter at sample rate" promise for
the time axis. `slow` inherits this via `self.fast(inverted)` (`:824`).

**Suggested fix:** For truly pattern-modulated time, integrate the speed pattern
over the query span (piecewise) rather than sampling one value at the cycle
boundary. Lower priority — most uses pass a constant.

---

## 10. Finding F8 — Stepped control values: no slew, string-parsed per lookup (LOW–MED)

**Where:** `src/unified_graph.rs:10124-10137`.

Pattern control values are stored as **strings** (`Hap<String>`) and parsed on
each lookup (`s.parse::<f32>()`, else `note_to_midi`). Two issues: (a) no
smoothing/slew is applied to stepped patterns, so audio-rate modulation of e.g.
filter cutoff clicks at each step boundary; (b) string parsing per lookup is
avoidable work in the render path.

**Suggested fix:** Parse control patterns to `Pattern<f64>` at compile time; add
an optional one-pole slew (e.g. `# slew 0.005`) for parameters modulated at
audio rate. Ties into F5.

---

## 11. Finding F9 — Hot-path `eprintln!` and per-sample `env::var` (LOW)

**Where:** `src/unified_graph.rs:5670-5682` (`transfer_session_timing` prints on
every graph swap); `std::env::var("DEBUG_*")` calls inside per-sample loops at
`:7908`, `:7953`, `:10103`, `:11912`, etc.

The swap-time `eprintln!` block writes to stderr on every live reload (jitter +
I/O during a swap). `std::env::var` performs a lock + lookup; calling it inside
the sample loop (even guarded) adds avoidable overhead in the real-time thread.

**Suggested fix:** Gate swap logging behind a debug flag; hoist `env::var`
lookups to `bool` fields read once at graph construction.

---

## 12. Tempo-Change Behavior (explicit assessment)

- **cps change via full reload:** SAFE. Fresh graph compiled -> `set_cps` on it ->
  `transfer_session_timing` (`:5652`) recomputes `cycle_offset` so position is
  continuous across the new cps (`:5667`: `new_offset = old_cycle_pos -
  old_elapsed*new_cps`). No beat drop.
- **cps change via direct `set_cps` on a live graph:** BROKEN (F2) — teleports
  position by `elapsed*Δcps`.
- **`setCycle` / `set_cycle`** (`:8238`): SAFE — adjusts `cycle_offset` to hit
  the target at current wall-clock, and updates node timing to avoid
  re-triggering (`:8320`).
- **`nudge`** (`:8253`): SAFE — shifts both `cycle_offset` and cached position by
  the same amount; monotonic, no jump.
- **`resetCycles`** (`:8225`): SAFE — resets origin and offset together.
- **Latency caveat (all of the above):** because output is buffered 1-2 s ahead
  (§2/§3), a tempo/cycle command takes audible effect ~one ring-buffer *later*,
  and the already-rendered audio in the ring plays at the old setting first.
  Cycle *position* stays coherent; the *audible* change is delayed by ring
  latency.

---

## 13. Graph-Swap Interaction (explicit assessment)

- **Cycle position survives a swap coherently.** `transfer_session_timing`
  (`:5652`) copies `session_start_time` and rebases `cycle_offset`, so the new
  graph continues from the old graph's position even across a cps change. No beat
  drop by construction.
- **Node timing is carried to prevent re-trigger/double-trigger.**
  `set_cycle_position` (`:8308`) and the sample-node init in
  `process_buffer_internal` (`:18025-18041`) seed `last_trigger_time` just below
  `buffer_start_cycle` so already-past events in the current cycle do not
  re-fire. This dedup is only as precise as F3 allows — after ~1 h the f32
  `last_trigger_time` may fail to distinguish "already triggered" from "new,"
  risking a doubled or dropped onset at the first buffer after a swap.
- **Overlap risk couples with F1.** Because live buffers are anchored to
  wall-clock (F1), consecutive buffers are not guaranteed to tile
  `[start, end)` contiguously; the *only* guard against an event being rendered
  in two overlapping buffers is the f32 `last_trigger_time` dedup (F3). Fixing F1
  (sample-accurate contiguous buffers) removes the overlap and makes the dedup
  robust.
- **Swap effect is delayed by ring latency**, same caveat as §12.

---

## 14. Prioritized Recommendations

1. **F1 + F2 together — unify on `GlobalClock`.** Route every live path through
   `process_buffer_at()` driven by the `phonon-audio.rs` `GlobalClock`; advance
   by sample count, rebase on wall-clock only at tempo changes/resync. This
   removes the render-time anchor, the two-clocks split, and the `set_cps`
   teleport in one move. *(Highest impact; the machinery already exists.)*
2. **F3 — widen timing state to `f64` and/or store cycle-relative.** Cheap,
   removes the long-session precision cliff and hardens swap dedup.
3. **F6 — never parse mini-notation in the render loop.** Parse at compile time;
   removes a real-time allocation and a major underrun contributor.
4. **F5 — evaluate continuous patterns per sample.** Restores the sample-rate
   modulation the project advertises.
5. **F4 — fix or retire `Fraction::from_float`.** Prefer standardizing on `f64`
   phase (composes with #2); otherwise use a real rational approximation +
   `i128`/`checked_mul` guards.
6. **F7/F8/F9 — polish:** integrate speed patterns over the span, add optional
   control-signal slew + compile-time numeric parsing, and remove hot-path
   `eprintln!`/`env::var`.

## 15. Suggested Follow-up Tasks (for the graph)

- **Implement F1/F2 fix:** migrate `LiveSession` + `main.rs` live loop to
  `process_buffer_at` + a shared `GlobalClock`; delete graph-owned live timing.
  *(code task — touches `live.rs`, `main.rs`, `unified_graph.rs` clock methods)*
- **Implement F3 fix:** change `last_trigger_time`/sample-node timing to `f64`
  (or cycle-relative). *(code task — `unified_graph.rs` only)*
- **Implement F6 fix:** compile-time parse for `Signal::Pattern`.
  *(code task — `unified_graph.rs`, parser)*
- **Regression harness:** add a long-session (simulated multi-hour) onset-timing
  test and a tempo-change onset test to quantify F1/F2/F3 before and after — see
  sibling task `extend-glitch-harness`.

---

*Investigation only — no source files were modified. All line references are
against the repository state at task start (branch `main`, commit `302cc3f`).*
