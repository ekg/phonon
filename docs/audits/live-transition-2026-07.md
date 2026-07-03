# Audit: Live-Coding Transition Stability

**Date:** 2026-07-03
**Task:** `audit-live-coding`
**Scope:** Investigation only — no behavioral code changes. Focus is the interactive
hot-swap path in the modal editor (`phonon edit`): what happens when the user presses
**C-x** to re-evaluate a block while audio is playing.

This audit **verifies current behavior** against source as of commit `302cc3f`
(`extend-glitch-harness`). Several fixes have landed since the two prior 2026-05-24
research docs (`docs/research/realtime-reload-audit.md`,
`docs/research/audio-stabilization-roadmap.md`,
`docs/research/audio-glitch-reproduction.md`); line numbers and conclusions below reflect
the **current tree**, not those docs. Section 7 reconciles what each prior fix solved and
what remains.

---

## 1. Executive Summary

The modal editor hot-swap is a **lock-free `ArcSwap` graph-pointer swap** feeding a
**single-producer/single-consumer ring buffer**. Compared to the 2026-05-24 audits it is
materially safer: the panic-prone `borrow_mut()` race is gone (both the synth thread and
the reload path now use `try_borrow_mut()`), underrun counters are atomic, a global
NaN/Inf/denormal guard sanitizes output before the ring, and an intra-buffer crossfade
smooths block edges.

The swap is **not glitch-free**, and the residual failures are exactly the ones the test
suite cannot see: the harness (`tests/audio_live_edit_glitch_harness.rs`) performs a
**direct in-process ownership transfer** with **no real synth thread, no `RefCell`
contention, no ring buffer, and no CPAL stream** (confirmed at
`tests/audio_live_edit_glitch_harness.rs:778-794`, `748-776`). Every race below lives in
the machinery the harness stubs out.

Confirmed residual discontinuities/races (detailed in §5, §6):

| # | Failure | Trigger | Symptom |
|---|---------|---------|---------|
| D1 | Voices faded on every swap | Any C-x while a sample/synth voice sounds | 10 ms fade-out + retrigger; truncated drum/pad |
| D2 | Partial FX-state transfer | C-x with TapeDelay/PingPong/Dattorro/MoogLadder/etc. in the patch | Effect tail resets to zero → click / lost tail |
| D3 | Cross-swap crossfade never fires | Every C-x | `prev_buffer_tail` not transferred → boundary click (disc ≤ 0.33) |
| D4 | Stale-audio latency | Every C-x | Ring not cleared → up to ~100 ms of old code still audible |
| R1 | Transfer failure → beat jump | C-x while render > ~25 ms (heavy voice load) | `transfer_session_timing` skipped → timing restarts, beat jumps |
| R2 | Synth-thread starvation | C-x while CPU-saturated | `try_borrow_mut` skips drain the ring → underrun / dropout |
| R3 | Voiceless-old-graph window | Every C-x | Old graph rendered with emptied VoiceManager during preload window |
| R4 | Rapid successive C-x | Mashing C-x | Cumulative fades + skip windows → choppiness (serialized, no crash) |
| U1 | Chunk replaces whole graph | C-x on a block lacking `out` | Entire output silenced; stale doc comment misleads |

No crash path was found in the modal editor swap: the old `borrow_mut()` panic
(prior-audit B7) does **not** exist here — see §7.

---

## 2. Architecture of the Hot-Swap Path

Five entry surfaces share the `UnifiedSignalGraph` engine; this audit targets the
**modal editor** (`phonon edit`, `src/modal_editor/mod.rs`), the primary interactive
surface. The others (`src/main.rs` file-watch live, `src/live.rs` `LiveSession`,
`src/bin/phonon-audio.rs` two-process server) are cross-referenced only where behavior
diverges.

```
 UI thread (crossterm event loop)              Synth thread (spawned)         CPAL callback
 ─────────────────────────────────             ──────────────────────         ─────────────
 C-x → eval_chunk() → load_code()
   parse + compile new_graph
   try_borrow_mut(OLD) ── transfer ──►  (contends on same RefCell)
   preload_samples(new)
   graph.store(Some(new))  ────────►    graph.load() snapshot
                                        try_borrow_mut(cur).process_buffer
                                        ring_producer.push_slice ──────────►  ring_consumer.pop_slice → device
```

