# Realtime Audio And Reload Lifecycle Audit

Date: 2026-05-24

Task: `audio-realtime-reload-audit`

Scope: audio callbacks, background audio render loops, render-loop helpers that can be reached from live audio, and state replacement during live edit/reload/relaunch. This is a research artifact only; no behavioral code changes were made.

## Executive Summary

The current audio engine has moved most expensive synthesis out of the CPAL callback for the primary live paths, but there are still several realtime risks:

- Some CPAL callbacks still lock mutexes, allocate, log, and optionally perform file I/O.
- The background synth thread is on the effective realtime path. It still allocates heavily in `UnifiedSignalGraph::process_buffer_dag`, voice rendering, plugin handling, pattern precomputation, expression evaluation, and some node variants.
- Graph sharing relies on `unsafe impl Send/Sync` for `RefCell<UnifiedSignalGraph>` and `UnifiedSignalGraph`. Several reload paths borrow old graph state from the control thread while the synth thread can borrow the same graph, producing either a panic or a skipped render depending on the frontend.
- Reload semantics are fragmented across `src/live.rs`, the inline `Commands::Live` path in `src/main.rs`, `src/modal_editor/mod.rs`, and `src/bin/phonon-audio.rs`. They differ on timing continuity, sample preloading, FX transfer, voice transfer, and stale ring-buffer behavior.
- NaN and denormal handling is partial. Some filters sanitize state, but there is no universal finite/denormal guard at graph output, ring-buffer write, or CPAL output conversion.

The highest-risk fixes are:

1. Make all CPAL callbacks lock-free, allocation-free, and I/O-free.
2. Replace `RefCell` graph sharing and cross-thread old-graph borrowing with a single audio/render owner and message-based graph swaps.
3. Remove per-buffer/per-sample allocations from the default DAG render path by precomputing plans and reusing scratch storage.
4. Move plugin loading/initialization/parameter lookup and sample loading misses out of the render path.
5. Unify reload semantics across live frontends.

## Entry Point Inventory

This section enumerates every CPAL callback or render-loop entry point found by searching `src/` and `examples/` for `build_output_stream`, `process_buffer`, `process_buffer_at`, `process_sample`, `process_sample_multi`, `render`, and adjacent render-loop calls.

### CPAL Output Callbacks

| Entry point | Path | What runs on callback | Notes |
| --- | --- | --- | --- |
| Legacy mixer callback | `src/audio.rs:93`-`src/audio.rs:100` | Locks `Arc<Mutex<Mixer>>`, calls `Mixer::process_audio`. | Direct realtime lock. `Mixer::process_audio` can allocate voices at `src/audio.rs:123`-`src/audio.rs:143`. |
| Scheduler callback | `src/engine.rs:127`-`src/engine.rs:135` | Locks scheduler and sample bank, calls `Scheduler::process_audio`. | Direct realtime locks. `Scheduler::process_audio` can allocate a new voice at `src/engine.rs:313`-`src/engine.rs:320` and logs at `src/engine.rs:329`-`src/engine.rs:331`. |
| File live session F32 callback | `src/live.rs:160`-`src/live.rs:188` | Reads ring buffer and fills underruns with silence. | No synthesis in callback, but logs underruns every 100 callbacks at `src/live.rs:177`-`src/live.rs:183`. |
| File live session I16 callback | `src/live.rs:191`-`src/live.rs:225` | Reads ring buffer, allocates temporary `Vec<f32>`, converts to i16. | Allocates in callback at `src/live.rs:198` and `src/live.rs:205`; logs underruns at `src/live.rs:215`-`src/live.rs:219`. |
| Inline `Commands::Live` callback | `src/main.rs:1023`-`src/main.rs:1052` | Reads ring buffer and fills underruns with silence. | Uses `static mut` underrun counter and `unsafe` at `src/main.rs:1040`-`src/main.rs:1047`; logs from callback. |
| Modal editor F32 callback | `src/modal_editor/mod.rs:382`-`src/modal_editor/mod.rs:412` | Optionally drains ring on graph clear, reads ring buffer, counts underruns atomically. | Best callback shape found in product paths: no direct synthesis, allocation, logging, or locks in the F32 path. |
| Modal editor I16 callback | `src/modal_editor/mod.rs:420`-`src/modal_editor/mod.rs:470` | Uses a captured conversion buffer, may resize, reads ring buffer, converts to i16. | Preallocates before callback at `src/modal_editor/mod.rs:415`-`src/modal_editor/mod.rs:418`, but can still allocate if callback buffer exceeds capacity at `src/modal_editor/mod.rs:433`-`src/modal_editor/mod.rs:435`. |
| Standalone audio process F32 callback | `src/bin/phonon-audio.rs:334`-`src/bin/phonon-audio.rs:370` | Reads ring buffer; on underrun writes silence; optionally records WAV samples. | Uses `static mut`/`unsafe` underrun counter at `src/bin/phonon-audio.rs:350`-`src/bin/phonon-audio.rs:356`. Recording locks a mutex and writes samples in callback at `src/bin/phonon-audio.rs:359`-`src/bin/phonon-audio.rs:365`. |
| Standalone audio process I16 callback | `src/bin/phonon-audio.rs:373`-`src/bin/phonon-audio.rs:435` | Allocates temp `Vec<f32>`, optionally records WAV samples, converts to i16. | Allocates at `src/bin/phonon-audio.rs:380` and `src/bin/phonon-audio.rs:398`; locks/writes recording output at `src/bin/phonon-audio.rs:383`-`src/bin/phonon-audio.rs:410`; uses `static mut`/`unsafe` at `src/bin/phonon-audio.rs:423`-`src/bin/phonon-audio.rs:428`. |
| Example live playground callback | `examples/live_playground.rs:245`-`examples/live_playground.rs:265` | Locks `LiveState`, calls `graph.process_sample()` once per frame. | Example/demo path, but it is a direct realtime lock and per-sample graph render. |
| Example phonon live callback | `examples/phonon_live.rs:440`-`examples/phonon_live.rs:448` | Locks `LiveState`, calls `graph.process_sample()` once per frame. | Example/demo path. |
| Example phonon poll callback | `examples/phonon_poll.rs:455`-`examples/phonon_poll.rs:464` | Locks `LiveState`, calls `graph.process_sample()` once per frame. | Example/demo path. |
| Example live callback | `examples/live.rs:454`-`examples/live.rs:465` | Locks `UnifiedSignalGraph`, calls `graph.process_sample()` once per frame. | Example/demo path. |

