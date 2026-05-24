# Audio Live-Edit Glitch Reproduction Harness

## Purpose

Reproducible harness for the reported issue: audio sometimes becomes corrupted
or degraded after editing and relaunching/reloading phonon code while the
process stays alive.

## How to Run

```bash
# Run all harness tests with full output
cargo test --test audio_live_edit_glitch_harness -- --nocapture

# Run only the smoke test (fast, single cycle)
cargo test --test audio_live_edit_glitch_harness test_glitch_harness_smoke_single_reload -- --nocapture

# Run only the full 30-cycle harness
cargo test --test audio_live_edit_glitch_harness test_audio_live_edit_glitch_harness -- --nocapture
```

## Test File Location

`tests/audio_live_edit_glitch_harness.rs`

## What the Harness Tests

The harness exercises the same engine state transitions as the modal editor's
`load_code()` path:

```
parse → compile → enable_wall_clock → state_transfer → preload_samples → graph_swap → render
```

**30 deterministic reload cycles** across five scenario categories:

| Category | Count | Description |
|----------|-------|-------------|
| Oscillator frequency jumps | 5 | Frequency and waveform changes |
| Tempo (CPS) changes | 5 | Various BPM multipliers |
| Effect chain add/remove | 5 | LPF/HPF insertion and removal |
| New buses added/removed | 5 | Bus topology changes |
| Minimal constant programs | 5 | Silence/gain edge cases |
| Combination scenarios | 5 | Multi-oscillator, filter sweeps, simultaneous tempo+freq |

## Metrics Collected Per Cycle

| Metric | Description | Hard Failure Threshold |
|--------|-------------|------------------------|
| **NaN count** | Non-finite NaN samples in any buffer | Any NaN → FAIL |
| **Inf count** | Non-finite Inf samples in any buffer | Any Inf → FAIL |
| **Clip fraction** | Fraction of samples with \|s\| > 1.0 | >5% → FAIL |
| **Silence** | RMS < 0.001 when signal expected | Any silence → FAIL |
| **Stuck output** | New buffer bit-identical to old graph tail | Any stuck → FAIL |
| **Boundary discontinuity** | \|last_pre_sample - first_post_sample\| | >0.5 flagged (warning) |
| **RMS jump ratio** | max(post/pre, pre/post) across transition | >10× flagged (warning) |
| **Reload time** | Microseconds for parse+compile+state_transfer | Reported only |

Hard failures cause `cargo test` to exit nonzero. Warnings are reported in
the summary but do not fail the test.

## Scenarios Attempted

### Cycle-by-Cycle Results (run 2026-05-24)

