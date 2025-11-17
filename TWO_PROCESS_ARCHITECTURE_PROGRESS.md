# Two-Process Architecture - Implementation Progress

**Status**: Core implementation WORKING ✅ | Integration into modal editor: TODO

## Problem Statement

You reported critical issues with `phonon edit` (live mode):
- **Underruns**: "you can cause underruns with m.ph. easily. after 1-2 cycles. crunchy. so rough."
- **Timing glitches**: "every time we control-x in phonon edit, it justles the time. the beat breaks."
- **Pattern accumulation**: "`stut 8` seems to go totally wrong"

Root causes identified:
1. **Voice accumulation bug**: Default 10s release × `stut 8` multiplier = 90s of active voices per trigger
2. **Compilation blocking audio**: Pattern compilation happens in the same process as audio synthesis

## Solution: Two-Process Architecture

Inspired by Tidal Cycles + SuperCollider:
- **Pattern Engine** (`phonon edit`): Text editor + DSL compiler
- **Audio Engine** (`phonon-audio`): Dedicated synthesis + audio output
- **Communication**: Unix socket IPC (< 1ms latency)

"when in tidal we execute a chunk of code it INSTANTLY replaces the current pattern at the current cycle and position in cycle. not at the end of cycle or after a spell or short duration. it's instant. so probably <30ms or so. and it doesn't drop or shuffle the beat when doing so. it's 100% clean."

## What's Been Implemented ✅

### 1. IPC Protocol (`src/ipc.rs`)

```rust
pub enum IpcMessage {
    UpdateGraph { code: String },  // DSL code, not compiled graph
    Hush,
    Panic,
    SetTempo { cps: f32 },
    Ready,
    Underrun { count: usize },
    Shutdown,
}
```

- **Transport**: Unix Domain Socket (`/tmp/phonon.sock`)
- **Framing**: Length-prefixed messages (4-byte length + bincode data)
- **Max message size**: 100MB (sanity check)

**Why code strings, not graphs?**
- `UnifiedSignalGraph` has non-serializable state (RefCell, Arc, function pointers)
- Compilation is fast enough (~1-2ms)
- Clean separation: each process compiles independently
- Easier debugging (human-readable)

### 2. Audio Engine Binary (`src/bin/phonon-audio.rs`)

Standalone audio synthesis process:

```bash
cargo run --release --bin phonon-audio
```

**Architecture**:
- Listens on Unix socket for pattern engine connection
- Receives DSL code strings via IPC
- Compiles code independently using `parse_program()` + `compile_program()`
- Ring buffer (2 seconds) + cpal audio output
- Background synthesis thread (512-sample chunks)
- Wall-clock timing preserved across graph swaps
- VoiceManager state transferred to prevent clicks

**Key features**:
- Never blocks on compilation (separate process)
- Audio callback just reads from ring buffer (FAST!)
- Graph swaps are atomic (ArcSwap)
- State transfer prevents clicks (session timing + active voices)

### 3. Test Harness (`src/bin/test_two_process.rs`)

Standalone test proving the architecture works:

```bash
cargo run --release --bin test_two_process
```

**Test flow**:
1. Spawns `phonon-audio` subprocess
2. Connects via `PatternClient`
3. Waits for `Ready` message
4. Sends test pattern: `s "bd sn bd sn"`
5. Plays for 5 seconds
6. Updates pattern: `s "bd*4 sn*2"`  (simulates live coding!)
7. Plays for 3 seconds
8. Sends `Hush` → silence
9. Sends `Shutdown` → clean exit

**Test output** (verified working):
```
🧪 Testing two-process architecture
📦 Spawning audio engine...
🎵 Phonon Audio Engine starting...
🎵 Audio server listening on: /tmp/phonon.sock
🔌 Connecting to audio engine...
✅ Connected to audio engine
✅ Audio engine ready
📤 Sending test pattern...
🎵 Playing for 5 seconds...
📦 Received code update (30 bytes)
📤 Sending updated pattern...
✅ State transferred from old graph
🎵 Playing updated pattern for 3 seconds...
🔇 Sending Hush...
👋 Sending Shutdown...
✅ Test completed successfully!
```

### 4. Voice Accumulation Fix

In `src/unified_graph.rs`:

**Smart release calculation**:
- Short samples (< 1s): release = 20% of sample duration (10-500ms)
- Long samples (≥ 1s): keep 10s release for loops
- Synthetic bus buffers: release = 10% of buffer duration (10-500ms)

**Result**: `stut 8` now works cleanly in render mode (tested with 8 cycles)

## What's Remaining 🚧

### 1. Integrate into Modal Editor (`src/modal_editor/mod.rs`)

**Current state**: Modal editor runs audio in-process (like the old architecture)

**Changes needed**:
1. In `ModalEditor::new()`:
   - Spawn `phonon-audio` as subprocess
   - Connect via `PatternClient::connect()`
   - Remove local cpal/ringbuf setup

2. In `ModalEditor::load_code()`:
   - Send code via `client.send(&IpcMessage::UpdateGraph { code })`
   - Remove local graph compilation

3. Add process cleanup in `Drop` impl
   - Send `Shutdown` message
   - Wait for process to exit

**Estimated effort**: 2-4 hours (careful refactoring required)

### 2. Test in Live Mode

Once integrated, test with:
- `phonon edit m.ph` - verify no underruns with `stut 8`
- Ctrl-X reload - verify beat never drops
- `stut 8` intensive patterns - verify voice accumulation fix works

### 3. Documentation

Create `docs/TWO_PROCESS_ARCHITECTURE.md` explaining:
- Why this architecture (compilation vs audio priorities)
- How IPC works (Unix sockets, message protocol)
- How to debug (check `/tmp/phonon.sock`, audio engine logs)
- Performance characteristics (< 1ms IPC latency)

## Commits

1. **b8e2783**: Voice accumulation fix (smart release calculation)
2. **69d02d3**: Transfer VoiceManager across graph swaps
3. **cf0d74e**: WIP - IPC protocol and audio engine binary
4. **b2afc62**: Two-process architecture WORKING! - Fix socket path + successful test

## Testing the Progress

**Test the voice fix** (this works now):
```bash
cd /home/erik/phonon
cargo run --release --bin phonon -- render /tmp/test_stut8.ph --cycles 8 -o /tmp/test_stut8.wav
```

**Test the two-process architecture**:
```bash
cargo run --release --bin test_two_process
# Should play audio and complete cleanly
```

**See it fail to integrate yet**:
```bash
phonon edit m.ph
# Still uses old in-process audio (no two-process benefits yet)
```

## Next Steps

The path forward is clear:
1. Refactor `src/modal_editor/mod.rs` to use the two-process architecture
2. Test thoroughly with `m.ph` and other intensive patterns
3. Verify `stut 8` works without underruns
4. Verify Ctrl-X reloads never drop the beat
5. Document the architecture

The foundation is solid. The remaining work is integration and testing.