Key shared state (`src/modal_editor/mod.rs`):

- `graph: Arc<ArcSwap<Option<GraphCell>>>` (`:76`, init `:211`) where
  `GraphCell(RefCell<UnifiedSignalGraph>)`. Lock-free pointer swap; `RefCell` guards the
  single mutable graph instance so UI and synth threads never mutate concurrently.
- `should_clear_ring: Arc<AtomicBool>` (`:106`, init `:222`) — one-shot flag the CPAL
  callback reads to drain the ring. **Set only by hush/panic**, never by C-x.
- Ring: `HeapRb::<f32>::new(sample_rate/5)` (`:227-228`). At 44.1 kHz that is 8820 f32
  slots. Output is **interleaved stereo**, so effective depth ≈ **4410 frames ≈ 100 ms**
  (the `~200ms` comment at `:225` counts samples, not frames, on a stereo device).
- Synth render chunk: `synthesis_buffer_size = buffer_size.unwrap_or(512)` (`:164`) =
  512 interleaved samples = 256 frames ≈ **5.8 ms/render**.

---

## 3. The Swap Protocol, Step by Step

All line references are `src/modal_editor/mod.rs` unless noted.

**Trigger.** `KeyCode::Char('x') + CONTROL → eval_chunk()` (`:939-943`).

**`eval_chunk()` (`:2114-2179`)**
1. `get_current_chunk()` — grabs the current blank-line-delimited paragraph (`:2115`).
2. If the chunk text starts with `hush`, call `hush()` first (`:2144-2150`).
3. `self.load_code(&chunk)` — **passes only the chunk**, not the whole buffer
   (`:2154`). (The doc comment at `:2112` claiming "we send the FULL session content" is
   **stale and wrong** — see U1.)
4. Flash-highlight the evaluated lines on success (`:2177`).

**`load_code()` (`:629-773`) — the swap itself**
1. `parse_program(code)` → statements; reject partial parses (`:633-642`).
2. `compile_program(...)` → `new_graph` (`:655-659`). CPS is taken from the code's
   `tempo:`/`bpm:` (`:647`).
3. Snapshot `has_old_graph = matches!(**self.graph.load(), Some(_))` (`:667`).
4. `new_graph.enable_wall_clock_timing()` **unconditionally** (`:675`) — guarantees a valid
   `session_start_time` even if transfer later fails.
5. If an old graph exists (`:689`), **retry loop, up to 50 × 500 µs = 25 ms**
   (`:695-738`):
   - `old_graph_cell.0.try_borrow_mut()` (`:696`). On `Err` (synth thread holds the
     borrow), sleep 500 µs and retry (`:732-736`).
   - On success, **while holding the borrow**:
     - `new_graph.transfer_session_timing(&old_graph)` (`:710`) — carries wall-clock
       reference (`session_start_time`, `cycle_offset`) so the beat does not drop
       (`src/unified_graph.rs:5652-5674`). CPS is **not** transferred (new code wins).
     - `new_graph.transfer_fx_states(&old_graph)` (`:720`) — copies stateful-effect
       buffers (`src/unified_graph.rs:7967-8222`).
     - `new_graph.transfer_voice_manager(old_graph.take_voice_manager())` (`:724`) —
       `take_voice_manager` swaps the old graph's `VoiceManager` for a fresh empty one and
       returns the live one (`src/unified_graph.rs:5621-5625`); `transfer_voice_manager`
       installs it into `new_graph` **after releasing all its voices**
       (`src/unified_graph.rs:5629-5635`).
     - `state_transferred = true; break`.
   - If the borrow never succeeds within 25 ms: log "Could not transfer state", proceed
     with the new graph's **fresh** timing (`:740-753`) → **beat jump** (R1).
6. `new_graph.preload_samples()` (`:758`) — **after** the borrow is dropped
   (`src/unified_graph.rs:5555-5601`); loads any sample not already in the bank cache.
7. `self.graph.store(Arc::new(Some(GraphCell(RefCell::new(new_graph)))))` (`:762-763`) —
   atomic pointer swap. The synth thread picks up the new graph on its next iteration.
8. **Ring is deliberately not cleared** (`:765-768`) → D4.

