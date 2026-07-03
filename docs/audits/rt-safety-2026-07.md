# Audio Engine Real-Time Safety Audit — 2026-07

Task: `audit-audio-engine`
Date: 2026-07-03
Scope: **investigation only — no behavioral code changes.** This report audits the
Phonon audio engine for real-time-safety violations that cause instability during
interactive/live use, traces the audio callback path end-to-end, reconciles the
prior audits against the current source, and shortlists the findings most likely
to cause interactive instability.

Predecessor artifacts (read and reconciled):

- `docs/research/audio-architecture-map.md` (agent-14) — architecture map
- `docs/research/realtime-reload-audit.md` (agent-15) — the detailed RT-safety audit (findings tabled here as B/R)
- `docs/research/audio-stabilization-roadmap.md` (agent-22) — synthesis + WG task graph (defines B1–B8, R1–R7)
- `docs/research/audio-glitch-reproduction.md` (agent-20) — glitch harness baseline

Fixes landed since the prior audits (verified against git history and current source):
`fix-static-mut` (main.rs), `fix-i16-callback` (live.rs), `fix-callback-issues`
(phonon-audio.rs), `fix-ipc-emergency` (ipc.rs + phonon-audio.rs), `add-graph-output`
(NaN guard in unified_graph.rs), `add-sample-preload` (live.rs), `unify-phonon-live`
(main.rs), `unify-phonon-audio` (phonon-audio.rs), `extend-glitch-harness` (tests).

---

## Executive Summary

The stabilization roadmap's low-cost, high-value fixes have largely landed and are
present in the current source: IPC emergency-message preservation, I16-callback and
static-mut cleanups, off-thread recording, the global NaN/denormal output guard, and
full state transfer on the `phonon live` and `phonon-audio` reload paths. The direct
CPAL-callback violations that the prior audit ranked #1 are, for the product surfaces,
**resolved** — the F32/I16 callbacks are now allocation-free, lock-free, and log-free.

However, the reload-transfer work (`unify-phonon-live`, `unify-phonon-audio`) closed the
"no state transfer" gap by making the reload thread **borrow the old graph** during
transfer — while the corresponding **synth threads still call unconditional
`RefCell::borrow_mut()`**. This spreads the prior B7 borrow-panic race (previously only in
`src/live.rs`) into the two most-used product surfaces (`phonon live` and `phonon-audio`).
This is the single most important open finding: it is a **crash** (panic that kills the
synth thread → permanent audio stop) reachable by ordinary live editing, and it is a
partial regression introduced by the very fixes that improved reload continuity.

The deeper structural risks the roadmap deferred as "high cost" remain **open**: the
plugin path still loads VST3s and allocates per-sample inside the render thread; the DAG
renderer still allocates/`getenv`s per buffer; the voice pool still heap-grows on the synth
thread; several per-node states sit behind `Mutex<f32>` locked with `.unwrap()` every
sample; `Box::leak` still accumulates per edit; and `transfer_voice_manager` still releases
(does not preserve) active voices on reload.

**Most-likely-to-destabilize shortlist** (detail in the final section):
1. **F-1** synth-thread `borrow_mut()` panic during reload transfer (`main.rs:1006`, `phonon-audio.rs:288`) — crash.
2. **F-2** VST3 plugin load + per-sample `Vec<String>` alloc + `lock().unwrap()` on the render thread (`unified_graph.rs:12122–12208`) — crash / severe glitch.
3. **F-3** `.lock().unwrap()` on per-node/clock mutexes on the render thread (`phonon-audio.rs:269`, `unified_graph.rs:14023`, `13788`) — crash on poison / priority inversion.
4. **F-4** voice-pool heap growth + `eprintln!` on the synth thread under dense triggers (`voice_manager.rs:905–955`) — glitch.
5. **F-5** per-buffer allocation / `getenv` in `process_buffer_dag` (`unified_graph.rs:7248+`) — glitch/underrun under complex graphs.

---

## 1. Scope & Method

Investigated per the task brief:

- Heap allocation, mutex/RwLock acquisition, file I/O, or unbounded work on the audio thread.
- Buffer-underrun sources and recovery behavior.
- NaN/Inf/denormal propagation through the signal graph.
- Voice-lifecycle races (stealing, retriggering, release during swap).