All product CPAL error handlers found either log through `log`/`tracing` or write an error file. The file-writing handlers are in `src/live.rs:139`-`src/live.rs:156`, `src/modal_editor/mod.rs:352`-`src/modal_editor/mod.rs:370`, and `src/bin/phonon-audio.rs:307`-`src/bin/phonon-audio.rs:325`.

### Background Synth And Render Loops

These are not CPAL callbacks, but they feed the callback and therefore determine audible underruns/glitches.

| Entry point | Path | What runs | Notes |
| --- | --- | --- | --- |
| File live synth thread | `src/live.rs:75`-`src/live.rs:135` | Renders 512-sample chunks with `graph_cell.0.borrow_mut().process_buffer(&mut buffer)` at `src/live.rs:100`, then pushes to ring. | Uses `RefCell` borrow that can panic if reload borrows same graph. Profiling/logging can run in this loop at `src/live.rs:107`-`src/live.rs:124`. |
| Inline live synth thread | `src/main.rs:982`-`src/main.rs:1017` | Renders 512-sample chunks with `borrow_mut().process_buffer` at `src/main.rs:998`. | Same `RefCell` risk; no reload state transfer. |
| Modal editor synth thread | `src/modal_editor/mod.rs:235`-`src/modal_editor/mod.rs:350` | Renders configured buffer size through `try_borrow_mut().process_buffer` at `src/modal_editor/mod.rs:279`-`src/modal_editor/mod.rs:284`. | Avoids borrow panic by skipping renders, but skips can drain the ring. Logs stats/peaks/skips at `src/modal_editor/mod.rs:246`-`src/modal_editor/mod.rs:258`, `src/modal_editor/mod.rs:289`-`src/modal_editor/mod.rs:302`, and `src/modal_editor/mod.rs:321`-`src/modal_editor/mod.rs:326`. |
| Standalone audio process synth thread | `src/bin/phonon-audio.rs:249`-`src/bin/phonon-audio.rs:305` | Locks `GlobalClock`, renders with `process_buffer_at`, pushes to ring. | `GlobalClock` lock at `src/bin/phonon-audio.rs:260`-`src/bin/phonon-audio.rs:263`; render call at `src/bin/phonon-audio.rs:280`-`src/bin/phonon-audio.rs:285`. |
| Modal test harness render loops | `src/modal_editor/test_harness.rs:279`, `src/modal_editor/test_harness.rs:390` | Calls `graph.process_buffer` for tests/headless harness. | Not a device callback, but exercises the same render path. |
| Profiling/perf binaries | `src/bin/profile_synthesis.rs:42`, `src/bin/profile_synthesis.rs:59`, `src/bin/profile_synthesis.rs:89`, `src/bin/phonon-perf.rs:138`, `src/bin/test_arch_audio.rs:41` | Calls `graph.process_buffer`. | Offline/profiling only, not callback-owned. |
| CLI render/profile path | `src/main.rs:460`, `src/main.rs:496` | Calls `my_graph.process_buffer`/`graph.process_buffer`. | Offline/profile path. |

### Graph Render Entry Points