**Synth thread (`:235-350`)** — per iteration:
- `graph_clone_synth.load()` snapshot (`:273`).
- `Some` → `graph_cell.0.try_borrow_mut()` (`:280`):
  - `Ok` → `graph.process_buffer(&mut buffer)` → `ring_producer.push_slice` (`:283-306`).
  - `Err` (UI thread holds the borrow mid-transfer) → **skip; write nothing** (`:314-329`).
    Deliberately does not inject silence, to avoid a harsh cutoff.
- `None` (hushed/panic) → fill zeros, push silence (`:331-343`).

**CPAL callback (`:380-471`)** — per device buffer:
- If `should_clear_ring.swap(false)` → `ring_consumer.skip(occupied_len())` (`:387-391`
  F32, `:425-429` I16). Only hush/panic sets the flag.
- Else `pop_slice`; on shortfall, zero-fill the tail and bump the atomic underrun counter
  (`:396-408`).

**hush/panic (`:2268-2283`)**: `graph.store(None)` **and** `should_clear_ring = true`.
Synth thread then continuously pushes silence; the callback drains the ring once. This is
the only path that clears the ring.

---

## 4. What Is Preserved vs Dropped Across a Swap

| State | Transferred? | Where | Notes |
|-------|-------------|-------|-------|
| Cycle position / beat clock | **Yes** | `transfer_session_timing` `unified_graph.rs:5652` | Wall-clock reference carried; beat continuous *if transfer succeeds* (see R1). |
| CPS / tempo | **No (by design)** | `:5650` comment | New code's tempo wins; the *phase* is preserved, the rate changes. |
| Delay, Reverb, Chorus, Flanger, Compressor, Limiter, LowPass, HighPass, BandPass state | **Yes** | `transfer_fx_states` `:7998-8175` | Simple `# lpf`/`# delay` tails survive. |
| TapeDelay, MultiTapDelay, PingPongDelay, DattorroReverb, Convolution, SidechainCompressor, Expander, MoogLadder state | **No (counted only)** | `:8176-8210` | Tail resets to zero → D2. |
| Active sample voices (drum hits, one-shots) | **No — faded** | `release_sample_voices` `voice_manager.rs:2381-2388` | 10 ms quick-release, then die → D1. |
| Active synthesis voices (pattern-driven notes) | **No — faded + detached** | `release_synthesis_voices` `voice_manager.rs:2367-2376` | 10 ms release *and* `synthesis_node_id = None` (cannot continue) → D1. |
| Buffer-boundary crossfade tail | **No** | `prev_buffer_tail` `unified_graph.rs:4871` | Not in any transfer fn → crossfade skipped across swap → D3. |
| VST3 plugins | **Yes (shared)** | `Arc<Mutex>` shared across clones, `mod.rs:726-727` | No explicit transfer needed. |
| Ring buffer contents (old audio) | **Not cleared** | `mod.rs:765-768` | ~100 ms of old audio still plays → D4. |

---

## 5. Discontinuities (audible but non-fatal)

### D1 — Every swap fades and kills active voices
**Source:** `load_code` `mod.rs:724` → `transfer_voice_manager`
`unified_graph.rs:5629-5635` → `release_sample_voices` / `release_synthesis_voices`
`voice_manager.rs:2367-2388`.
**Reality vs comment:** `load_code:722-724` claims "Transfer VoiceManager to preserve
active voices! This prevents the click from voices being cut off mid-sample." The
implementation **releases** every voice with a 10 ms fade and, for synth voices, nulls
`synthesis_node_id` so they cannot resume. Voices are *faded*, not preserved.
**Reproduce:** In `phonon edit`, run `~pad $ saw "55" # lpf 800 0.7` and `out $ ~pad`.
Press **C-x** on the block while the pad sustains → a 10 ms dip/retrigger is audible on
every press. Worse with a long one-shot: `~s $ s "break:0"` (a 2 s loop) C-x'd mid-sample
truncates the sample to a 10 ms fade.
**Severity:** Medium. Not a click (the 10 ms fade prevents that) but a perceptible
amplitude notch / retrigger on *every* evaluation, and outright truncation of long samples.

