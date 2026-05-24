# Phonon Audio Stabilization Roadmap

Synthesized from three predecessor research artifacts:

- **Architecture map**: `docs/research/audio-architecture-map.md` (agent-14 / 5e9cca0)
- **Realtime/reload audit**: `docs/research/realtime-reload-audit.md` (agent-15 / f000550)
- **Live-edit glitch harness results**: `docs/research/audio-glitch-reproduction.md` (agent-20 / 8f4bd30)

No source-level fixes are implemented in this document. The deliverable is the roadmap and the follow-up WG task graph.

---

## 1. Current Understanding of the Repository

Phonon is a Rust live-coding DSL backed by a single large synthesis engine
(`UnifiedSignalGraph`) that combines pattern scheduling, sample playback, synthesis
voice management, and a block-based DAG renderer.

Five execution surfaces share the same core engine:

| Surface | Entry point | Live? | Ring buffer |
|---------|------------|-------|-------------|
| `phonon live` (file watcher) | `src/main.rs:869–1106` | Yes | 1 s |
| Modal editor (`phonon edit`) | `src/modal_editor/mod.rs` | Yes | ~200 ms |
| Two-process audio server | `src/bin/phonon-audio.rs` | Yes | 2 s |
| `src/live.rs` `LiveSession` | `src/live.rs` | Yes | 1 s |
| Offline render / play | `src/main.rs:237–700` | No | — |

All four live surfaces follow the same broad pattern:
```
compile → UnifiedSignalGraph
         ↓
   ArcSwap<Option<GraphCell(RefCell<…>)>>
         ↓ synthesis thread
   process_buffer_dag (DAG renderer)
         ↓
   HeapRb<f32> ring buffer (SPSC)
         ↓ CPAL callback
   device output
```

The **modal editor** is the most complete surface: it performs state transfer
(timing + FX + voice manager + sample preload) before swapping the graph.
The other three live surfaces have gaps of varying severity.

The **glitch harness** tests the modal-editor state-transition path in headless
mode over 30 cycles and passes with no hard failures. All five live surfaces
ultimately call the same `process_buffer_dag` rendering kernel.

---

## 2. Top Failure Modes

### 2a. Confirmed Bugs

These are unambiguously present in the current source code with known audible
symptoms.

#### B1 — `phonon live` performs no state transfer on reload  
**Source**: `src/main.rs:1080–1089` (reload path) vs `src/modal_editor/mod.rs:628–770` (reference path)  
**Evidence**: Architecture map §"phonon live file reload" confirms no call to
`enable_wall_clock_timing`, `transfer_session_timing`, `transfer_fx_states`,
`transfer_voice_manager`, or `preload_samples` in the reload window.  
**Audible symptom**: Beat restart/jump on every save; FX tails cut; active voices
cut; first-hit sample dropout after reload.

#### B2 — `phonon-audio` (two-process server) performs no FX/voice transfer and no sample preload  
**Source**: `src/bin/phonon-audio.rs:472–487` (IPC reload) has no transfer calls.  
**Evidence**: Realtime audit §"Standalone phonon-audio IPC Reload" confirms only
`GlobalClock` CPS update and graph store.  
**Audible symptom**: FX tails cut on every code update; first-hit sample dropout
after every update; complete state loss on audio process relaunch.

#### B3 — IPC coalescing drops Hush/Panic/SetTempo messages during rapid edits  
**Source**: `src/ipc.rs:143–190` — `receive_coalesced` keeps only the newest
`UpdateGraph` message and discards all non-`UpdateGraph` messages it encounters
while draining the burst.  
**Evidence**: Architecture map §"IPC update coalescing" and realtime audit §
"Standalone phonon-audio IPC Reload/Relaunch" both confirm this.  
**Audible symptom**: Emergency silence (`Hush`, `Panic`) ignored during rapid
live edits; tempo changes ignored.

#### B4 — I16 CPAL callbacks allocate `Vec<f32>` in the realtime path  
**Source**: `src/live.rs:198,205` and `src/bin/phonon-audio.rs:380,398`.  
**Evidence**: Realtime audit §"Direct CPAL Callback Risks" rows for both files,
severity High.  
**Audible symptom**: Periodic allocator stalls on non-F32 CPAL devices causing
underruns.