Method: read the three predecessor research docs; diffed the nine fix commits to learn what
each changed; then re-read the **current** source (line numbers below are current as of
this commit, not the prior audits') to verify each prior finding as fixed / still-open /
regressed, and to surface new findings the earlier passes did not name.

Two threads matter for RT-safety in every live surface:

- **CPAL device callback** — hard-real-time; must be allocation-free, lock-free, I/O-free, log-free.
- **Background synth thread** — the *effective* real-time path: it feeds the ring buffer, so
  any stall here drains the ring and produces a device underrun. It is not the device
  callback, but for RT purposes it is treated as real-time here.

---

## 2. Audio Callback Path — End-to-End Trace

Four live surfaces share one engine (`UnifiedSignalGraph`) and one broad shape:
`compile → ArcSwap<Option<GraphCell(RefCell<graph>)>> → synth thread renders → HeapRb ring → CPAL callback → device`.

### 2.1 `phonon-audio` (two-process server) — cleanest external-clock path

| Stage | File:line | Notes |
|---|---|---|
| Device F32 callback entry | `src/bin/phonon-audio.rs:398` | `move |data: &mut [f32], _|` |
| Ring read (fast path) | `src/bin/phonon-audio.rs:403` | `ring_consumer.pop_slice(data)` |
| Underrun path | `src/bin/phonon-audio.rs:406–410` | partial pop + silence fill + `underrun_count_f32.fetch_add` (atomic, no log) ✅ |
| Recording tap | `src/bin/phonon-audio.rs:414–416` | `rec_prod_f32.push_slice` into SPSC ring → writer thread (`:353–372`) ✅ |
| Device I16 callback | `src/bin/phonon-audio.rs:425–` | uses pre-allocated `i16_conv_buf` (`:367`), no alloc ✅ |
| Ring fed by synth thread | `src/bin/phonon-audio.rs:256–320` | spawned once |
| Clock read (per buffer) | `src/bin/phonon-audio.rs:269` | `clock_clone_synth.lock().unwrap()` ⚠️ (F-3) |
| Graph snapshot + render | `src/bin/phonon-audio.rs:288` | `graph_cell.0.borrow_mut().process_buffer_at(...)` ⚠️ **unconditional borrow_mut** (F-1) |
| → block renderer | `src/unified_graph.rs:17982` `process_buffer_at` → `:18014` `process_buffer_internal` → `:7240` `process_buffer_dag` | |
| → per-node eval | `src/unified_graph.rs:7802` `eval_node_buffer_dag` → `:10270` `eval_node` | allocs/locks/plugin/sample I/O live here (F-2..F-8) |
| Output mix + limiter | `src/unified_graph.rs:~7673–7715` | hard clamp to master ceiling |
| **NaN/denormal guard** | `src/unified_graph.rs:7719–7724` | flushes non-finite & `<1e-38` to 0 ✅ (add-graph-output) |
| Boundary crossfade | `src/unified_graph.rs:7726+` (Phase 4d) | equal-power cosine |
| Ring write | `src/bin/phonon-audio.rs:296` | `ring_producer.push_slice(&buffer)` |

### 2.2 `phonon live` (inline `Commands::Live`, the actual CLI command)

| Stage | File:line | Notes |
|---|---|---|
| Command dispatch | `src/main.rs:869` | `Commands::Live { .. }` |
| Synth thread render | `src/main.rs:1006` | `graph_cell.0.borrow_mut().process_buffer(&mut buffer)` ⚠️ **unconditional borrow_mut** (F-1) |
| Device F32 callback | `src/main.rs:1032–1054` | atomic underrun counter ✅ (fix-static-mut), no alloc/log |
| Initial load | `src/main.rs:976–977` | `enable_wall_clock_timing()` + `preload_samples()` ✅ |
| Reload transfer | `src/main.rs:1097–1131` | `try_borrow_mut` retries (`:1107`) + timing/FX/voice transfer + `preload_samples` ✅ (unify-phonon-live) — but see F-1 |

Note: `src/main.rs` only ever builds an **F32** stream here; no I16 arm.

### 2.3 Modal editor (`phonon edit`) — the safe reference surface

| Stage | File:line | Notes |
|---|---|---|
| Synth thread render | `src/modal_editor/mod.rs:280` | `match graph_cell.0.try_borrow_mut()` — **skips on `Err`** (`:314`) ✅ no panic |
| Device F32 callback | `src/modal_editor/mod.rs:384` | ring clear flag honored, atomic underruns ✅ |
| Device I16 callback | `src/modal_editor/mod.rs:422` | pre-allocated `conversion_buffer` (`:418`) but can `resize()` in callback (`:434–435`) ⚠️ (F-9 / prior R4) |
| Reload | `src/modal_editor/mod.rs:~628–770` | full transfer + preload; `try_borrow_mut` retries ✅ |

This is the only surface whose **synth side** uses `try_borrow_mut`; it is the pattern the
other surfaces should adopt (F-1).

### 2.4 `src/live.rs` `LiveSession` — present but unreachable from the CLI

`LiveSession` (`src/live.rs`) still exists, is `pub`, and is compiled/exported, but **no CLI
command constructs it** (verified: the only references to `LiveSession` are within
`src/live.rs`; `phonon live` uses the inline `src/main.rs:869` path). Its callbacks were
cleaned by `fix-i16-callback`/`add-sample-preload`, but its reload still uses the original
**panicking** `borrow()`/`borrow_mut()` on the old graph (`:284`, `:288`) with an
unconditional synth-thread `borrow_mut()` (`:108`). Because it is unreachable it is a
**latent trap** rather than an active crash (see F-10).

---

## 3. Reconciliation of Prior Findings

Prior IDs are from `audio-stabilization-roadmap.md` (B1–B8 confirmed bugs, R1–R7 risks).

| ID | Prior finding | Status | Evidence (current source) |
|---|---|---|---|
| **B1** | `phonon live` no state transfer on reload | **FIXED** | `src/main.rs:975–976` initial + `:~1092–1130` reload now do `enable_wall_clock_timing` + timing/FX/voice transfer + `preload_samples` (unify-phonon-live) |
| **B2** | `phonon-audio` no FX/voice transfer, no preload | **FIXED** | `src/bin/phonon-audio.rs:524–552` transfer + `preload_samples` (unify-phonon-audio) |
| **B3** | IPC coalescing drops Hush/Panic/SetTempo | **FIXED** | `src/ipc.rs` `receive_coalesced` returns `Vec<Self>`, emergency msgs pushed to `side_buffer`, never dropped; `phonon-audio.rs:485–580` dispatches all in order (Hush `:569`, Panic `:574`) (fix-ipc-emergency) |
| **B4** | I16 callbacks allocate `Vec<f32>` | **FIXED** (product) | `src/live.rs:201` & `src/bin/phonon-audio.rs:367` pre-allocate conversion buffers. Modal editor can still `resize()` — see R4/F-9 |
| **B5** | `static mut UNDERRUN_COUNT` in callbacks | **FIXED** | `src/main.rs:1049` & `phonon-audio.rs:395,409` use `Arc<AtomicUsize>`; live.rs `:191,224` atomic |
| **B6** | Recording locks mutex + writes WAV in callback | **FIXED** | `src/bin/phonon-audio.rs:353–372` dedicated writer thread fed by SPSC ring; callback only `push_slice` |
| **B7** | `src/live.rs` reload borrows old graph while synth borrows same | **STILL OPEN + SPREAD** | `src/live.rs:284/288` unchanged (but path unreachable, F-10). **Newly present in product surfaces**: reload transfer added to `main.rs`/`phonon-audio.rs` holds `try_borrow_mut` on old graph while their synth threads use unconditional `borrow_mut` (`main.rs:1006`, `phonon-audio.rs:288`) → **F-1** |
| **B8** | Voice transfer releases rather than preserves voices | **STILL OPEN** | `src/unified_graph.rs:5629` `transfer_voice_manager` calls `release_synthesis_voices()` + `release_sample_voices()` before install |
| **R1** | Per-buffer DAG allocations | **STILL OPEN** | `src/unified_graph.rs:7248` `env::var("DEBUG_DAG")` per buffer; `HashMap::new()`/`vec!`/`.collect()` in DAG head; per-node/voice/expression scratch still allocated in render (F-5) |
| **R2** | `Box::leak` in parser accumulates | **STILL OPEN (worse)** | `src/compositional_parser.rs:586` **and** `:632` — two `Box::leak` per parse, every edit (F-7) |
| **R3** | NaN/Inf/denormal propagation, no global guard | **FIXED (output) / partial (internal)** | Guard added at `unified_graph.rs:7719–7724` (buffer path) and `:17750–17753` (`process_sample_stereo`). **Gap**: internal stateful-node state is not sanitized → a filter/FM that goes NaN internally stays NaN (stuck silence); `process_sample` (mono) and `process_sample_multi` have no guard (offline/example only). See F-6 |
| **R4** | Modal I16 callback can resize | **STILL OPEN** | `src/modal_editor/mod.rs:434–435` `conversion_buffer.resize(data.len(), 0.0)` inside callback (F-9) |
| **R5** | MIDI monitoring `Mutex<VecDeque>` shared with render | **STILL OPEN** | `src/unified_graph.rs:10809, 14951, 15099` `event_queue.lock()` in eval path (guarded by `if let Ok`, so no panic, but Mutex + priority inversion) (F-8) |
| **R6** | Plugin path locks/inits/allocs during render | **STILL OPEN (worse than described)** | `src/unified_graph.rs:12122–12208` locks `real_plugins`/`VST3_LOAD_MUTEX` with `.unwrap()`, **loads+initializes the VST3 from disk on first render**, and allocates a `Vec<(String,f32)>` of cloned param names **per sample**; fundsp path `:13788–13882` locks + may recreate the unit (F-2) |
| **R7** | Ring starvation during compile window | **STILL OPEN (partially mitigated)** | Preload + modal `try_borrow`/skip reduce it; still bounded by single-threaded compile on a shared CPU. Not separately re-verified with a live device here |

Additional prior architectural notes still true: `unsafe impl Send/Sync` for `GraphCell` in
all four surfaces (`main.rs:927`, `modal_editor/mod.rs:58`, `bin/phonon-audio.rs:167`,
`live.rs:30`) and for `UnifiedSignalGraph` (`unified_graph.rs:4903–4904`); buffer-length
semantics (`process_buffer_dag` treats the slice as stereo-interleaved,
`buffer.len()/2` frames) unchanged.

**No prior finding was found to have silently regressed in behavior**, with the one
important exception that the reload-transfer improvements re-introduced the B7 borrow race
on the product surfaces (F-1). That is called out explicitly rather than hidden.

---

## 4. Findings (this audit) — full list

Severity legend: **crash** = can panic/kill audio thread or hard-stop output;
**glitch** = audible dropout/click/underrun; **degradation** = CPU/jitter/memory creep that
erodes headroom over time.

### F-1 — Synth-thread unconditional `borrow_mut()` panics during reload transfer  ·  **crash**
`src/main.rs:1006`, `src/bin/phonon-audio.rs:288` (synth); `src/main.rs:1107` &
`src/bin/phonon-audio.rs:531` (reload `try_borrow_mut`).

The reload thread now loads the **old** `GraphCell` and holds `try_borrow_mut()` across
`transfer_session_timing` + `transfer_fx_states` + `transfer_voice_manager(take_voice_manager())`
before the `ArcSwap::store`. During that window the graph pointer still points at the old
cell, so the synth thread's `graph.load()` returns that same cell and calls unconditional
`borrow_mut()` → `RefCell` is already mutably borrowed → **panic**, killing the synth thread;
the ring drains and audio stops permanently. Roles are reversed from prior B7 but the class
is identical, and it is now on the two most-used surfaces. The modal editor avoids this by
using `try_borrow_mut()` + skip on the synth side (`modal_editor/mod.rs:280,314`).

*Failure scenario:* live-edit a `phonon live` or `phonon-audio` session while a note/FX
transfer is in flight; if the transfer borrow overlaps a synth tick (≈ every 11.6 ms at
512 frames), the synth thread panics.

*Fix:* make the synth threads use `try_borrow_mut()` and skip-on-`Err` (mirror the modal
editor), or move to render-thread-owned graph with a message-based swap so no cross-thread
borrow exists. Do not "just shorten" the transfer — any overlap panics.

### F-2 — VST3 plugin load + per-sample allocation + `lock().unwrap()` in render  ·  **crash / glitch**
`src/unified_graph.rs:12122–12208` (plugin), `13788–13882` (fundsp).

Inside `eval_node` the plugin branch:
- `self.real_plugins.lock().unwrap()` (×3–4) — poison → panic on the render thread;
- on first hit, `create_real_plugin_by_name(plugin_id)` + `plugin.initialize(sr, 512)` —
  **loads and initializes a VST3 from disk on the render thread** (unbounded blocking);
- `let param_values: Vec<(String,f32)> = params.iter().map(|(n,s)| (n.clone(), ...)).collect()`
  — **heap allocation with String clones every sample, per plugin** (single-sample mode).

The fundsp path similarly `state.lock().unwrap()`s and may recreate the unit on parameter
change during render.

*Failure scenario:* any patch using a real VST3 (`vst3` feature) dropouts hard on first
note and jitters continuously; a panic in any plugin-holding thread poisons the mutex and
the next `.unwrap()` kills audio.

*Fix:* instantiate/initialize plugins at compile/reload; pre-resolve params to a scratch
`Vec` reused across samples; use `try_lock` with silence fallback; never `.unwrap()` a lock
on the render thread.

### F-3 — Per-node / clock `Mutex.lock().unwrap()` on the render thread  ·  **crash / degradation**
`src/bin/phonon-audio.rs:269` (GlobalClock, per buffer); `src/unified_graph.rs:14023–14033`
(Sample&Hold: `last_sample_cycle`/`last_sampled_value` are `Mutex<f32>` locked **4× per
sample** with `.unwrap()`); `:13788, 13834, 13872` (fundsp state).

These are both priority-inversion points (render thread blocks on a lock another thread may
hold) and panic points (`.unwrap()` on a poisoned mutex kills the synth thread). The
Sample&Hold case is egregious: per-node scalar state does not need a mutex at all, yet it is
locked four times every sample on the hot path.

*Fix:* store per-node state as plain fields (the graph is single-writer under the render
borrow); for GlobalClock use an atomics/double-buffered snapshot instead of `Mutex`. Replace
`.unwrap()` with `try_lock`/`if let Ok` everywhere on the render path.

### F-4 — Voice-pool heap growth + `eprintln!` on the synth thread  ·  **glitch**
`src/voice_manager.rs:905–955` (`grow_voice_pool`, called from `allocate_voice` `:1044–1045`).

When all voices are active a new trigger calls `grow_voice_pool`, which `self.voices.push(Voice::new())`
in a loop (heap allocation + possible full-vector reallocation/memcpy) **and** `eprintln!`s
the growth — all on the synth thread. `Voice` is a large struct (per-voice filter/env
state), so growth from e.g. 256→384 allocates and moves a substantial buffer mid-render.

*Failure scenario:* dense sample patterns (`s "bd*16"` layered, long tails, no cut groups)
exhaust the 256 initial pool; the growth allocation spikes render time and can underrun; the
`eprintln!` adds stderr backpressure exactly during the overload.

*Fix:* pre-grow the pool at compile/reload to a configured ceiling; on exhaustion steal
without allocating; count growths atomically and report off-thread.

### F-5 — Per-buffer allocation and `getenv` in `process_buffer_dag`  ·  **glitch / degradation**
`src/unified_graph.rs:7248` (`env::var("DEBUG_DAG")` every buffer), `7294` (`env::var("DEBUG_VOICE_BUFFERS")`),
plus `HashMap::new()`/`vec!`/`.collect()` in the DAG head and per-node/expression/voice
scratch allocated during render (prior R1 range).

`std::env::var` on glibc takes a global lock and linearly scans `environ` — a per-buffer
overhead paid unconditionally even when debugging is off. Combined with per-buffer container
allocation, complex graphs (many buses, dense patterns, arithmetic modulation) can push
render time past the buffer budget and underrun. The ring buffer hides but does not remove
this.

*Fix:* cache env flags once at graph build; compile an immutable DAG plan + reusable scratch
arena at compile/reload; reuse node/voice/expression buffers keyed by node id.

### F-6 — NaN guard covers output but not internal node state  ·  **degradation (stuck silence)**
Guard: `src/unified_graph.rs:7719–7724`, `17750–17753`. Gap: stateful nodes.

The output guard correctly stops NaN/Inf/denormals from reaching the ring (fixes the
"poisoned device" class). But if a resonant filter, FM feedback loop, delay/reverb, or
plugin drives its **internal state** to NaN/Inf, the state persists: the output guard zeros
the sample but the node keeps producing NaN → the affected voice/bus is stuck silent until
reload. Local sanitizers exist only in the DJ/SVF filter paths (`:15504–15533`, `:20268–20300`).
`process_sample` (mono) and `process_sample_multi` have no guard (offline/example paths, so
lower priority).

*Fix:* sanitize stateful node state on write in the common eval helpers (filters, delays,
feedback, oscillator phase), not just the final buffer; optionally reset a node to a safe
state when its output is caught non-finite.

### F-7 — `Box::leak` per parse accumulates unboundedly across a live session  ·  **degradation**
`src/compositional_parser.rs:586` and `:632` — two leaks per `parse_program`.

Every live edit/reload (all surfaces call `parse_program`) leaks the preprocessed and
expanded source strings. Over a long session of hundreds of edits this grows resident memory
without bound, eventually contributing to paging/instability.

*Fix:* give the parser an owned-string arena tied to the compiled graph's lifetime, or
restructure to borrow from a caller-owned `String` instead of leaking to `'static`.

### F-8 — MIDI monitoring `Mutex<VecDeque>` drained on the render thread  ·  **degradation**
`src/unified_graph.rs:10809, 14951, 15099` (`event_queue.lock()`), producer in
`src/midi_input.rs`.

Guarded by `if let Ok`, so no panic, but the render thread contends on a `Mutex` with the
MIDI input callback; under contention the render drains nothing (drops monitoring) or the
MIDI callback stalls (priority inversion).

*Fix:* bounded lock-free SPSC ring for MIDI events with a bounded per-buffer drain.

### F-9 — Modal editor I16 callback can `resize()` inside the callback  ·  **glitch (rare)**
`src/modal_editor/mod.rs:434–435`.

If CPAL ever requests a callback buffer larger than the pre-allocated 4096 frames,
`conversion_buffer.resize()` allocates inside the real-time callback.

*Fix:* size the conversion buffer from the stream's actual max buffer size, or clamp+fill
instead of resizing (as `src/live.rs:207` already does with `req = data.len().min(len)`).

### F-10 — `src/live.rs` `LiveSession` panicking reload path (latent)  ·  **crash (unreachable)**
`src/live.rs:284,288` (old-graph `borrow()`/`borrow_mut()` in `load_file`), `:108` (synth
`borrow_mut`), `:157–174` (error callback writes `/tmp/phonon_audio_errors.log` — file I/O in
the CPAL error handler).

`LiveSession` is not wired to any CLI command, so this is not an active crash — but it is a
`pub`, compiled, exported implementation of the exact B7 race using the *panicking*
`borrow()`/`borrow_mut()` (worse than the product surfaces' `try_borrow_mut`). It is a
copy-paste hazard and dead-but-compiled risk.

*Fix:* either delete `LiveSession`/`LiveRepl` (both appear superseded by the inline
`main.rs` path and the modal editor) or bring them up to the `try_borrow_mut`+skip pattern
and move the error-log write off the callback.

### F-11 — Noise nodes call `rand::thread_rng()` per sample  ·  **degradation**
`src/unified_graph.rs:10664, 10740, 10772` (and `:2812, 3080`).

White/pink/other noise fetch a thread-local RNG handle every sample in `eval_node`; the TLS
lookup + refcell of `thread_rng` is avoidable per-sample overhead in the hot path.

*Fix:* store a fast per-graph RNG (e.g. xorshift/PCG) in node state and advance it inline.

---

## 5. Shortlist — findings most likely to cause interactive instability

Ranked by likelihood-of-occurrence-during-live-use × severity. These are what a live
performer is most likely to actually hit:

1. **F-1 — synth `borrow_mut()` panic on reload** (`main.rs:1006`, `phonon-audio.rs:288`).
   *Why #1:* triggered by the core interactive action (editing while playing), on the two
   main surfaces, and the outcome is a hard audio stop. It is also a partial regression from
   the recent reload fixes, so it is both high-impact and freshly introduced. **Cheap to fix**
   (adopt the modal editor's `try_borrow_mut`+skip).

2. **F-2 — plugin load + per-sample alloc + `lock().unwrap()` in render**
   (`unified_graph.rs:12122–12208`). *Why #2:* any real-VST3 patch dropouts on first note and
   jitters forever; poison → panic. Only gated by the `vst3` feature, but catastrophic when used.

3. **F-3 — `.lock().unwrap()` on render-thread mutexes** (`phonon-audio.rs:269`,
   `unified_graph.rs:14023`, `13788`). *Why #3:* a single poisoned lock (from any panic,
   including F-1/F-2) cascades into a render-thread panic; Sample&Hold locks 4×/sample even in
   the happy path.

4. **F-4 — voice-pool heap growth + log on synth thread** (`voice_manager.rs:905–955`).
   *Why #4:* ordinary dense drum patterns exhaust 256 voices; the growth spike + stderr
   backpressure lands exactly when the engine is already loaded.

5. **F-5 — per-buffer alloc/`getenv` in `process_buffer_dag`** (`unified_graph.rs:7248+`).
   *Why #5:* steady jitter that erodes headroom on complex live patches; the most common cause
   of "it underruns when the patch gets big."

Secondary (real but lower interactive probability): F-6 (stuck-silence after a filter blows
up), F-7 (long-session memory creep), F-8 (MIDI monitoring contention), F-9 (rare I16
resize), F-11 (noise RNG overhead). F-10 is latent (unreachable) until someone wires
`LiveSession` back in.

---

## 6. Suggested Follow-up Tasks (not implemented here)

These map cleanly to independent, mostly single-file WG tasks:

1. **`fix-synth-thread-borrow-race`** (`src/main.rs`, `src/bin/phonon-audio.rs`) — synth
   threads use `try_borrow_mut`+skip. *Highest priority; small, testable.* (F-1)
2. **`fix-plugin-render-path`** (`src/unified_graph.rs`) — pre-instantiate plugins on
   reload; reuse a param scratch `Vec`; `try_lock` + silence fallback; no `.unwrap()`. (F-2)
3. **`derefcell-render-locks`** (`src/unified_graph.rs`, `src/bin/phonon-audio.rs`) — remove
   `Mutex<f32>` per-node state; atomics/snapshot for GlobalClock; ban `.lock().unwrap()` on
   render. (F-3)
4. **`preallocate-voice-pool`** (`src/voice_manager.rs`) — grow at compile/reload, steal
   without alloc, off-thread growth reporting. (F-4)
5. **`dag-scratch-arena`** (`src/unified_graph.rs`) — cache env flags, compiled DAG plan,
   reusable scratch. (F-5)
6. **`sanitize-node-state`** (`src/unified_graph.rs`) — flush non-finite stateful node state
   in the common eval helpers. (F-6)
7. **`parser-arena-no-leak`** (`src/compositional_parser.rs`) — remove the two `Box::leak`s. (F-7)
8. **`retire-or-harden-livesession`** (`src/live.rs`) — delete or bring to the safe pattern. (F-10)

A `tests/audio_live_edit_glitch_harness.rs` extension should add: a reload-while-rendering
stress loop that would surface F-1 (drive `phonon live` / `phonon-audio` reload paths, not
just the modal editor), and an allocation-counting guard around the render thread for
F-2/F-4/F-5.

---

## Validation Checklist (self-audit against task acceptance criteria)

- [x] Report exists at `docs/audits/rt-safety-2026-07.md`.
- [x] Audio callback path traced end-to-end (device callback entry → graph eval → output)
      with file:line references — §2, for all four surfaces.
- [x] Every finding has severity + suggested fix — §3 table + §4 F-1..F-11 + §6.
- [x] Prior audit findings explicitly reconciled fixed / still-open / regressed — §3
      (B1–B8, R1–R7), including the explicit F-1 regression note.
- [x] Explicit shortlist of findings most likely to cause interactive instability — §5.
- [x] No behavioral code changes made (investigation only).