### D2 — Partial FX-state transfer resets some effect tails
**Source:** `transfer_fx_states` transfers 9 effect types (`:7998-8175`) but only
**counts** 8 others (`:8176-8210`).
**Reproduce:** `~d $ s "bd*4" # pingpong 0.25 0.5 0.4` then C-x an unrelated tweak.
The ping-pong echo tail snaps to silence at the swap (its buffer is re-zeroed in the new
graph) → click + lost tail. Same for TapeDelay, DattorroReverb, MoogLadder resonance, etc.
**Severity:** Medium, effect-type-dependent. Confusing because `# lpf`/`# delay` survive
but `# pingpong`/`# moog` do not — inconsistent live-edit feel.

### D3 — Cross-swap crossfade never fires
**Source:** Phase 4d boundary crossfade (`unified_graph.rs:7728-7764`) reads
`self.prev_buffer_tail`; the **new** graph starts with `prev_buffer_tail = Vec::new()`
(`:4960`, `:5050`) and nothing transfers it. The guard `if self.prev_buffer_tail.len()
>= 2` (`:7738`) is false on the new graph's first buffer, so the swap boundary is **not**
smoothed. The crossfade only smooths edges *within* one graph's buffer stream.
**Reproduce:** the offline harness already shows this: `osc-waveform-sine-to-saw`
produces `disc=0.3301` (`docs/research/audio-glitch-reproduction.md:75`), and several sine
scenarios cluster at `disc=0.2925` — a phase-dependent step at the swap sample. At 44.1 kHz
a 0.2–0.33 step is an audible click. Because the ring is not cleared (D4), the actual click
occurs in the *ring* between the old graph's last pushed sample and the new graph's first —
which no code smooths.
**Severity:** Medium. Phase-dependent click on every swap; worst on waveform/topology
changes, near-zero when the swap lands at a zero crossing (luck).

### D4 — Stale-audio latency (ring not cleared on C-x)
**Source:** `load_code:765-768` intentionally leaves the ring intact for "groove
continuity".
**Consequence:** After `graph.store`, up to one ring depth (~100 ms) of **old-code** audio
is still queued and plays before the new graph is heard. C-x therefore has ~0–100 ms of
apparent latency, and the audible swap point is offset from the keystroke.
**Reproduce:** `~d $ s "bd*8"` at a fast tempo, C-x to `~d $ s "cp*8"`. Up to ~100 ms of
`bd` plays after the highlight flashes before `cp` is heard.
**Interaction with the CLAUDE.md claim:** the status note "Ring buffer clear on graph swap
(instant C-x transitions)" describes the **hush/panic** path, not C-x. C-x transitions are
*not* instant; they trail by up to a ring depth. This is a documentation/expectation
mismatch worth correcting.
**Severity:** Low–Medium. Trade-off, not a bug: clearing would remove the stale tail but
expose D3's click immediately and risk a gap under load.

### U1 — C-x replaces the entire graph with only the evaluated chunk
**Source:** `eval_chunk:2154` passes `&chunk` (a single paragraph) to `load_code`, which
`compile_program`s it into a **complete replacement** graph.
**Consequence:** If the current session is multiple buses and you C-x a block that has no
`out`/`out:` (e.g. just `~bass $ saw 55`), the resulting graph has no output → silence, and
all other buses are gone until you C-r (reload all). The status line even warns `out: NO!`
(`:2170-2174`). The stale comment at `:2112` ("we send the FULL session content") documents
the opposite of what the code does.
**Reproduce:** Session with `~drums`, `~bass`, and `out $ ~drums + ~bass`. Put the cursor on
the `~bass` block (blank-line separated) and press C-x → output drops to silence.
**Severity:** Medium (UX/semantic). Not a low-level race but a primary source of "it went
silent when I evaluated" confusion. Reconcile with the intended Tidal-style block model.

---

## 6. Races (timing-dependent, harness-invisible)

All of these require the concurrent synth thread + ring + `RefCell` that the harness
replaces with a direct move (`tests/audio_live_edit_glitch_harness.rs:778-794`).