#### B5 — `static mut UNDERRUN_COUNT` used inside CPAL callbacks  
**Source**: `src/main.rs:1040–1047` and `src/bin/phonon-audio.rs:350–356,423–428`.  
**Evidence**: Realtime audit §"Direct CPAL Callback Risks" rows; UB if CPAL host
invokes callback from multiple threads simultaneously.  
**Audible symptom**: Undefined behavior risk; unreliable underrun diagnostics.

#### B6 — Recording path in `phonon-audio` locks a mutex and writes from the CPAL callback  
**Source**: `src/bin/phonon-audio.rs:359–365` (F32) and `src/bin/phonon-audio.rs:383–410` (I16).  
**Evidence**: Realtime audit §"Direct CPAL Callback Risks" row, severity High.  
**Audible symptom**: Dropouts while recording, especially on slow-disk or
high-flush-pressure systems.

#### B7 — `src/live.rs` `load_file` borrows the old graph mutably during state transfer while the synth thread may hold the same `borrow_mut`  
**Source**: `src/live.rs:280–284` (`borrow()` / `borrow_mut()` for transfer) vs
`src/live.rs:100` (synth thread `borrow_mut`).  
**Evidence**: Realtime audit §"File Live Watch Reload" identifies this as a High
risk that can panic and terminate audio.  
**Audible symptom**: Process termination / audio stop during a save if reload and
render happen to overlap.

#### B8 — Voice transfer releases rather than preserves active voices  
**Source**: `src/unified_graph.rs:5619–5635` (`transfer_voice_manager` releases
synthesis and sample voices before installing the old manager).  
**Evidence**: Realtime audit §"Modal Editor Reload" and §"State Transfer
Fidelity" confirm the mismatch between the code comment ("active voices continue
playing") and the implementation.  
**Audible symptom**: Surprise fade-outs or clicks on reload when sample/synth
voices are mid-playback.

### 2b. Plausible Risks and Unknowns

These are identified structural risks that have not been directly observed to
cause a failure, either because the current usage avoids the path or because the
harness is not broad enough to exercise them.

#### R1 — Per-buffer DAG allocations may cause underruns under complex graphs  
`process_buffer_dag` rebuilds dependency/topological/batch structures and
allocates node-output caches, voice buffers, expression operand buffers, and
pattern trigger vectors every buffer call  
(`src/unified_graph.rs:7246–7528`, `7819–7843`, `21962–22084`). The harness
uses simple sine/saw+LPF programs; a program with many buses, heavy pattern
density, or arithmetic modulation chains will stress this path further.

#### R2 — `Box::leak` in parser accumulates memory over long sessions  
`parse_program` leaks the preprocessed string every call
(`src/compositional_parser.rs:583–613`). Over a long live-coding session this
is unbounded. Risk becomes observable only after many tens or hundreds of edits.

#### R3 — NaN/Inf/denormal propagation from stateful nodes  
DJ filter paths sanitize locally (`src/unified_graph.rs:15492–15525,
20250–20285`), but there is no universal guard at final graph output before ring
write. The harness observed 0 NaN/Inf samples, but NaN from a resonant filter,
FM feedback, or plugin crash is not ruled out.

#### R4 — Modal editor I16 callback resize  
`src/modal_editor/mod.rs:433–435` can resize the preallocated conversion buffer
if CPAL requests a buffer larger than 4096 frames. Unlikely in practice but
present.

#### R5 — MIDI monitoring mutex contention  
Render path drains a `Mutex<VecDeque>` while MIDI callback pushes
(`src/unified_graph.rs:10797–10828`, `src/midi_input.rs:197–216`). Priority
inversion if MIDI callback holds lock during render.

#### R6 — Plugin path allocations and locks during render  
Plugin nodes lock plugin managers, may initialize plugins, and allocate scratch
during `process_buffer_dag`
(`src/unified_graph.rs:18855–19088`,
`src/plugin_host/real_plugin.rs:128–186`). Risk is activation-dependent; no
plugin testing was done in the harness.

#### R7 — Ring buffer starvation during the 25 ms compile window  
Harness measured avg 25.1 ms compile time, max 31.4 ms. With a 512-frame buffer
at 44100 Hz (~11.6 ms) and a 200 ms ring (modal editor), there is ~17× headroom
nominally. But if the UI and synth threads share a saturated CPU, the ring can
drain during compile.

---

## 3. Highest-Leverage Fixes