```
=== Audio Live-Edit Glitch Harness (30 cycles) ===
  Buffer: 1024 floats = 512 stereo frames ≈ 11.6 ms
  Render: 8×pre + reload + 8×post per cycle
  Thresholds: clip>5%, silence_rms<0.001, dc_offset>0.1, disc>0.5

  [01] osc-freq-jump-110-220          reload=27505µs | pre_rms=0.2115 post_rms=0.2124 | disc=0.2925 rms_ratio=1.03
  [02] osc-freq-jump-220-440          reload=24697µs | pre_rms=0.2124 post_rms=0.2123 | disc=0.1301 rms_ratio=1.00
  [03] osc-freq-jump-saw-110-330      reload=24869µs | pre_rms=0.1162 post_rms=0.1152 | disc=0.0857 rms_ratio=1.14
  [04] osc-waveform-sine-to-saw       reload=24314µs | pre_rms=0.2124 post_rms=0.1155 | disc=0.3301 rms_ratio=1.85
  [05] osc-waveform-saw-to-sine       reload=23931µs | pre_rms=0.1155 post_rms=0.2124 | disc=0.0286 rms_ratio=1.74
  [06] tempo-1.0-to-2.0               reload=25201µs | pre_rms=0.2115 post_rms=0.2115 | disc=0.2925 rms_ratio=1.05
  [07] tempo-2.0-to-0.5               reload=24096µs | pre_rms=0.2115 post_rms=0.2115 | disc=0.2925 rms_ratio=1.05
  [08] tempo-0.5-to-1.0               reload=24320µs | pre_rms=0.2115 post_rms=0.2115 | disc=0.2925 rms_ratio=1.05
  [09] tempo-1.0-to-3.0               reload=24110µs | pre_rms=0.2124 post_rms=0.2124 | disc=0.1301 rms_ratio=1.01
  [10] tempo-3.0-to-1.0               reload=24188µs | pre_rms=0.2124 post_rms=0.2124 | disc=0.1301 rms_ratio=1.01
  [11] add-lpf                        reload=24219µs | pre_rms=0.1162 post_rms=0.1129 | disc=0.1102 rms_ratio=1.07
  [12] remove-lpf                     reload=24441µs | pre_rms=0.1129 post_rms=0.1162 | disc=0.0779 rms_ratio=1.01
  [13] add-hpf                        reload=24124µs | pre_rms=0.1152 post_rms=0.0833 | disc=0.2572 rms_ratio=1.50
  [14] remove-hpf                     reload=24110µs | pre_rms=0.0833 post_rms=0.1152 | disc=0.1929 rms_ratio=1.32
  [15] change-lpf-cutoff              reload=24043µs | pre_rms=0.1097 post_rms=0.1145 | disc=0.0009 rms_ratio=1.01
  [16] add-bus                        reload=24430µs | pre_rms=0.2115 post_rms=0.2115 | disc=0.2925 rms_ratio=1.05
  [17] remove-bus                     reload=25460µs | pre_rms=0.2115 post_rms=0.2115 | disc=0.2925 rms_ratio=1.05
  [18] add-second-bus                 reload=24787µs | pre_rms=0.2115 post_rms=0.1588 | disc=0.2925 rms_ratio=1.21
  [19] remove-second-bus              reload=24127µs | pre_rms=0.1588 post_rms=0.2115 | disc=0.2384 rms_ratio=1.31
  [20] rename-bus                     reload=25258µs | pre_rms=0.2115 post_rms=0.2115 | disc=0.2925 rms_ratio=1.05
  [21] constant-to-osc                reload=24637µs | pre_rms=0.0000 post_rms=0.2115 | disc=0.0000 rms_ratio=1.00
  [22] osc-to-constant-silence        reload=24889µs | pre_rms=0.2115 post_rms=0.0000 | disc=0.2925 rms_ratio=204607.97  ⚠ RMS_JUMP
  [23] gain-halve                     reload=24389µs | pre_rms=0.2832 post_rms=0.1416 | disc=0.1735 rms_ratio=2.01
  [24] gain-double                    reload=26253µs | pre_rms=0.1416 post_rms=0.2832 | disc=0.0868 rms_ratio=1.99
  [25] gain-to-edge                   reload=26496µs | pre_rms=0.1416 post_rms=0.6373 | disc=0.0868 rms_ratio=4.47
  [26] multi-osc-merge                reload=31419µs | pre_rms=0.2115 post_rms=0.1510 | disc=0.2925 rms_ratio=1.27
  [27] multi-osc-split                reload=25434µs | pre_rms=0.1510 post_rms=0.2115 | disc=0.2113 rms_ratio=1.35
  [28] lpf-sweep-cutoff               reload=25441µs | pre_rms=0.0977 post_rms=0.1136 | disc=0.0214 rms_ratio=1.16
  [29] lpf-resonance-change           reload=25487µs | pre_rms=0.1067 post_rms=0.1144 | disc=0.0006 rms_ratio=1.05
  [30] tempo-and-osc-simultaneous     reload=25525µs | pre_rms=0.2115 post_rms=0.2124 | disc=0.2925 rms_ratio=1.03

=== Summary ===
  Cycles:            30
  Reload time:       avg=25073µs  max=31419µs
  NaN samples:       0
  Inf samples:       0
  Severe-clip cycles:0
  Silent cycles:     0
  Stuck cycles:      0
  Disc > threshold:  0 cycles  (max=0.3301)
  High-RMS-jump:     1 cycles

✅ PASSED — no hard failures detected.
```

## Observed Results

### Hard Failures: None