### R1 — Heavy voice load defeats state transfer → beat jump
**Window:** The transfer retry loop gives up after **25 ms** (`mod.rs:695`, 50 × 500 µs).
The synth thread holds the old graph's `RefCell` borrow for the duration of one
`process_buffer` (`:280-283`). The code itself flags renders exceeding **11.6 ms** as a
"NEW PEAK" over budget (`:293-302`). If a dense patch renders in >25 ms, the UI thread's
`try_borrow_mut` never succeeds within the window → `state_transferred = false` →
`transfer_session_timing` is skipped → the new graph runs on fresh wall-clock timing.
**Symptom:** Beat jumps to a new phase (tempo is still correct because
`enable_wall_clock_timing` ran, but `cycle_offset`/`session_start_time` are reset). FX and
voice transfer are also skipped in this branch.
**Reproduce:** Build a patch heavy enough to push `process_buffer` over ~25 ms (many
polyphonic synth voices + several effects), keep it playing, and C-x repeatedly. Some
evaluations will log "Could not transfer state after retries" (`:746`) and jump the beat.
**Severity:** High when it fires (musically obvious), load-dependent, and **completely
untested** — the harness never contends for the borrow.

### R2 — Synth-thread starvation during transfer → ring drain → underrun
**Window:** While the UI thread holds the old graph's borrow (up to 25 ms), every synth
iteration hits the `Err` arm and **skips** (`:314-329`) — it pushes nothing. The ring
drains at device rate (~100 ms depth). If the machine is CPU-saturated (UI redraw + synth +
compile competing) and the transfer sits near its 25 ms ceiling, ring occupancy can reach
zero → the CPAL callback zero-fills and increments the underrun counter (`:400-407`).
**Symptom:** Brief dropout / click at the swap under load; underrun counter climbs (visible
in the editor's perf line).
**Reproduce:** Single-core / `taskset -c 0`, a moderately heavy patch, mash C-x. Watch the
underrun counter increment on evaluations that coincide with a full render.
**Severity:** Medium. Bounded (25 ms < 100 ms nominal) but real under contention; not
covered because the harness has no ring.

### R3 — Old graph rendered with an emptied VoiceManager (take/store window)
**Window:** `take_voice_manager` (`unified_graph.rs:5621-5625`) replaces the **old** graph's
`VoiceManager` with a fresh empty one *while the old graph is still the one in `ArcSwap`*.
The borrow is dropped at `break` (`mod.rs:730`); `store` does not happen until after
`preload_samples` (`:758-763`). In the gap between drop-borrow and store, the synth thread
can `load()` the old graph and render it — now with **no voices**.
**Symptom:** Any sample/synth voice that was sounding vanishes for the length of the
`preload_samples` window instead of getting its 10 ms fade. Usually negligible
(`preload_samples` is a cache hit and returns in microseconds), but if the swap introduces a
new, uncached sample, `preload_samples` does disk I/O (`:5589-5600`) and the window widens.
**Reproduce:** C-x a change that references a never-before-loaded large sample; the
previously-sounding voice cuts abruptly rather than fading.
**Severity:** Low–Medium. Narrow window, but interacts badly with sample cold-loads.

### R4 — Rapid successive C-x
**Behavior:** `load_code` runs **synchronously on the UI event loop**; a second C-x cannot
begin until the first returns. So there is **no swap-arriving-mid-swap reentrancy and no
crash** — swaps serialize. `ArcSwap` + the synth thread's `load()` guard also guarantee the
old graph's `Arc` stays alive until the synth finishes its in-flight render (no
use-after-free).
**Residual cost:** Each C-x independently (a) fades all voices (D1), (b) opens a 25 ms
borrow/skip window (R1/R2), and (c) adds a swap-boundary click (D3). Mashing C-x therefore
compounds into choppiness and repeated ring pressure even though nothing crashes.
**Reproduce:** Hold/repeat C-x ~5×/s on a voiced patch → stuttering fades and occasional
underruns.
**Severity:** Low (no crash) but a real quality-of-experience floor; untested.

### Non-issue: the old `borrow_mut()` panic (prior-audit B7) is absent here
Both sides use `try_borrow_mut` (synth `:280`, reload `:696`), so the two threads never
panic on double-borrow — the loser simply skips/retries. The `src/live.rs` `LiveSession`
path still uses raw `borrow_mut()` on its synth side and is the surface where B7 can still
bite; the modal editor is clean. (See §7.)

---

## 7. Reconciliation With Prior Fixes