| Entry point | Path | Realtime relevance |
| --- | --- | --- |
| `UnifiedSignalGraph::process_buffer_at` | `src/unified_graph.rs:17965`-`src/unified_graph.rs:17977` | Main live render entry for `src/bin/phonon-audio.rs`; timing supplied by `GlobalClock`. |
| `UnifiedSignalGraph::process_buffer` | `src/unified_graph.rs:17982`-`src/unified_graph.rs:17993` | Main live render entry for `src/live.rs`, `src/main.rs`, and modal editor. |
| `UnifiedSignalGraph::process_buffer_internal` | `src/unified_graph.rs:17995`-`src/unified_graph.rs:18045` | Shared block render. Initializes sample node timing, clears cache, checks `ENABLE_HYBRID_ARCH`, then calls DAG renderer. |
| `UnifiedSignalGraph::process_buffer_dag` | `src/unified_graph.rs:7240`-`src/unified_graph.rs:7765` | Default block render path. Contains many per-buffer allocations, env-var checks, debug logging, and graph traversal. |
| `UnifiedSignalGraph::eval_node_buffer_dag` | `src/unified_graph.rs:7791`-`src/unified_graph.rs:7952` | Per-node DAG evaluation. Allocates a cache buffer and a `newly_triggered_voices` vector; calls per-sample `eval_node`. |
| `UnifiedSignalGraph::process_buffer_hybrid` | enabled by `src/unified_graph.rs:18037`-`src/unified_graph.rs:18039` | Disabled by default and commented as having timing bugs at `src/unified_graph.rs:18033`-`src/unified_graph.rs:18036`; if enabled it is realtime. |
| `UnifiedSignalGraph::process_sample` | `src/unified_graph.rs:17571`-`src/unified_graph.rs:17668` | Older per-sample render path used by examples and some offline paths. |
| `UnifiedSignalGraph::process_sample_stereo` | `src/unified_graph.rs:17686`-`src/unified_graph.rs:17741` | Per-sample stereo variant. |
| `UnifiedSignalGraph::process_sample_multi` | `src/unified_graph.rs:9614`-`src/unified_graph.rs:9675` | Per-sample multi-output variant used by `render_stereo`; allocates an output vector and processes voice state each sample. |
| `UnifiedSignalGraph::process_buffer_stereo` | `src/unified_graph.rs:17743`-`src/unified_graph.rs:17763` | Block wrapper over `process_sample_stereo`. |
| `UnifiedSignalGraph::precompute_pattern_events` | `src/unified_graph.rs:17765`-`src/unified_graph.rs:17854` | Called by DAG render each buffer. Queries patterns and mutates event contexts. |
| `UnifiedSignalGraph::process_pattern_events_event_driven` | `src/unified_graph.rs:17864`-`src/unified_graph.rs:17952` | Trigger-list builder used from block render logic. |
| `UnifiedSignalGraph::eval_signal_buffer` | `src/unified_graph.rs:21913`-`src/unified_graph.rs:21975` | Buffer evaluation for signals. Pattern signals still loop per sample. |
| `UnifiedSignalGraph::eval_expression_buffer` | `src/unified_graph.rs:21977`-`src/unified_graph.rs:22084` | Allocates operand buffers for every arithmetic expression. |
| `UnifiedSignalGraph::render` | `src/unified_graph.rs:18417`-`src/unified_graph.rs:18427` | Offline mono helper that allocates a stereo buffer and calls `process_buffer`. Not a callback, but it exercises the live block renderer. |
| `UnifiedSignalGraph::render_stereo` | `src/unified_graph.rs:18432`-`src/unified_graph.rs:18449` | Offline stereo helper that loops through `process_sample_multi`. Not callback-owned. |
| `AudioNodeGraph::process_buffer` | `src/audio_node_graph.rs:244`-`src/audio_node_graph.rs:287` | Older graph architecture block entry. |
| `AudioNodeGraph::process_buffer_multi_output` | `src/audio_node_graph.rs:292`-`src/audio_node_graph.rs:319` | Allocates channel buffers. |
| `AudioNodeGraph::render` | `src/audio_node_graph.rs:330`-`src/audio_node_graph.rs:345` | Offline render helper with allocation. |
| WAV/offline renderer | `src/render.rs:148`-`src/render.rs:173`, `src/render.rs:237`-`src/render.rs:299` | Offline file I/O and rendering; not realtime. |
| `SimpleDspExecutor::render` | `src/simple_dsp_executor.rs:359`-`src/simple_dsp_executor.rs:383` | Legacy/offline executor render path. No CPAL caller found in this audit. |
| `SimpleDspExecutorV2::render` / `render_stereo` | `src/simple_dsp_executor_v2.rs:359`-`src/simple_dsp_executor_v2.rs:419` | Legacy/offline executor path; sample-by-sample and allocates output vectors. No CPAL caller found. |
| `SignalExecutor::render` | `src/signal_executor.rs:339`-`src/signal_executor.rs:348` | Older block-render helper over `process_block`. No CPAL caller found. |

`src/osc_live_server.rs:169`-`src/osc_live_server.rs:215` can parse OSC live commands into a new `UnifiedSignalGraph`, hush, or panic graph, but no product CPAL callback or audio render loop calls this module directly in the searched tree. It is treated here as a control-only graph construction path rather than a realtime entry point.

## Blocking Or Allocation-Prone Operations On Realtime Paths

The following are the blocking, allocation-prone, logging, I/O, unsafe, or shared-mutable operations found on callback or render-feed paths.

### Direct CPAL Callback Risks