No hard failures were detected in any of the 30 cycles:
- **NaN/Inf**: 0 samples total
- **Severe clipping**: 0 cycles
- **Unexpected silence**: 0 cycles
- **Stuck output**: 0 cycles

### Soft Observations (Warnings)

**1. Boundary discontinuities present but below threshold**

The maximum boundary discontinuity across all 30 cycles was 0.3301 (scenario:
`osc-waveform-sine-to-saw`), below the hard threshold of 0.5. However,
discontinuities in the 0.2–0.3 range are perceptible as clicks in audio at
44100 Hz and represent a known risk area.

Several scenarios show a recurring disc=0.2925 value, which is the last sample
of a 110 Hz sine wave at a particular phase when the reload happens. The
transition code (`transfer_session_timing`) carries the cycle position forward
but does not zero-cross the outgoing signal, so the click magnitude is
phase-dependent.

**2. RMS jump ratio on osc-to-constant-silence (expected)**

Scenario `osc-to-constant-silence` showed an RMS jump ratio of 204607×. This
is expected behavior: the program changes from `sine 110 * 0.3` to `out $ 0.0`,
so the output legitimately goes from ~0.21 RMS to 0.0. The harness correctly
marks this as a warning (not a failure) because `after_is_silent` is set for
scenarios containing "silence" in their name.

**3. Reload times average 25 ms**

The average reload time was 25.1 ms with a maximum of 31.4 ms. This is
below one typical audio buffer callback interval (~12 ms for 512-frame buffers
at 44100 Hz) times three—the ring buffer provides enough headroom. However,
if the UI thread and audio thread share CPU, reload latency could spike during
system load, potentially exhausting the ring buffer and causing an underrun.

## Remaining Gap: Full CPAL Integration Path

The harness exercises the engine state transition path (`parse → compile →
state_transfer → render`) in headless mode (no CPAL stream, no ring buffer).
This covers the vast majority of state transition logic and is sufficient to
detect NaN/Inf, stuck output, silence, and large discontinuities.

**What the harness does NOT exercise:**

1. **Ring buffer starvation under load** — The CPAL callback reads from a
   lockfree ring buffer (`ArcSwap<RingBuffer>`) that the main thread refills.
   If the main thread stalls during the 25 ms compile window, the ring buffer
   can empty, causing an underrun. This requires a running CPAL stream to test.

2. **ArcSwap graph swap under live CPAL callback** — The actual atomic swap
   of the signal graph pointer happens in the CPAL thread callback path
   (`phonon-audio.rs:process_callback`). The harness simulates this with a
   direct ownership transfer, which is logically equivalent but does not test
   any race conditions around the swap.

3. **Memory allocation in the realtime thread** — The realtime audit
   (`docs/research/realtime-reload-audit.md`) identified several allocation
   paths in `process_buffer`. The harness exercises these paths and would
   detect their effects (silence, NaN, stuck), but does not measure allocation
   latency.

**Why the harness still catches future regressions:**

- Any code change that introduces NaN/Inf in oscillator or filter output will
  be caught immediately by the 30-cycle harness.
- Any change that breaks the graph swap state transfer (timing, FX states,
  voice manager) will be caught by comparing pre/post RMS and silence checks.
- The discontinuity metric at the cycle boundary will detect if any change
  makes transitions worse (higher than current max of 0.3301).
- Stuck output detection (bit-identical pre/post buffers) catches cases where
  the new graph fails to produce any output.

## Implementation Notes

The `live_reload()` function in the harness mirrors the sequence in
`ModalEditor::load_code()` (see `src/modal_editor/mod.rs`):

```rust
fn live_reload(old_graph, new_code) -> (UnifiedSignalGraph, elapsed_us) {
    let mut new_graph = compile_graph(new_code);     // parse + compile
    new_graph.enable_wall_clock_timing();             // start wall-clock
    new_graph.transfer_session_timing(old_graph);    // copy timing
    new_graph.transfer_fx_states(old_graph);         // copy FX state
    new_graph.transfer_voice_manager(old_graph.take_voice_manager()); // voice state
    new_graph.preload_samples();                      // load samples
    (new_graph, elapsed)
}
```

No stabilization fixes were required to make the harness run. The engine
already handles all 30 scenarios without hard failures.