| Prior work (commit) | What it solved | State now | What remains |
|---------------------|----------------|-----------|--------------|
| `fix-static-mut` (3ff3423) | `static mut` underrun counters → UB | Modal editor uses `AtomicUsize` (`mod.rs:214,407`) | — (modal editor clean) |
| `fix-i16-callback` (f558d62) | I16 callback allocated `Vec` per call | Conversion buffer preallocated (`mod.rs:418`) | Residual: resize-in-callback if device buffer > 4096 frames (`:434-436`) — rare (prior R4). |
| `fix-callback-issues` (6ad14f1) | `phonon-audio` callback cleanups | phonon-audio surface | Not modal editor; separate surface. |
| `fix-ipc-emergency` (11f93cf) | IPC coalescing dropped Hush/Panic | `phonon-audio`; regression test `tests/audio_live_edit_glitch_harness.rs:960` | Not modal editor (modal hush/panic are direct, no IPC). |
| `unify-phonon-audio` (b8b7e64) | phonon-audio reload lacked state transfer | phonon-audio now transfers | Shares D1/D2 (same transfer fns). |
| `add-sample-preload` (1fda28a) | `live.rs` reload lacked preload | `live.rs` now preloads | `live.rs` still uses raw `borrow_mut` (B7 lives here, not in modal). |
| `unify-phonon-live` (5a77a32) | `main.rs` file-watch reload had no transfer | main.rs now mirrors modal transfer | Shares D1/D2/D3/R1. |
| add-graph-output NaN guard (7e130be) | NaN/Inf could poison the ring | Output sanitized before ring (`unified_graph.rs:7717-7726`) | Covers the DAG path (`process_buffer_dag`) and `process_sample_multi` (`:17749`); good. |
| ring-buffer-clear-on-swap | Harsh cutoff / stale audio on swap | Synth **skips** (not silence) on contention (`mod.rs:314-329`); ring cleared **only** on hush/panic (`:387-391`) | C-x deliberately does **not** clear ring (D4); no cross-swap crossfade (D3). |
| glitch harness (c386856) + extend (302cc3f) | Regression coverage for reload | 30-cycle modal path + phonon_live + live_rs + IPC tests | **Still headless**: no synth thread, no `RefCell` contention, no ring, no CPAL. R1–R4 uncovered. |

**Net:** the *structural safety* fixes (UB, NaN, panic-in-modal, IPC emergencies, per-surface
transfer unification) are in and effective. The *continuity/quality* gaps —
voice preservation (D1), full FX transfer (D2), cross-swap smoothing (D3), and the
*concurrency* gaps under load (R1–R3) — remain, and the harness cannot observe them because
it stubs out the concurrent machinery.

---

## 8. Remediation Options (ranked)

Ranked by (audible-stability impact) × (implementation risk⁻¹). Each is scoped for a
follow-up implementation task; none is implemented here.

### Rank 1 — Transfer `prev_buffer_tail` across the swap (fixes D3)
**Change:** In `transfer_session_timing` (or a new `transfer_render_continuity`), copy
`old_graph.prev_buffer_tail` into `new_graph.prev_buffer_tail` so the existing Phase-4d
crossfade (`unified_graph.rs:7728-7764`) fires on the new graph's first buffer.
**Trade-off:** ~3 lines, no new machinery, reuses a proven crossfade. Only smooths the
*engine-level* boundary; because the ring is not cleared (D4), the audible boundary is in
the ring, so this helps most if paired with Rank 4 (clear-and-crossfade). Low risk, high
value. **Best first move.**

### Rank 2 — Complete FX-state transfer (fixes D2)
**Change:** Extend `transfer_fx_states` (`:8176-8210`) to actually inject state for
TapeDelay, MultiTapDelay, PingPongDelay, DattorroReverb, Convolution, SidechainCompressor,
Expander, MoogLadder (they are already keyed/counted; add the `ExtractedFxState` arms and
`extract_fx_states` coverage).
**Trade-off:** Mechanical but touches many node variants; needs a per-effect extract/inject
pair and a harness scenario per effect. Medium cost, removes a whole class of
effect-dependent clicks. Medium risk (state-struct plumbing).

