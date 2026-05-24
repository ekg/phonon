# Phonon Audio Architecture Map

Source-grounded research artifact for `audio-architecture-map`.

This document maps what this repository is, where Phonon code enters the runtime, how parsed code becomes audio, how live edit/reload paths swap state, and where audio-quality or edit-stability risk is concentrated. Source links point to the first line of the cited range in this worktree.

Scope note: this task is documentation-only; no runtime, synthesis, parser, scheduler, callback, or test behavior is changed by this artifact.

## Repository Purpose

Phonon is a Rust live-coding language for pattern-driven synthesis. The crate metadata describes it as a "Rust-based live coding language combining TidalCycles patterns with modular synthesis" in [`Cargo.toml:1-7`](../../Cargo.toml#L1). The README describes the main model as "Live coding synthesis + patterns in pure Rust" where patterns are signals and everything is represented in one signal graph, with pure Rust audio output through CPAL in [`README.md:1-20`](../../README.md#L1) and [`README.md:316-341`](../../README.md#L316).

Important runtime dependencies match that purpose:

- CPAL for audio device output, [`Cargo.toml:15-27`](../../Cargo.toml#L15).
- `ringbuf` for producer/consumer audio buffering, [`Cargo.toml:15-27`](../../Cargo.toml#L15).
- `arc-swap` for live graph replacement, [`Cargo.toml:15-27`](../../Cargo.toml#L15).
- `hound` for WAV render/playback paths, [`Cargo.toml:52`](../../Cargo.toml#L52).
- `notify` is present as a dependency, although the observed live paths poll modification time directly, [`Cargo.toml:73-74`](../../Cargo.toml#L73).

The library root describes the conceptual feature set and exports the modules that make up the audio path. The high-level doc comments list mini-notation, sample playback, synthesis, effects, and deterministic offline rendering in [`src/lib.rs:8-24`](../../src/lib.rs#L8). The exported modules include the parser/compiler, live/editor/runtime paths, IPC, render support, sample loading, unified graph, and voice manager in [`src/lib.rs:320-391`](../../src/lib.rs#L320).

## Main Entry Points And How To Exercise Them

The CLI is defined with `clap` in [`src/main.rs:11-20`](../../src/main.rs#L11). Its subcommands are declared in [`src/main.rs:23-170`](../../src/main.rs#L23), and `main()` dispatches them after setting logging and Rayon thread pool behavior in [`src/main.rs:211-230`](../../src/main.rs#L211).

Exercise paths:

| Surface | Command shape | What it exercises | Key source |
| --- | --- | --- | --- |
| Offline render | `cargo run -- render input.ph output.wav --duration 10` | Reads file/stdin/inline code, parses, compiles, renders buffers or samples, writes WAV. | [`src/main.rs:237-337`](../../src/main.rs#L237), [`src/main.rs:359-552`](../../src/main.rs#L359), [`src/main.rs:665-700`](../../src/main.rs#L665) |
| Quick playback | `cargo run -- play input.ph --duration 4` | Parses and compiles, renders to `/tmp/phonon_play.wav`, then shells out to an available system player. | [`src/main.rs:734-855`](../../src/main.rs#L734) |
| File live coding | `cargo run -- live live.ph` | Polls a file, recompiles on content changes, swaps a graph behind a background synthesis thread and CPAL callback. | [`src/main.rs:869-1106`](../../src/main.rs#L869) |
| Modal editor | `cargo run -- edit [file] --buffer-size 512` | Full-screen editor with live graph reload, chunk/all evaluation, hush/panic, ring-buffer synthesis, and CPAL output. | [`src/main.rs:1120-1124`](../../src/main.rs#L1120), [`src/modal_editor/mod.rs:156-229`](../../src/modal_editor/mod.rs#L156) |
| Two-process audio server | `cargo run --bin phonon-audio -- --record /tmp/out.wav` | Separate audio process listening on Unix socket; receives DSL text via IPC, compiles in the audio process, synthesizes through ring buffer and CPAL. | [`src/bin/phonon-audio.rs:1-11`](../../src/bin/phonon-audio.rs#L1), [`src/bin/phonon-audio.rs:170-245`](../../src/bin/phonon-audio.rs#L170), [`src/ipc.rs:15-46`](../../src/ipc.rs#L15) |
| Disabled REPL | `cargo run -- repl` | Instantiates `LiveRepl`, but the REPL path reports that it uses the old scheduling engine and is disabled. | [`src/main.rs:1109-1118`](../../src/main.rs#L1109), [`src/live.rs:382-404`](../../src/live.rs#L382) |

The README lists similar user-facing commands for build, live, and render in [`README.md:11-20`](../../README.md#L11) and CLI usage examples in [`README.md:203-217`](../../README.md#L203).

## High-Level Runtime Shape

At a high level, Phonon code travels through this path:

```text
.ph text / inline code / editor buffer / IPC UpdateGraph
  -> compositional_parser::parse_program
  -> compositional_compiler::compile_program
  -> UnifiedSignalGraph
  -> process_sample_stereo, process_buffer, or process_buffer_at
  -> VoiceManager, SynthVoiceManager, SampleBank, graph node evaluation
  -> offline WAV buffer or live ring buffer
  -> CPAL output callback or WAV writer/system player
```

Where code enters the engine depends on the surface:

- Render reads from a file, stdin, or inline `--code`, then calls `parse_program` and `compile_program`, [`src/main.rs:237-337`](../../src/main.rs#L237).
- Play reads a source file, parses and compiles, then calls `graph.render`, [`src/main.rs:734-800`](../../src/main.rs#L734).
- `phonon live` reads the live file initially and on change, parses and compiles in the foreground reload loop, and swaps the new graph into `ArcSwap`, [`src/main.rs:952-979`](../../src/main.rs#L952), [`src/main.rs:1060-1106`](../../src/main.rs#L1060).
- The modal editor compiles the full buffer or a chunk in `load_code`, then swaps the graph after state-transfer work, [`src/modal_editor/mod.rs:628-770`](../../src/modal_editor/mod.rs#L628), [`src/modal_editor/mod.rs:2093-2198`](../../src/modal_editor/mod.rs#L2093).
- The two-process server receives an `IpcMessage::UpdateGraph { code }`, then parses and compiles the code inside `phonon-audio`, [`src/ipc.rs:15-46`](../../src/ipc.rs#L15), [`src/bin/phonon-audio.rs:450-520`](../../src/bin/phonon-audio.rs#L450).

## Parser And Compiler

`src/compositional_parser.rs` is the source of truth for the Phonon text syntax. Its top-level comments describe a compositional parser with buses, expressions, pattern and macro support in [`src/compositional_parser.rs:1-16`](../../src/compositional_parser.rs#L1). The core AST statement variants include bus assignments, templates, patterns, output statements, tempo/BPM, mix mode, function definitions, hush/unhush/panic, cycle controls, nudge, and buffer size in [`src/compositional_parser.rs:42-90`](../../src/compositional_parser.rs#L42). Expression variants include numbers, strings, bus/template/pattern references, calls, bus calls, chains, transforms, binary operators, and function definitions in [`src/compositional_parser.rs:92-156`](../../src/compositional_parser.rs#L92).

`parse_program` preprocesses multiline input, leaks the preprocessed string to obtain a `'static` lifetime, manually parses statements, and returns the remaining input plus a vector of statements in [`src/compositional_parser.rs:583-613`](../../src/compositional_parser.rs#L583). The parser has dedicated forms for bus assignment in [`src/compositional_parser.rs:691-764`](../../src/compositional_parser.rs#L691), output statements in [`src/compositional_parser.rs:803-821`](../../src/compositional_parser.rs#L803), chains in [`src/compositional_parser.rs:968-993`](../../src/compositional_parser.rs#L968), and `$` transforms/function application in [`src/compositional_parser.rs:1166-1275`](../../src/compositional_parser.rs#L1166).

The compiler's top comment says it compiles to the AudioNode architecture, but the current compile-time switch is set to the legacy graph path: `USE_AUDIO_NODES` is `false` in [`src/compositional_compiler.rs:169-171`](../../src/compositional_compiler.rs#L169). In practice, `CompilerContext` owns both a `UnifiedSignalGraph` and an `AudioNodeGraph`, but normal statements are compiled into `UnifiedSignalGraph` nodes and buses through the legacy signal-node path, [`src/compositional_compiler.rs:180-220`](../../src/compositional_compiler.rs#L180), [`src/compositional_compiler.rs:309-333`](../../src/compositional_compiler.rs#L309).

`compile_program` makes two passes: it pre-registers bus placeholders, compiles all statements, then auto-routes output through `master`, `out`, `dN/outN`, or all buses mixed when no explicit output exists, [`src/compositional_compiler.rs:509-585`](../../src/compositional_compiler.rs#L509). Bus assignments compile modifier and transform buses specially, otherwise compile expressions into signal nodes and add them to the graph, [`src/compositional_compiler.rs:633-700`](../../src/compositional_compiler.rs#L633). Output statements compile expressions and set graph outputs in [`src/compositional_compiler.rs:734-755`](../../src/compositional_compiler.rs#L734). Tempo/BPM and buffer-size statements update graph timing/configuration in [`src/compositional_compiler.rs:758-789`](../../src/compositional_compiler.rs#L758).

Important expression lowering points:

- Numbers compile to constant nodes; string literals compile through mini-notation pattern parsing, [`src/compositional_compiler.rs:886-903`](../../src/compositional_compiler.rs#L886).
- Bus references handle self-reference as a unit delay and otherwise resolve MIDI, effect/modifier, or ordinary buses, [`src/compositional_compiler.rs:906-962`](../../src/compositional_compiler.rs#L906).
- Zero-argument oscillator calls become 1 Hz modulators, [`src/compositional_compiler.rs:1045-1057`](../../src/compositional_compiler.rs#L1045).
- Sample playback through `s "pattern"` is parsed, transformed, and lowered to `SignalNode::Sample`, [`src/compositional_compiler.rs:2477-2582`](../../src/compositional_compiler.rs#L2477), [`src/compositional_compiler.rs:2831-2908`](../../src/compositional_compiler.rs#L2831).
- Synth patterns are lowered through `compile_synth_pattern` into `SignalNode::SynthPattern`, [`src/compositional_compiler.rs:3661-3697`](../../src/compositional_compiler.rs#L3661), [`src/compositional_compiler.rs:4955-5029`](../../src/compositional_compiler.rs#L4955).

## Synthesis Engine And Scheduler

The central runtime object is `UnifiedSignalGraph`. Its state includes graph nodes, buses, outputs, hush state, sample rate, session timing, wall-clock timing controls, CPS, pattern/effect caches, the sample bank, sample voice manager, synthesis voice manager, MIDI/poly-synth state, DAG buffer caches, and feedback/output history, [`src/unified_graph.rs:4678-4825`](../../src/unified_graph.rs#L4678). `UnifiedSignalGraph::new` initializes a default 48 kHz graph, `use_wall_clock = false`, `cps = 0.5`, `buffer_size = 512`, sample and synth voice managers, a sample bank, and a master limiter ceiling of 0.95 in [`src/unified_graph.rs:5004-5057`](../../src/unified_graph.rs#L5004).

Graph nodes are represented by `SignalNode` variants. Runtime-stateful examples include oscillators with phase state in [`src/unified_graph.rs:686-695`](../../src/unified_graph.rs#L686), pattern nodes in [`src/unified_graph.rs:1004-1010`](../../src/unified_graph.rs#L1004), sample nodes in [`src/unified_graph.rs:1028-1048`](../../src/unified_graph.rs#L1028), synth pattern nodes in [`src/unified_graph.rs:1050-1067`](../../src/unified_graph.rs#L1050), and MIDI poly-synth state in [`src/unified_graph.rs:1097-1115`](../../src/unified_graph.rs#L1097).

There are three render entry styles:

- `process_sample` and `process_sample_stereo` evaluate one sample at a time, update cycle position, process active voices, evaluate output nodes, mix outputs, update feedback/previous values, and increment sample count, [`src/unified_graph.rs:17571-17668`](../../src/unified_graph.rs#L17571), [`src/unified_graph.rs:17686-17740`](../../src/unified_graph.rs#L17686).
- `process_buffer` computes timing from the graph's own wall-clock or cached cycle position, then calls `process_buffer_internal`, [`src/unified_graph.rs:17979-17993`](../../src/unified_graph.rs#L17979).
- `process_buffer_at` receives timing from an external clock and then calls `process_buffer_internal`, [`src/unified_graph.rs:17954-17977`](../../src/unified_graph.rs#L17954).

The default buffer path is the DAG renderer, not the hybrid branch. `process_buffer_internal` initializes sample timing, clears and enables buffer caches, chooses `process_buffer_dag` unless `ENABLE_HYBRID_ARCH` is enabled, and disables caches afterward, [`src/unified_graph.rs:17995-18044`](../../src/unified_graph.rs#L17995). `process_buffer_dag` treats the input buffer as stereo interleaved by computing `buffer_size = buffer.len() / 2`, updates cached cycle position, initializes sample-node timing, processes sample voices, precomputes pattern events, renders synthesis voice buffers, builds a dependency graph and topological batches, processes the required bus/output nodes, copies/mixes output buffers to stereo interleaved output, applies hard limiting, crossfades buffer boundaries, updates previous-output buffers, and increments `sample_count`, [`src/unified_graph.rs:7240-7753`](../../src/unified_graph.rs#L7240).

The scheduler is cycle-based. The graph carries CPS and cycle position. In the two-process server, `GlobalClock` is the external timing owner: it stores sample rate, sample count, session start, cycle offset, and CPS, then returns buffer-start cycle and sample increment for `process_buffer_at`, [`src/bin/phonon-audio.rs:80-157`](../../src/bin/phonon-audio.rs#L80). In in-process live/editor paths, the graph itself owns wall-clock or cached timing, and live reload code can transfer timing between old and new graphs with `transfer_session_timing`, [`src/unified_graph.rs:5647-5693`](../../src/unified_graph.rs#L5647).

Voice and sample execution is split out:

- `VoiceManager` owns sample and synthesis voice state. Voice records include sample/synth node ID, playback position, gain, pan, speed, source node, envelope state, buffer trigger offset, and fadeout state, [`src/voice_manager.rs:205-286`](../../src/voice_manager.rs#L205).
- The default voice pool is 256 voices and can grow up to 4096, [`src/voice_manager.rs:92-99`](../../src/voice_manager.rs#L92).
- Sample trigger allocation includes cut-group release and pool growth behavior, [`src/voice_manager.rs:957-1045`](../../src/voice_manager.rs#L957).
- `process_buffer_vec` renders sample voices into per-node buffers and uses Rayon above a threshold, [`src/voice_manager.rs:1832-1908`](../../src/voice_manager.rs#L1832).
- Synthesis voice buffers are processed with envelopes in [`src/voice_manager.rs:1921-1965`](../../src/voice_manager.rs#L1921).
- `SampleBank` creates search directories and preloads common samples, [`src/sample_loader.rs:259-309`](../../src/sample_loader.rs#L259), loads WAV files through `hound`, [`src/sample_loader.rs:342-386`](../../src/sample_loader.rs#L342), and resolves/caches sample lookups, including filesystem directory scanning for numbered samples, [`src/sample_loader.rs:389-450`](../../src/sample_loader.rs#L389).

## Audio Backend And Callback Paths

### Offline Render

The `render` subcommand parses and compiles the program, then either renders sample by sample or in blocks. The realtime-ish block path uses `BLOCK_SIZE = 512`, calls `graph.process_buffer`, and applies an output crossfade between blocks before writing WAV data through `hound`, [`src/main.rs:359-552`](../../src/main.rs#L359), [`src/main.rs:665-700`](../../src/main.rs#L665). The sequential branch uses one graph; the parallel branch clones the graph, seeks each chunk to the chunk start sample, renders, and crossfades chunks, [`src/main.rs:390-507`](../../src/main.rs#L390), [`src/main.rs:524-552`](../../src/main.rs#L524).

The `play` subcommand compiles the graph, calls `graph.render`, writes `/tmp/phonon_play.wav`, and then attempts `play`, `aplay`, `pw-play`, or `paplay`, [`src/main.rs:734-855`](../../src/main.rs#L734).

### `phonon live`

The inlined `Live` subcommand in `src/main.rs` opens the default CPAL host/device/config and records sample rate/channels, [`src/main.rs:895-907`](../../src/main.rs#L895). The code comment describes the intended split: a file watcher thread, a background synthesis thread, and an audio callback where the callback only reads precomputed samples, [`src/main.rs:910-918`](../../src/main.rs#L910).

State is shared as `ArcSwap<Option<GraphCell>>`, where `GraphCell` wraps `RefCell<UnifiedSignalGraph>` and declares `unsafe impl Send` and `unsafe impl Sync`, [`src/main.rs:923-930`](../../src/main.rs#L923). A one-second `HeapRb<f32>` ring buffer sits between the synthesis thread and the CPAL callback, [`src/main.rs:932-937`](../../src/main.rs#L932). The synthesis thread loads the current graph snapshot, borrows it mutably, calls `process_buffer` into a 512-element `f32` buffer, and pushes samples into the producer side of the ring, [`src/main.rs:982-1017`](../../src/main.rs#L982). The CPAL callback reads from the consumer side and fills silence on underrun, [`src/main.rs:1019-1054`](../../src/main.rs#L1019).

### Modal Editor

The modal editor uses the same broad design but with more reload-state handling. It creates a graph `ArcSwap`, underrun/synth/ring metrics, a clear-ring flag, and an approximately 200 ms ring buffer, [`src/modal_editor/mod.rs:210-229`](../../src/modal_editor/mod.rs#L210). The synthesis thread renders chunks of `synthesis_buffer_size` samples, logs performance once per second, tries to borrow the graph mutably, calls `process_buffer`, and pushes into the ring, [`src/modal_editor/mod.rs:231-350`](../../src/modal_editor/mod.rs#L231). The CPAL callback supports `F32` and `I16`, clears the ring when requested, converts to device format, and increments underrun counters, [`src/modal_editor/mod.rs:352-479`](../../src/modal_editor/mod.rs#L352).

### Two-Process Audio Server

`src/bin/phonon-audio.rs` is explicitly documented as a separate audio process: the pattern engine sends DSL code over a Unix socket, while the audio engine receives, compiles, synthesizes, and writes to speakers, [`src/bin/phonon-audio.rs:1-11`](../../src/bin/phonon-audio.rs#L1). It accepts optional recording, buffer-size environment configuration, opens a fixed-buffer CPAL stream, initializes `GlobalClock`, creates graph `ArcSwap`, and allocates a two-second ring buffer, [`src/bin/phonon-audio.rs:52-78`](../../src/bin/phonon-audio.rs#L52), [`src/bin/phonon-audio.rs:170-245`](../../src/bin/phonon-audio.rs#L170).

The synth thread asks `GlobalClock` for buffer timing, calls `graph.process_buffer_at`, and pushes the result into the ring, [`src/bin/phonon-audio.rs:246-305`](../../src/bin/phonon-audio.rs#L246). The CPAL callback drains the ring and optionally writes recording samples, [`src/bin/phonon-audio.rs:331-439`](../../src/bin/phonon-audio.rs#L331). The IPC loop receives coalesced messages, compiles `UpdateGraph` messages, updates tempo continuity in `GlobalClock`, swaps graphs, handles hush/panic by storing `None`, handles tempo changes, and exits on shutdown, [`src/bin/phonon-audio.rs:450-520`](../../src/bin/phonon-audio.rs#L450).

The IPC protocol sends DSL text, not serialized graph state, because the message comments say graph state contains non-serializable components and compilation is expected to be fast, [`src/ipc.rs:15-46`](../../src/ipc.rs#L15). The socket path is fixed at `/tmp/phonon.sock`, so the IPC layer documents that only one instance can run at a time, [`src/ipc.rs:195-199`](../../src/ipc.rs#L195).

### `src/live.rs`

`src/live.rs` contains a separate `LiveSession` path with the same ring-buffer architecture comment, [`src/live.rs:1-10`](../../src/live.rs#L1). It creates `GraphCell(RefCell<UnifiedSignalGraph>)`, `ArcSwap`, and a one-second ring buffer, [`src/live.rs:26-70`](../../src/live.rs#L26). Its synthesis thread calls `process_buffer`, and its CPAL callback drains the ring, [`src/live.rs:72-228`](../../src/live.rs#L72). `load_file` reads, parses, compiles, enables wall-clock timing, transfers session timing, FX states, and the voice manager, then stores the new graph, [`src/live.rs:251-301`](../../src/live.rs#L251). The file run loop polls file modification time and reloads on changes, [`src/live.rs:314-343`](../../src/live.rs#L314). The REPL wrapper is currently disabled, [`src/live.rs:382-404`](../../src/live.rs#L382).

## Live Edit, Relaunch, And Reload State Transitions

### `phonon live` file reload

`phonon live` creates a default live file if it does not exist, opens CPAL, creates graph/ring state, parses the initial file, and stores the initial graph, [`src/main.rs:883-979`](../../src/main.rs#L883). After starting the synth thread and CPAL stream, the main loop polls every 100 ms. On modification time and content change, it parses and compiles the new text and stores `Arc::new(Some(GraphCell(RefCell::new(new_graph))))` into `ArcSwap`, [`src/main.rs:1060-1106`](../../src/main.rs#L1060).

State transition:

```text
current graph in ArcSwap
  -> file content changes
  -> parse_program + compile_program create a fresh UnifiedSignalGraph
  -> graph.store(Some(new GraphCell))
  -> synth thread eventually loads new Arc snapshot
  -> old graph drops when outstanding Arc snapshots disappear
```

This path does not call `enable_wall_clock_timing`, `transfer_session_timing`, `transfer_fx_states`, `transfer_voice_manager`, or `preload_samples` around the swap in the observed source range, [`src/main.rs:952-979`](../../src/main.rs#L952), [`src/main.rs:1060-1106`](../../src/main.rs#L1060). It also does not clear the ring on normal reload.

### Modal editor eval/reload

The modal editor has three user-facing reload forms:

- `play_code` reloads the whole editor content through `load_code`, [`src/modal_editor/mod.rs:2093-2106`](../../src/modal_editor/mod.rs#L2093).
- `eval_chunk` extracts the current paragraph, treats a `hush` prefix specially, then reloads only that chunk through `load_code`, [`src/modal_editor/mod.rs:2109-2179`](../../src/modal_editor/mod.rs#L2109).
- `eval_all` reloads the full session through `load_code`, [`src/modal_editor/mod.rs:2181-2198`](../../src/modal_editor/mod.rs#L2181).

`load_code` parses, rejects leftover input, compiles, enables wall-clock timing, attempts to borrow the old graph up to 50 times with 0.5 ms sleeps, transfers timing, FX states, and voice manager, preloads samples, and stores the new graph in `ArcSwap`, [`src/modal_editor/mod.rs:628-770`](../../src/modal_editor/mod.rs#L628).

State transition:

```text
editor buffer/chunk
  -> parse_program with leftover check
  -> compile_program
  -> enable_wall_clock_timing
  -> borrow old graph if possible
  -> transfer session timing, FX state, voice manager
  -> preload samples
  -> graph.store(Some(new GraphCell))
```

`hush` and `panic` do a different state transition: they store `None` in the graph slot and set `should_clear_ring`, which causes the callback to drain the ring on the next callback pass, [`src/modal_editor/mod.rs:2267-2283`](../../src/modal_editor/mod.rs#L2267).

### Two-process IPC reload

The separate audio server starts a CPAL stream and sends `Ready` once playback is live, [`src/bin/phonon-audio.rs:441-448`](../../src/bin/phonon-audio.rs#L441). In the IPC loop, `receive_coalesced` drains older `UpdateGraph` messages so only the newest update is compiled, [`src/ipc.rs:143-190`](../../src/ipc.rs#L143). On `UpdateGraph`, the server parses and compiles the received DSL code, updates `GlobalClock` tempo while preserving current cycle position, and stores the new graph, [`src/bin/phonon-audio.rs:450-520`](../../src/bin/phonon-audio.rs#L450).

State transition:

```text
client sends DSL code
  -> length-prefixed bincode IPC message
  -> server coalesces update bursts
  -> parse_program + compile_program
  -> GlobalClock tempo/cycle continuity update
  -> graph.store(Some(new GraphCell))
```

`Hush` and `Panic` store `None`; `SetTempo` updates `GlobalClock`; `Shutdown` breaks the loop, [`src/bin/phonon-audio.rs:450-520`](../../src/bin/phonon-audio.rs#L450).

### `src/live.rs` file reload

The older `LiveSession::load_file` path reads the file, parses, compiles, enables wall-clock timing, transfers timing, FX states, and voice manager from the old graph, then swaps the new graph, [`src/live.rs:251-301`](../../src/live.rs#L251). It does not call `preload_samples` in that range. Its `run` loop polls modification time and calls `load_file` on changes, [`src/live.rs:314-343`](../../src/live.rs#L314).

## State Ownership Map

| State | Owner | Access pattern | Source |
| --- | --- | --- | --- |
| Parsed program AST | Stack/local values returned by parser | Created per parse, consumed by compiler. | [`src/compositional_parser.rs:583-613`](../../src/compositional_parser.rs#L583) |
| Compile context | `CompilerContext` | Local to compilation; accumulates buses, templates, functions, graph, synth library, MIDI queues. | [`src/compositional_compiler.rs:180-220`](../../src/compositional_compiler.rs#L180) |
| Runtime graph | `UnifiedSignalGraph` | Owns graph nodes, bus/output maps, timing, caches, sample bank, voice managers, limiter/crossfade state. | [`src/unified_graph.rs:4678-4825`](../../src/unified_graph.rs#L4678) |
| Live graph pointer | `ArcSwap<Option<GraphCell>>` | Reload thread stores a new graph; synthesis thread loads current snapshot. | [`src/main.rs:923-930`](../../src/main.rs#L923), [`src/modal_editor/mod.rs:55-106`](../../src/modal_editor/mod.rs#L55), [`src/bin/phonon-audio.rs:160-167`](../../src/bin/phonon-audio.rs#L160) |
| Graph interior mutability | `GraphCell(RefCell<UnifiedSignalGraph>)` | Synthesis thread borrows mutably while rendering; reload paths may inspect old graph for transfer. | [`src/main.rs:982-1017`](../../src/main.rs#L982), [`src/modal_editor/mod.rs:231-350`](../../src/modal_editor/mod.rs#L231) |
| Live audio buffer | `ringbuf::HeapRb<f32>` split producer/consumer | Synth thread writes generated samples; CPAL callback reads. | [`src/main.rs:932-937`](../../src/main.rs#L932), [`src/modal_editor/mod.rs:210-229`](../../src/modal_editor/mod.rs#L210), [`src/bin/phonon-audio.rs:237-245`](../../src/bin/phonon-audio.rs#L237) |
| External timing in two-process server | `Arc<Mutex<GlobalClock>>` | Synth thread locks to get buffer timing; IPC loop locks to update tempo. | [`src/bin/phonon-audio.rs:80-157`](../../src/bin/phonon-audio.rs#L80), [`src/bin/phonon-audio.rs:246-305`](../../src/bin/phonon-audio.rs#L246), [`src/bin/phonon-audio.rs:450-520`](../../src/bin/phonon-audio.rs#L450) |
| Samples | `SampleBank` inside `UnifiedSignalGraph` | Cached by sample name/index; can load WAV files and scan directories on demand. | [`src/unified_graph.rs:4678-4825`](../../src/unified_graph.rs#L4678), [`src/sample_loader.rs:389-450`](../../src/sample_loader.rs#L389) |
| Sample/synth voices | `VoiceManager` and `SynthVoiceManager` inside graph | Triggered by pattern nodes and rendered into per-node buffers. | [`src/unified_graph.rs:4678-4825`](../../src/unified_graph.rs#L4678), [`src/voice_manager.rs:205-286`](../../src/voice_manager.rs#L205), [`src/voice_manager.rs:1832-1965`](../../src/voice_manager.rs#L1832) |
| Hush/panic state | `UnifiedSignalGraph` plus graph pointer/ring clear in editor | Graph-level hush/panic can kill voices and outputs; editor also swaps graph to `None` and clears ring. | [`src/unified_graph.rs:9021-9059`](../../src/unified_graph.rs#L9021), [`src/modal_editor/mod.rs:2267-2283`](../../src/modal_editor/mod.rs#L2267) |

## Highest-Risk Audio Quality And Edit-Stability Areas

1. Inconsistent reload state transfer between live surfaces.

   `phonon live` recompiles and swaps a fresh graph without the timing/FX/voice/sample preload work used by the modal editor and `src/live.rs`, [`src/main.rs:952-979`](../../src/main.rs#L952), [`src/main.rs:1060-1106`](../../src/main.rs#L1060). By contrast, modal reload enables wall-clock timing, transfers session timing, FX states, voice manager, preloads samples, then stores the graph, [`src/modal_editor/mod.rs:628-770`](../../src/modal_editor/mod.rs#L628). This matters because live edits can otherwise reset phase/cycle continuity, drop tails or active voices, lose effect state, and trigger first-use sample loading after the swap.

2. `GraphCell` uses `RefCell<UnifiedSignalGraph>` with unsafe cross-thread `Send`/`Sync`.

   The in-process live paths and two-process server wrap `UnifiedSignalGraph` in `RefCell` and declare the wrapper `Send` and `Sync` manually, [`src/main.rs:923-930`](../../src/main.rs#L923), [`src/modal_editor/mod.rs:55-59`](../../src/modal_editor/mod.rs#L55), [`src/bin/phonon-audio.rs:160-167`](../../src/bin/phonon-audio.rs#L160), [`src/live.rs:26-30`](../../src/live.rs#L26). The main live and two-process synth loops use `borrow_mut` during rendering, [`src/main.rs:982-1017`](../../src/main.rs#L982), [`src/bin/phonon-audio.rs:246-305`](../../src/bin/phonon-audio.rs#L246), while the modal editor uses `try_borrow_mut` and skips if borrowed, [`src/modal_editor/mod.rs:231-350`](../../src/modal_editor/mod.rs#L231). This matters because the thread-safety contract is not enforced by the type system. Any overlapping borrow introduced by reload transfer, diagnostics, or future callback access could panic or violate the assumptions behind the unsafe impls.

3. Realtime callbacks still contain allocation, locking, or unsynchronized mutable state in some formats.

   The two-process callback allocates a temporary `Vec<f32>` in the `I16` path and may lock a WAV writer while the callback is running, [`src/bin/phonon-audio.rs:331-439`](../../src/bin/phonon-audio.rs#L331). `src/live.rs` similarly allocates a `Vec` in its `I16` callback path, [`src/live.rs:137-228`](../../src/live.rs#L137). `phonon live` and `phonon-audio` use `static mut UNDERRUN_COUNT` inside callbacks, [`src/main.rs:1019-1054`](../../src/main.rs#L1019), [`src/bin/phonon-audio.rs:331-439`](../../src/bin/phonon-audio.rs#L331). The modal editor preallocates its conversion buffer outside the callback, but can still resize it from inside the callback if needed, [`src/modal_editor/mod.rs:352-479`](../../src/modal_editor/mod.rs#L352). This matters because callback allocations, locks, and mutable statics increase underrun, jitter, and data-race risk exactly where timing tolerance is lowest.

4. Buffer length semantics are easy to misuse.

   Live synth threads create buffers described as chunks of samples, such as `[0.0f32; 512]` in `phonon live`, [`src/main.rs:982-1017`](../../src/main.rs#L982). The unified DAG renderer treats the buffer as stereo interleaved and sets `buffer_size = buffer.len() / 2`, [`src/unified_graph.rs:7240-7257`](../../src/unified_graph.rs#L7240). This matters because a caller passing "512 samples" as a 512-float buffer actually gives the graph 256 stereo frames. Ring-buffer sizing, timing increments, trigger offsets, and underrun diagnostics can drift if different layers mean frames, mono samples, or interleaved floats.

5. On-demand sample loading can occur on synthesis/reload-sensitive paths.

   `SampleBank::get_sample` can scan directories and load WAVs if a sample is not cached, [`src/sample_loader.rs:389-450`](../../src/sample_loader.rs#L389). The modal editor preloads samples before swapping a compiled graph, [`src/modal_editor/mod.rs:628-770`](../../src/modal_editor/mod.rs#L628), but `phonon live`, the two-process server, and `src/live.rs` do not call `preload_samples` in the cited reload path ranges, [`src/main.rs:952-979`](../../src/main.rs#L952), [`src/bin/phonon-audio.rs:450-520`](../../src/bin/phonon-audio.rs#L450), [`src/live.rs:251-301`](../../src/live.rs#L251). This matters because filesystem I/O and WAV decode work can appear on the synthesis side after an edit, causing audible stalls or first-trigger glitches.

6. Parser memory grows on repeated parse calls.

   `parse_program` preprocesses input and then `Box::leak`s the preprocessed string to satisfy parser lifetimes, [`src/compositional_parser.rs:583-613`](../../src/compositional_parser.rs#L583). Live surfaces call `parse_program` on every edit/reload, including `phonon live`, the modal editor, and the two-process server, [`src/main.rs:952-979`](../../src/main.rs#L952), [`src/modal_editor/mod.rs:628-770`](../../src/modal_editor/mod.rs#L628), [`src/bin/phonon-audio.rs:450-520`](../../src/bin/phonon-audio.rs#L450). This matters because a long editing session can accumulate leaked parse buffers, increasing memory pressure and eventually contributing to paging or process instability.

7. IPC update coalescing can discard non-update messages while draining update bursts.

   `receive_coalesced` keeps only the newest `UpdateGraph` message, but while draining it ignores non-`UpdateGraph` messages rather than preserving them for later dispatch, [`src/ipc.rs:143-190`](../../src/ipc.rs#L143). The IPC message set includes `Hush`, `Panic`, `SetTempo`, and `Shutdown`, [`src/ipc.rs:15-46`](../../src/ipc.rs#L15). This matters because during rapid edits, a safety or timing command sent near update bursts can be lost by the audio server, which directly affects edit/relaunch stability and emergency silence behavior.

8. Callback underrun accounting is not consistently atomic.

   The modal editor tracks underruns with atomics in its struct fields, [`src/modal_editor/mod.rs:61-106`](../../src/modal_editor/mod.rs#L61), but `phonon live` and `phonon-audio` use `static mut UNDERRUN_COUNT` inside callbacks, [`src/main.rs:1019-1054`](../../src/main.rs#L1019), [`src/bin/phonon-audio.rs:331-439`](../../src/bin/phonon-audio.rs#L331). This matters because underrun metrics are the primary signal for buffer health. Unsynchronized metrics can hide real timing problems or introduce undefined behavior risk if callback threading changes.

9. Hard limiting and crossfade mask discontinuities but can become audible artifacts.

   `process_buffer_dag` hard-clamps the final output to the master limiter ceiling, [`src/unified_graph.rs:7706-7714`](../../src/unified_graph.rs#L7706), then crossfades buffer boundaries using previous-buffer tails, [`src/unified_graph.rs:7717-7753`](../../src/unified_graph.rs#L7717). The render command also applies crossfades between rendered blocks/chunks, [`src/main.rs:524-552`](../../src/main.rs#L524). This matters because these mechanisms can reduce clicks, but hard clipping can distort and crossfades can smear transients or hide a deeper timing/state discontinuity across edits.

10. State transfer is best-effort and topology-dependent.

    `transfer_fx_states` extracts old effect states and reinjects them into matching new effect nodes by state key/bus relationship, [`src/unified_graph.rs:7956-8032`](../../src/unified_graph.rs#L7956). `transfer_voice_manager` releases old synthesis and sample voices with a quick fade before installing the old voice manager in the new graph, [`src/unified_graph.rs:5619-5635`](../../src/unified_graph.rs#L5619). `transfer_session_timing` enables wall-clock timing and adjusts cycle offset from old position/new CPS, [`src/unified_graph.rs:5647-5693`](../../src/unified_graph.rs#L5647). This matters because it is the right direction for stable live editing, but it depends on graph structure and timing assumptions. Renamed buses, changed output topology, or changed node identities can still drop tails, restart patterns, or leave state unmatched.

## Architecture Notes For Downstream Work

- The currently active compiler path is still `UnifiedSignalGraph`/`SignalNode`; `AudioNodeGraph` exists but is not the default compile target because `USE_AUDIO_NODES` is false, [`src/compositional_compiler.rs:169-171`](../../src/compositional_compiler.rs#L169).
- The most complete live reload path is currently the modal editor's `load_code`; it includes timing transfer, FX transfer, voice transfer, and sample preload before swapping, [`src/modal_editor/mod.rs:628-770`](../../src/modal_editor/mod.rs#L628).
- The highest-value stabilization target is consistency across live surfaces: `phonon live`, `src/live.rs`, modal editor, and `phonon-audio` all implement similar graph/ring/callback patterns with slightly different reload, callback, and buffer behavior.
- Any glitch harness should collect at least ring fill level, underrun count, graph swap count, reload parse/compile duration, and post-swap first-trigger timing. The modal editor already exposes atomics for underruns, synth performance, and ring fill, [`src/modal_editor/mod.rs:61-106`](../../src/modal_editor/mod.rs#L61).