Ranked by: (audible/stability impact) × (implementation risk⁻¹)

| Rank | Fix | Impact | Cost | Grounds |
|------|-----|--------|------|---------|
| 1 | **IPC emergency message preservation** (B3) | High | Low | One function in `src/ipc.rs`; safety-critical for Hush/Panic. |
| 2 | **Fix I16 CPAL callback allocations** (B4) | High | Low | Localized to three files; preallocate conversion buffer from stream config. |
| 3 | **Replace static mut underrun counters with atomics** (B5) | Medium | Low | Directly eliminates UB; needed before metrics are trustworthy. |
| 4 | **Add global NaN/Inf/denormal guard at graph output** (R3) | Medium | Low | Single insertion point before ring write; cheap defensive containment. |
| 5 | **Unify `phonon live` reload with full state transfer** (B1) | High | Medium | Closes the largest confirmed gap; mirrors existing `load_code` pattern. |
| 6 | **Unify `phonon-audio` reload with FX/voice transfer + sample preload** (B2) | High | Medium | Second largest confirmed gap; same fix pattern as modal editor. |
| 7 | **Add sample preload to `src/live.rs` reload path** (related to B2) | High | Low | One missing call to `preload_samples`; prevents first-hit dropout after save. |
| 8 | **Move recording to non-realtime thread in `phonon-audio`** (B6) | Medium | Medium | SPSC ring to writer thread; prevents recording-induced dropouts. |
| 9 | **Fix old-graph borrow race in `src/live.rs` reload** (B7) | High | Medium | Replace cross-thread RefCell access with atomic graph ownership transfer. |
| 10 | **Precompute DAG plan at compile time; reuse scratch** (R1) | High | High | Largest structural change; deferred to after lower-cost wins are in. |

---

## 4. Test Strategy

### 4a. Existing Coverage (harness baseline)

The live-edit glitch harness (`tests/audio_live_edit_glitch_harness.rs`) covers the
modal-editor state-transition path in headless mode over 30 cycles. Current
baseline (run 2026-05-24):

| Metric | Baseline | Hard threshold |
|--------|----------|---------------|
| NaN samples | 0 | Any → FAIL |
| Inf samples | 0 | Any → FAIL |
| Severe-clip cycles | 0 | >0 → FAIL |
| Silent cycles | 0 | >0 → FAIL |
| Stuck cycles | 0 | >0 → FAIL |
| Max discontinuity | 0.3301 | >0.5 → FAIL |
| High-RMS-jump cycles | 1 (expected: osc→silence) | — |
| Avg reload time | 25.1 ms | — |
| Max reload time | 31.4 ms | — |

Every implementation task that touches state transfer must run the harness and
confirm no regression against this baseline.

### 4b. Gaps in Current Coverage

The harness does **not** exercise:
1. Ring buffer starvation under live CPAL stream (no device open)
2. Actual ArcSwap race between graph swap and render thread
3. I16 CPAL device paths (allocation bugs are structural, not signal-level)
4. `phonon live` or `phonon-audio` reload paths (harness mirrors modal editor)
5. IPC coalescing (requires running phonon-audio process + IPC client)
6. Allocation latency spikes (harness detects effects but not causes)

### 4c. Test Extensions Required by Implementation Tasks

For each live-surface reload fix, add a harness scenario that calls the
surface's reload function directly and checks the 30-cycle baseline metrics.
For the IPC fix, add a unit test that sends a burst of `UpdateGraph` messages
with a `Hush` or `Panic` interleaved and asserts the emergency message is not
discarded. For I16 callback fixes, assert no `Vec::new()` or `.to_vec()` inside
the callback closure using Rust's `#[global_allocator]` counting trick or a
review checklist.

---

## 5. End-to-End Acceptance Target for Live Edit and Relaunch Stability

A Phonon session is considered **stable** when all of the following are true:

### Signal quality (offline harness, all surfaces)
- Zero NaN or Inf samples in any rendered buffer over a 30-cycle load test
- Zero severe-clipping cycles (>5% of samples with |s| > 1.0)
- Zero unexpected silent cycles
- Maximum boundary discontinuity < 0.35 (current max is 0.3301; aiming to hold,
  not degrade)

### Realtime behavior (CPAL stream running)
- Zero underruns in a 5-minute idle session with no edits
- Zero underruns during a 30-cycle rapid-edit session (save once per second)
- Ring buffer fill ≥ 10% at all times during rapid-edit session on a
  single-core load (measured via modal editor `ring_fill_percent` atomic)