| Severity | Source | Operation | Suspected audible symptom | Recommended fix shape |
| --- | --- | --- | --- | --- |
| High | `src/audio.rs:95`-`src/audio.rs:99` | CPAL callback locks `Mutex<Mixer>`. | Dropout if another thread holds the mixer lock; callback priority inversion. | Replace with lock-free command queue into a preallocated mixer state owned only by the callback/render thread, or retire this legacy engine. |
| High | `src/audio.rs:123`-`src/audio.rs:143` | `Mixer::process_audio` drains pending commands and can `Vec::push` a new voice. | Sporadic click/dropout when voice pool grows on callback. | Preallocate voice slots; reject/steal voices without heap growth. |
| High | `src/engine.rs:129`-`src/engine.rs:132` | CPAL callback locks scheduler and sample bank. | Dropout if scheduling or loading code holds either lock. | Audio thread owns scheduler snapshot and sample-bank references; producers enqueue commands lock-free. |
| High | `src/engine.rs:313`-`src/engine.rs:320` | Scheduler allocates a new `Voice` during callback render. | First trigger after capacity exhaustion can click/drop. | Preallocate voice pool. |
| Medium | `src/engine.rs:329`-`src/engine.rs:331` | Logging from callback render when triggering/missing samples. | Jitter or stalls under logger backpressure. | Count atomically and report off-thread. |
| High | `src/live.rs:198`, `src/live.rs:205` | I16 callback allocates temp vectors. | Periodic allocator stalls on non-F32 devices. | Capture a preallocated conversion buffer sized from the stream config; never resize in callback. |
| Medium | `src/live.rs:177`-`src/live.rs:183`, `src/live.rs:215`-`src/live.rs:219` | Underrun logging in callback. | Additional jitter during an underrun cascade. | Atomic counter only; UI/control thread drains diagnostics. |
| Medium | `src/main.rs:1040`-`src/main.rs:1047` | Inline live callback uses `static mut` underrun counter and logs. | Undefined data-race risk if callback is invoked concurrently by host; logging jitter. | Replace with `AtomicUsize`; move reporting off callback. |
| Medium | `src/modal_editor/mod.rs:433`-`src/modal_editor/mod.rs:435` | I16 callback can resize conversion buffer. | Rare allocation spike if backend chooses a larger buffer than 4096 frames. | Size conversion buffer from the actual maximum/stream config, or allocate a fixed upper bound and truncate/fill safely. |
| High | `src/bin/phonon-audio.rs:359`-`src/bin/phonon-audio.rs:365`, `src/bin/phonon-audio.rs:383`-`src/bin/phonon-audio.rs:410` | Recording path locks a WAV writer and writes samples from CPAL callback. | Dropouts while recording, especially on slow disks or flush pressure. | Move recording to a non-realtime writer thread fed by a lock-free SPSC ring. |
| High | `src/bin/phonon-audio.rs:380`, `src/bin/phonon-audio.rs:398` | I16 callback allocates temp vectors. | Allocation stalls on I16 devices. | Same preallocated conversion-buffer fix as live/modal. |
| Medium | `src/bin/phonon-audio.rs:350`-`src/bin/phonon-audio.rs:356`, `src/bin/phonon-audio.rs:423`-`src/bin/phonon-audio.rs:428` | `static mut` underrun counter and callback logging. | Data-race risk and callback jitter. | Use atomics and off-thread reporting. |
| Medium | `src/live.rs:139`-`src/live.rs:156`, `src/modal_editor/mod.rs:352`-`src/modal_editor/mod.rs:370`, `src/bin/phonon-audio.rs:307`-`src/bin/phonon-audio.rs:325` | CPAL error callback opens/appends a file. | Error handling can block on filesystem. It is not the sample callback, but still runs in audio backend context. | Use nonblocking diagnostics queue or best-effort atomic error state; write file from control thread. |
| Medium | `examples/live_playground.rs:247`-`examples/live_playground.rs:264`, `examples/phonon_live.rs:442`-`examples/phonon_live.rs:448`, `examples/phonon_poll.rs:457`-`examples/phonon_poll.rs:464`, `examples/live.rs:456`-`examples/live.rs:465` | Examples lock state/graph and call per-sample render inside callback. | Demo glitches; copied patterns can reintroduce product bugs. | Mark examples as non-realtime-safe or update them to the ring-buffer architecture. |

### Render-Feed Thread Risks