### Rank 3 — Optionally *continue* voices instead of always fading (mitigates D1)
**Change:** Make voice handling on swap policy-driven. For **sample one-shots**, let them
ring out under their own envelope (don't force-release) unless voice-count pressure demands
stealing; for **synthesis voices**, only release those whose `synthesis_node_id` has no
counterpart in the new graph. This requires mapping old→new node identity (e.g. stable bus
names) rather than the current blanket `release_*`.
**Trade-off:** Real design work — needs voice→node identity across compiles and a stealing
policy to bound accumulation (the current blanket release exists precisely to prevent
unbounded voices on rapid swaps). High value for pads/long samples, higher cost/risk. Ship
behind a flag; keep the 10 ms fade as the fallback.

### Rank 4 — Clear-and-crossfade ring on C-x (addresses D3+D4 together)
**Change:** On swap, instead of "leave the ring" *or* "hard-drain the ring", apply a short
equal-power fade between the ring's stale tail and the new graph's first output at the
consumer boundary (a small crossfade region rather than `skip(occupied_len())`).
**Trade-off:** Removes the ~100 ms stale latency (D4) *and* the boundary click (D3) at once,
but is the most invasive: the crossfade must live at the ring boundary (SPSC, realtime
callback) or in a dedicated splice step, and must stay allocation-free. Medium–high cost;
supersedes Rank 1 if done. Prototype behind a setting and A/B against the current
"play-out" behavior (some users prefer groove continuity).

### Rank 5 — Bound/queue the transfer to remove the beat-jump and starvation windows (fixes R1, R2, R3)
**Change:** Move the swap to a **render-thread-owned** model: the UI thread compiles +
preloads off-thread, then hands the finished graph to the synth thread via a lock-free
message; the synth thread performs `transfer_*` and the pointer swap **at a buffer
boundary** (single owner, no cross-thread `RefCell` borrow, no 25 ms retry ceiling). This
is the "render-owner" fix recommended by the 2026-05-24 audit
(`docs/research/realtime-reload-audit.md`, rank 2).
**Trade-off:** Eliminates R1 (no give-up-and-reset), R2 (no borrow contention on the render
thread), and R3 (transfer + swap are atomic w.r.t. render), and unifies all four live
surfaces. Highest cost/risk (touches the threading model of every frontend). Do after the
cheaper D-fixes land and after the harness can actually exercise concurrency (below).

### Rank 6 — Extend the harness to cover the concurrent path (enables verifying R1–R4)
**Change:** Add a harness mode that spins the **real** synth thread + `HeapRb` ring +
`ArcSwap`/`RefCell` (no CPAL device needed — drive the ring consumer manually) and:
(a) forces a slow `process_buffer` to reproduce R1's 25 ms transfer failure and assert the
beat does **not** jump; (b) measures ring occupancy across a swap to catch R2 underruns;
(c) fires N rapid swaps to catch R4 accumulation. Also add the doc-comment fixes for U1 /
`eval_chunk:2112` / the CLAUDE.md "instant C-x" claim.
**Trade-off:** Pure test/observability investment; unblocks confident work on R1–R5. Low
risk, no runtime change. Should precede Rank 5 so the threading rewrite is verifiable.

### Also worth a cheap follow-up (out of swap scope)
- Clarify **U1**: decide whether C-x should replace the whole graph (current) or merge the
  evaluated bus into the running graph (true Tidal-style per-orbit replacement), and fix the
  contradictory comments at `eval_chunk:2112` and `load_code:722-724`.
- Background risks carried from the prior audit and still present: `Box::leak` per parse
  (`src/compositional_parser.rs`, unbounded over a long session), per-buffer DAG
  allocations in `process_buffer_dag`, and the I16 resize-in-callback edge (`mod.rs:434`).

---

## 9. Validation of This Audit

- Report exists at `docs/audits/live-transition-2026-07.md`. ✅
- Swap protocol documented step-by-step with file:line references (§3). ✅
- Each race/discontinuity (D1–D4, U1, R1–R4) has a concrete reproduction scenario
  (§5, §6). ✅
- Remediation options ranked with trade-offs (§8). ✅
- Prior fixes reconciled — what each solved and what remains (§7). ✅
- No behavioral code changes were made.

### Suggested follow-up WG tasks
1. `swap-transfer-prev-buffer-tail` — Rank 1 (low risk, high value).
2. `complete-fx-state-transfer` — Rank 2.
3. `harness-concurrent-swap-mode` — Rank 6 (unblocks verifying R1–R5).
4. `render-owner-graph-swap` — Rank 5 (large; after 1–3).
5. `clarify-cx-block-semantics` — U1 doc + behavior decision.