### Live-edit state continuity (human-observable)
- Tempo/cycle position continuous across reload (no beat jump audible at 120 BPM)
- Reverb/delay tails survive across a reload that does not change effect topology
- No audible click larger than -40 dBFS at reload boundary (measured from
  `disc` metric in harness)

### Safety commands
- `Hush` clears audio within one ring-buffer duration (≤ 200 ms for modal
  editor, ≤ 2 s for phonon-audio)
- `Panic` clears audio within one ring-buffer duration even during a rapid-edit
  burst (i.e., Hush/Panic survive IPC coalescing)

### Memory stability
- Resident memory does not grow by more than 10 MB after 100 rapid edits
  (detects `Box::leak` accumulation) — measured with `/usr/bin/time -v` or
  `valgrind --tool=massif`

---

## 6. Proposed WG Task Breakdown

The following tasks are created by this roadmap agent (see logs for task IDs).
All tasks depend on `audio-stabilization-roadmap` being complete. Tasks that
modify the same file are serialized with `--after` dependencies.

### Independent tasks (no same-file conflict)

| Task | File scope | Depends on |
|------|-----------|-----------|
| `fix-ipc-emergency-coalescing` | `src/ipc.rs` | roadmap |
| `add-graph-output-nan-guard` | `src/unified_graph.rs` | roadmap |

### `src/live.rs` sequential chain

| Task | File scope | Depends on |
|------|-----------|-----------|
| `fix-i16-callback-alloc-live` | `src/live.rs` | roadmap |
| `add-sample-preload-live-rs` | `src/live.rs` | `fix-i16-callback-alloc-live` |

### `src/main.rs` sequential chain

| Task | File scope | Depends on |
|------|-----------|-----------|
| `fix-static-mut-underrun-main` | `src/main.rs` | roadmap |
| `unify-phonon-live-reload` | `src/main.rs` | `fix-static-mut-underrun-main` |

### `src/bin/phonon-audio.rs` sequential chain

| Task | File scope | Depends on |
|------|-----------|-----------|
| `fix-callback-issues-phonon-audio` | `src/bin/phonon-audio.rs` | roadmap |
| `unify-phonon-audio-reload` | `src/bin/phonon-audio.rs` | `fix-callback-issues-phonon-audio` |

### Harness extension (depends on all surface fixes)

| Task | File scope | Depends on |
|------|-----------|-----------|
| `extend-glitch-harness-surfaces` | `tests/audio_live_edit_glitch_harness.rs` | all surface tasks |

---

## Appendix: Key Source References

| Area | File | Lines |
|------|------|-------|
| Modal editor load_code (reference reload path) | `src/modal_editor/mod.rs` | 628–770 |
| phonon live reload (missing state transfer) | `src/main.rs` | 1060–1106 |
| phonon-audio IPC reload (missing state transfer) | `src/bin/phonon-audio.rs` | 450–520 |
| IPC coalescing (drops emergency messages) | `src/ipc.rs` | 143–190 |
| live.rs reload (borrow race) | `src/live.rs` | 251–301 |
| I16 callback allocation (live.rs) | `src/live.rs` | 198, 205 |
| I16 callback allocation (phonon-audio) | `src/bin/phonon-audio.rs` | 380, 398 |
| Recording-in-callback (phonon-audio) | `src/bin/phonon-audio.rs` | 359–365, 383–410 |
| static mut underrun (main.rs) | `src/main.rs` | 1040–1047 |
| static mut underrun (phonon-audio) | `src/bin/phonon-audio.rs` | 350–356, 423–428 |
| transfer_voice_manager (releases voices) | `src/unified_graph.rs` | 5619–5635 |
| transfer_fx_states (partial coverage) | `src/unified_graph.rs` | 7956–8199 |
| Per-buffer DAG allocations | `src/unified_graph.rs` | 7246–7528 |
| NaN guard (local, DJ filter only) | `src/unified_graph.rs` | 15492–15525, 20250–20285 |
| Graph output / limiter (pre-ring-write) | `src/unified_graph.rs` | 7673–7715 |
| Box::leak in parser | `src/compositional_parser.rs` | 583–613 |
| Glitch harness | `tests/audio_live_edit_glitch_harness.rs` | — |