| Severity | Source | Operation | Suspected audible symptom | Recommended fix shape |
| --- | --- | --- | --- | --- |
| High | `src/live.rs:100`, `src/main.rs:998` | Synth thread calls `RefCell::borrow_mut()` on graph loaded from `ArcSwap`. | Panic during reload if control thread borrows the same old graph; process termination or audio stop. | Single-owner render thread receives graph-swap messages and performs transfer/swap at buffer boundary. |
| Medium | `src/modal_editor/mod.rs:279`-`src/modal_editor/mod.rs:329` | Modal uses `try_borrow_mut()` and skips render if reload holds the graph. | Ring underrun or brief gap during live edit if transfer/preload takes too long. | Same owner-thread swap. If keeping this design, transfer from immutable snapshots without borrowing render-owned state. |
| Medium | `src/bin/phonon-audio.rs:260`-`src/bin/phonon-audio.rs:263` | Synth thread locks `GlobalClock` once per buffer. | Render stalls if IPC/control thread holds clock lock; likely short but priority inversion remains. | Store timing in atomics or a lock-free double-buffered clock snapshot. |
| Medium | `src/live.rs:107`-`src/live.rs:124`, `src/modal_editor/mod.rs:246`-`src/modal_editor/mod.rs:258`, `src/modal_editor/mod.rs:289`-`src/modal_editor/mod.rs:326`, `src/bin/phonon-audio.rs:269`-`src/bin/phonon-audio.rs:294` | Synth/render-feed loops do env-var checks, timing, and `eprintln!` diagnostics. | Extra jitter; if stderr blocks, ring can starve. | Convert to counters/timestamps stored atomically and drained by UI/logger thread. Cache env flags outside loop. |
| High | `src/unified_graph.rs:7246`-`src/unified_graph.rs:7249`, `src/unified_graph.rs:7458`-`src/unified_graph.rs:7499`, `src/unified_graph.rs:7502`-`src/unified_graph.rs:7528` | Default DAG render checks env vars, builds dependency/topological/batch structures, and allocates temporary maps/vectors per buffer. | CPU spikes, underruns, timing jitter on complex graphs. | Compile immutable DAG plan at graph build/reload time; keep per-render scratch arenas. |
| High | `src/unified_graph.rs:7259`-`src/unified_graph.rs:7262`, `src/unified_graph.rs:7537`-`src/unified_graph.rs:7545`, `src/unified_graph.rs:7590`-`src/unified_graph.rs:7609`, `src/unified_graph.rs:7717`-`src/unified_graph.rs:7752` | Buffer caches and node outputs allocate/resize during render. | First render after graph/buffer-size change can spike; repeated node buffer allocation causes steady pressure. | Allocate all scratch buffers when graph or buffer size changes; reuse by node id. |
| High | `src/unified_graph.rs:7285`-`src/unified_graph.rs:7288`, `src/voice_manager.rs:1832`-`src/voice_manager.rs:1895` | Voice buffer rendering allocates `VoiceBuffers` and per-voice `Vec`s. | Underruns under high sample voice counts. | Preallocate voice output buffers per active node/voice pool and reuse. |
| High | `src/unified_graph.rs:7310`-`src/unified_graph.rs:7447`, `src/voice_manager.rs:1921`-`src/voice_manager.rs:1929` | Synthesis voice path allocates `HashMap`s, state snapshots, synth buffers, and per-sample maps. | Heavy spikes with bus-triggered synth voices/chords. | Compile per-synth voice scratch and state storage; avoid cloning node chains per buffer. |
| Medium | `src/unified_graph.rs:17788`-`src/unified_graph.rs:17850`, `src/unified_graph.rs:17872`-`src/unified_graph.rs:17935` | Pattern precompute allocates `HashMap` controls, event vectors, context strings, trigger list, and sorts. | Jitter on dense patterns or many sample/pattern nodes. | Reuse event/trigger buffers; avoid string context insertion in audio render; precompile deterministic pattern schedules where possible. |
| Medium | `src/unified_graph.rs:17608`-`src/unified_graph.rs:17612`, `src/unified_graph.rs:17673`-`src/unified_graph.rs:17681`, `src/unified_graph.rs:17719`-`src/unified_graph.rs:17721` | Per-sample paths collect output channels and bus names into vectors. | Severe overhead in example/per-sample realtime paths. | Do not use per-sample path in realtime callbacks; if retained, cache output/bus id lists. |
| Medium | `src/unified_graph.rs:7819`-`src/unified_graph.rs:7821`, `src/unified_graph.rs:7841`-`src/unified_graph.rs:7843` | Per-node DAG evaluation inserts a new cache vector and allocates trigger tracking. | Per-node allocation pressure. | Reuse node cache buffers and a fixed trigger-index scratch vector. |
| High | `src/unified_graph.rs:21962`-`src/unified_graph.rs:21967`, `src/unified_graph.rs:21983`-`src/unified_graph.rs:22084` | Buffer signal/expression evaluation can fall back to per-sample signal eval and allocates operand buffers for each expression. | CPU spikes with arithmetic-heavy modulation. | Use scratch-buffer stack/arena per render pass; compile expression graph to reusable buffers. |
| Medium | `src/unified_graph.rs:10652`-`src/unified_graph.rs:10657` | White noise calls `rand::thread_rng()` in sample eval. | Thread-local/RNG overhead in sample-rate path; possible jitter. | Store a fast per-graph RNG state and advance it without TLS lookup/allocation risk. |
| Medium | `src/unified_graph.rs:10797`-`src/unified_graph.rs:10828`, `src/unified_graph.rs:14939`-`src/unified_graph.rs:15005`, `src/midi_input.rs:197`-`src/midi_input.rs:216` | MIDI callback and graph render share a `Mutex<VecDeque<...>>`. Render drains the queue while MIDI callback pushes. | Render stalls if MIDI callback holds lock; MIDI callback drops monitoring updates if render holds lock. | Bounded lock-free MIDI ring or double-buffered queue with bounded per-buffer drain. |
| Medium | `src/unified_graph.rs:15046`-`src/unified_graph.rs:15068` | `MidiPolySynth` can push a new voice when no inactive voice exists. | Allocation on first polyphony expansion; possible click/dropout. | Preallocate max polyphony or steal/reuse voices without growing. |
| High | `src/unified_graph.rs:13769`-`src/unified_graph.rs:13861` | Fundsp unit path allocates `input_values`, locks state, can recreate unit on parameter changes, then locks again for `tick`. | Blocking/jitter, state discontinuity on parameter modulation. | Store inputs in scratch storage; make Fundsp state render-thread-owned; smooth/reconfigure outside realtime path. |
| High | `src/unified_graph.rs:18855`-`src/unified_graph.rs:18891`, `src/unified_graph.rs:18903`-`src/unified_graph.rs:19008`, `src/unified_graph.rs:19017`-`src/unified_graph.rs:19088` | Plugin path locks plugin managers, may initialize/load plugins, queries parameter info, allocates MIDI/output/input vectors, and logs errors while rendering. | Large dropout on first plugin use/reload; repeated jitter from locks/parameter lookup/allocation. | Instantiate and initialize plugins during compile/reload; pre-resolve parameter handles; preallocate process buffers/events; use render-thread-owned plugin instances or try-lock fallback silence. |
| High | `src/plugin_host/real_plugin.rs:128`-`src/plugin_host/real_plugin.rs:186` | Real plugin wrapper allocates MIDI conversion vectors and silent input buffers during process. | Plugin render jitter. | Keep per-plugin scratch buffers and MIDI event storage. |
| High | `src/unified_graph.rs:14500`-`src/unified_graph.rs:14513`, `src/unified_graph.rs:5552`-`src/unified_graph.rs:5600` | Sample node calls `sample_bank.borrow_mut().get_sample(...)`; preload exists to avoid disk I/O but only finds discovered pattern names. Dynamic names or preload misses can still load in render path. | Big dropout on first sample hit or dynamic sample change. | Make render path cache-only. Compile/reload must preload all statically known samples and schedule async preload for dynamic misses, returning silence until ready. |
| Medium | `src/unified_graph.rs:15492`-`src/unified_graph.rs:15525`, `src/unified_graph.rs:20250`-`src/unified_graph.rs:20285` | DJ filter paths flush denormals and finite-check state/output. This is positive but local. | Other nodes can still propagate NaN/Inf/denormals into output, plugins, filters, or ring buffer. | Add graph-output finite and denormal sanitizer; audit stateful nodes for finite inputs/state. |
| Medium | `src/unified_graph.rs:7673`-`src/unified_graph.rs:7715` | Output mix/limiter clamps/saturates but does not explicitly handle NaN before ring write. | NaN can survive into callback output; some hosts/devices produce clicks or silence after NaN poisoning. | `if !sample.is_finite() || sample.abs() < DENORMAL_THRESHOLD { sample = 0.0 }` at final graph output and CPAL conversion. |
| Medium | `src/voice_manager.rs:1442`-`src/voice_manager.rs:1489`, `src/voice_manager.rs:1659`-`src/voice_manager.rs:1714`, `src/voice_manager.rs:1755`-`src/voice_manager.rs:1815`, `src/voice_manager.rs:2147`-`src/voice_manager.rs:2221` | Voice manager paths allocate `Vec`s/`HashMap`s and can spawn scoped rayon work for high counts. | CPU spikes and scheduling jitter under dense voices. | Reuse voice buffers; measure whether rayon work belongs on audio render thread; consider fixed worker pool or single-thread deterministic path for live. |

## Reload And Relaunch Lifecycle

### Shared Graph Model

The live frontends use `ArcSwap<Option<GraphCell>>`, where `GraphCell` wraps `RefCell<UnifiedSignalGraph>`. The wrappers mark `RefCell` and graph state as `Send`/`Sync` with unsafe implementations:

- File live wrapper: `src/live.rs:26`-`src/live.rs:30`
- Inline live wrapper: `src/main.rs:923`-`src/main.rs:927`
- Modal editor wrapper: `src/modal_editor/mod.rs:55`-`src/modal_editor/mod.rs:59`
- Standalone audio process wrapper: `src/bin/phonon-audio.rs:160`-`src/bin/phonon-audio.rs:167`
- `UnifiedSignalGraph` itself: `src/unified_graph.rs:4896`-`src/unified_graph.rs:4904`

The safety comments claim each graph instance is accessed by one thread at a time. That is not consistently true during state transfer: reload code loads the old `Arc`, borrows it, and reads/mutates state while the synth thread can still have loaded the same `Arc` and be rendering it.

### `src/live.rs` File Watch Reload

Reload flow:

1. Polls metadata every 100 ms in `LiveSession::run` at `src/live.rs:325`-`src/live.rs:343`.
2. `load_file` reads the file at `src/live.rs:255`-`src/live.rs:257`.
3. Parses and compiles a new graph at `src/live.rs:259`-`src/live.rs:271`.
4. Enables wall-clock timing at `src/live.rs:273`-`src/live.rs:274`.
5. Loads the old graph and transfers timing, FX state, and voice manager at `src/live.rs:278`-`src/live.rs:285`.
6. Stores the new graph with `ArcSwap` at `src/live.rs:287`-`src/live.rs:290`.

Risks:

- High: `old_graph_cell.0.borrow()` and `borrow_mut()` at `src/live.rs:280`-`src/live.rs:284` can panic if the synth thread is in `borrow_mut().process_buffer` at `src/live.rs:100`.
- High: `borrow_mut().take_voice_manager()` mutates the old graph while a previously loaded snapshot can still be rendered. If it succeeds between synth buffers, stale snapshots after the transfer can render with a fresh/empty voice manager.
- Medium: Samples are not preloaded in this path, unlike modal editor. Sample misses can reach `get_sample` in render.
- Medium: Ring buffer is not drained on reload. This smooths transitions but can play old-code audio after the swap for up to the ring duration (`src/live.rs:66`-`src/live.rs:70`, one second).
- Medium: FX transfer is partial; see "State Transfer Fidelity" below.
- Low/Medium: OSC command graph construction in `src/osc_live_server.rs:169`-`src/osc_live_server.rs:215` has no state transfer if a caller swaps its returned graph into an audio loop later; no such direct product caller was found.

Suspected audible symptoms: panic/audio stop during unlucky reload, beat jumps on failed/partial transfer, delayed transition, stale old pattern after save, missing-sample dropout after reload.

### Inline `Commands::Live` Reload In `src/main.rs`

Reload flow:

1. Initial load reads/parses file and stores graph at `src/main.rs:965`-`src/main.rs:970`.
2. Synth thread renders old graph snapshots at `src/main.rs:982`-`src/main.rs:1017`.
3. Poll loop checks file metadata every 100 ms at `src/main.rs:1060`-`src/main.rs:1106`.
4. On change, it reads file, compiles, and stores the new graph at `src/main.rs:1080`-`src/main.rs:1089`.

Risks:

- High: No timing transfer, FX transfer, voice transfer, or sample preload. Live edits reset graph timing/state.
- High: Same unsafe `RefCell` sharing model as `src/live.rs`, though this path does not borrow old graph during reload.
- Medium: One-second ring buffer (`src/main.rs:932`-`src/main.rs:937`) means old audio can continue after reload.
- Medium: CPAL callback uses `static mut` underrun counter and logs.

Suspected audible symptoms: beat restart/jump on every save, FX tails cut, active voices cut, stale old-code audio after save, callback jitter on underruns.

### Modal Editor Reload

Reload flow:

1. `load_code` parses and compiles at `src/modal_editor/mod.rs:632`-`src/modal_editor/mod.rs:659`.
2. Enables wall-clock timing at `src/modal_editor/mod.rs:672`-`src/modal_editor/mod.rs:675`.
3. If there is an old graph, retries `try_borrow_mut()` up to 50 times with 500 us sleeps at `src/modal_editor/mod.rs:681`-`src/modal_editor/mod.rs:738`.
4. Transfers timing, FX state, and voice manager at `src/modal_editor/mod.rs:708`-`src/modal_editor/mod.rs:724`.
5. If transfer fails, logs that the new graph starts with fresh timing at `src/modal_editor/mod.rs:740`-`src/modal_editor/mod.rs:752`.
6. Preloads samples before swap at `src/modal_editor/mod.rs:756`-`src/modal_editor/mod.rs:758`.
7. Stores the new graph at `src/modal_editor/mod.rs:760`-`src/modal_editor/mod.rs:763`.
8. Explicitly does not clear the ring on live-code reload at `src/modal_editor/mod.rs:765`-`src/modal_editor/mod.rs:769`; hush/panic do clear it at `src/modal_editor/mod.rs:2267`-`src/modal_editor/mod.rs:2282`.

Risks:

- Medium: This is the safest reload path found, but it still depends on cross-thread `RefCell` borrowing. It avoids panic but can skip synth renders while waiting.
- Medium: If the old graph remains busy for roughly 25 ms, state transfer fails and timing can restart.
- Medium: The ring is intentionally not cleared on reload. At the configured ring size (`src/modal_editor/mod.rs:224`-`src/modal_editor/mod.rs:229`), old-code audio may play for roughly 200 ms.
- Medium: `transfer_voice_manager` does not truly let all old voices continue. It releases synthesis and sample voices before inserting the old manager into the new graph at `src/unified_graph.rs:5627`-`src/unified_graph.rs:5635`.
- Medium: The reload code claims "active voices continue playing" at `src/modal_editor/mod.rs:722`-`src/modal_editor/mod.rs:724`, but the transfer implementation releases them. This mismatch can produce surprise fade-outs or clicks depending on voice envelope state.

Suspected audible symptoms: occasional short gap on reload under load, old audio tail after edit, active sample/synth tails fade/cut despite comments, beat jump if transfer retries fail.

### Standalone `phonon-audio` IPC Reload/Relaunch

Reload flow:

1. Audio process creates a two-second ring buffer at `src/bin/phonon-audio.rs:240`-`src/bin/phonon-audio.rs:244`.
2. Synth thread reads timing from `GlobalClock` and renders with `process_buffer_at` at `src/bin/phonon-audio.rs:260`-`src/bin/phonon-audio.rs:285`.
3. IPC loop receives coalesced messages at `src/bin/phonon-audio.rs:450`-`src/bin/phonon-audio.rs:488`.
4. On `UpdateGraph`, it parses/compiles, updates the `GlobalClock` CPS under lock at `src/bin/phonon-audio.rs:472`-`src/bin/phonon-audio.rs:478`, and swaps the graph at `src/bin/phonon-audio.rs:485`-`src/bin/phonon-audio.rs:487`.
5. On `Hush`/`Panic`, it sets graph to `None` at `src/bin/phonon-audio.rs:500`-`src/bin/phonon-audio.rs:508`.

Risks:

- High: No FX state transfer, voice transfer, or sample preload on graph update.
- Medium: Timing continuity is better than other paths because `GlobalClock` is the single timing source, but the clock is protected by a `Mutex`.
- Medium: `Hush`/`Panic` do not drain the ring. With a two-second ring, stale old audio can continue after silence commands until consumed.
- Medium: `IpcMessage::receive_coalesced` drops non-`UpdateGraph` messages encountered while draining stale updates at `src/ipc.rs:143`-`src/ipc.rs:190`, especially `src/ipc.rs:166`-`src/ipc.rs:173`. A `Hush` or `Panic` behind rapid updates can be ignored.
- Low/Medium: `AudioServer::new` removes an old socket before binding at `src/ipc.rs:207`-`src/ipc.rs:218`; relaunching loses in-memory graph/plugin/sample/FX/voice state unless the pattern client reconnects and resends code.

Suspected audible symptoms: stale audio after hush/panic, FX tails cut on update, first-hit sample/plugin dropout after update, emergency commands ignored during rapid live edits, complete state loss on audio process relaunch.

### State Transfer Fidelity

Timing transfer is relatively comprehensive in `UnifiedSignalGraph::transfer_session_timing`: it transfers wall-clock session start, adjusts `cycle_offset`, clones `cycle_bus_cache`, updates sample/pattern/plugin trigger state, and logs the transfer at `src/unified_graph.rs:5647`-`src/unified_graph.rs:5748`.

Voice transfer is intentionally destructive: `transfer_voice_manager` releases synthesis and sample voices before installing the old manager at `src/unified_graph.rs:5627`-`src/unified_graph.rs:5635`. That prevents unbounded accumulation but means reload cannot preserve arbitrary active voices click-free.

FX extraction covers a broad set of stateful effects beginning at `src/unified_graph.rs:5750`, but injection is narrower:

- Transferred: Delay, Reverb, Chorus, Flanger, Compressor, Limiter, LowPass, HighPass, BandPass at `src/unified_graph.rs:7987`-`src/unified_graph.rs:8164`.
- Counted but not transferred: TapeDelay, MultiTapDelay, PingPongDelay, DattorroReverb, Convolution, SidechainCompressor, Expander, MoogLadder at `src/unified_graph.rs:8165`-`src/unified_graph.rs:8199`.

Suspected audible symptoms: some effects keep tails across reload while others cut/reset, creating inconsistent live-edit behavior that depends on effect type.

## Ranked Fix Plan

The ranking combines risk reduction and implementation cost. "Cost" assumes current architecture and no broad rewrite unless stated.

| Rank | Fix | Risk reduction | Cost | Why this order |
| --- | --- | --- | --- | --- |
| 1 | Remove locks, allocation, logging, and file I/O from CPAL callbacks. Cover `src/live.rs` I16, `src/bin/phonon-audio.rs` recording/I16, modal I16 resize, legacy `src/audio.rs`, and `src/engine.rs`. | Very high | Low to medium | Direct callback violations are the most deterministic source of device underruns and are localized. |
| 2 | Replace cross-thread `RefCell` graph access with render-thread-owned graph state and message-based swap/transfer at buffer boundaries. | Very high | Medium to high | Eliminates panic/stale-reference class and makes reload transfer deterministic. |
| 3 | Precompute DAG plan and reusable render scratch on compile/reload. Remove per-buffer dependency/topology rebuilds, node-output allocations, voice-buffer allocations, expression operand allocations, and pattern trigger allocations. | Very high | High | The default render path allocates enough that ring-buffering hides but does not solve realtime instability. |
| 4 | Move plugin loading/initialization/parameter lookup and plugin scratch allocation out of render. | High | Medium | First plugin hit can block for far longer than an audio buffer; fixes are clear and testable. |
| 5 | Make sample access render-path cache-only. Preload static samples during compile/reload and perform dynamic sample loads asynchronously. | High | Medium | Disk I/O on first sample miss is catastrophic and already acknowledged by `preload_samples`. |
| 6 | Unify live reload semantics across `src/live.rs`, `src/main.rs`, `src/modal_editor/mod.rs`, and `src/bin/phonon-audio.rs`. | High | Medium | Current behavior depends on entry point; downstream harnesses will otherwise chase inconsistent glitches. |
| 7 | Fix IPC coalescing and hush/panic ring clearing in `phonon-audio`. | Medium to high | Low | Emergency silence must have priority and should not wait behind a two-second ring. |
| 8 | Add global finite/denormal sanitation at graph output and CPAL conversion boundaries; audit stateful nodes for finite state updates. | Medium | Low | Cheap containment for NaN/Inf/denormal failures while deeper node audits continue. |
| 9 | Replace shared MIDI monitoring mutex with a bounded lock-free queue and bounded render drain. | Medium | Low to medium | Avoids render/MIDI callback priority inversion. |
| 10 | Update examples to avoid teaching lock-and-render-in-callback patterns or label them explicitly as non-realtime-safe. | Low to medium | Low | Reduces regression/copy-paste risk. |

## Suggested Implementation Follow-Ups

These are intentionally scoped for later tasks; they should not be implemented in this audit task.

1. Callback cleanup task: make CPAL callbacks allocation-free and I/O-free, with a validation harness that scans for `Vec` allocation, `Mutex::lock`, `eprintln!`, and writer calls inside callback closures.
2. Reload ownership task: prototype render-thread-owned graph swap in one primary frontend, preferably modal editor or `phonon-audio`, then retire duplicated inline live reload logic.
3. DAG scratch task: add reusable per-graph scratch buffers for `process_buffer_dag`, `eval_expression_buffer`, pattern triggers, and voice outputs.
4. Plugin/sample preload task: make plugins and sample cache misses impossible in the render path; return silence or previous stable state until async preparation completes.
5. IPC emergency task: preserve and prioritize `Hush`, `Panic`, and `Shutdown` while coalescing graph updates; drain or explicitly fade ring buffers on emergency silence.

## Validation Checklist

- Every CPAL callback found in `src/` and `examples/` is listed in "CPAL Output Callbacks".
- Every live render-loop entry point found in `src/` is listed in "Background Synth And Render Loops" or "Graph Render Entry Points"; offline/control-only render APIs are either listed there or explicitly marked as not callback-owned.
- Blocking/allocation-prone realtime operations found during the scan are listed in "Blocking Or Allocation-Prone Operations On Realtime Paths" with source references, severity, audible symptom, and fix shape.
- Reload/relaunch state swaps are described for `src/live.rs`, inline `Commands::Live`, modal editor, and `phonon-audio`, including timing, FX, voice, sample, stale-ring, IPC, race, and stale-reference risks.
- Concrete fixes are ranked by risk reduction and implementation cost.
- No behavioral code changes were made for this task.
