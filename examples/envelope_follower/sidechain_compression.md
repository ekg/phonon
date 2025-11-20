# Envelope Follower Examples

This directory contains examples demonstrating the `EnvelopeFollowerNode` for DAW-style buffer-based audio processing.

## Overview

The `EnvelopeFollowerNode` extracts the amplitude envelope from an audio signal using exponential smoothing with separate attack and release time constants. This is a fundamental building block for:

- **Sidechain compression** - Classic ducking effect (kick drum ducks bass)
- **Auto-wah** - Envelope controls filter cutoff
- **Envelope-following synthesis** - Amplitude-to-CV conversion
- **Dynamics visualization** - VU meters, level indicators
- **Adaptive effects** - Effects that respond to input dynamics

## Node Specification

```rust
pub struct EnvelopeFollowerNode {
    input: NodeId,           // Audio signal to analyze
    attack_input: NodeId,    // Attack time in seconds (0.001-0.01 typical)
    release_input: NodeId,   // Release time in seconds (0.01-3.0 typical)
    envelope_state: f32,     // Current envelope value
}
```

### Algorithm

Classic analog envelope follower:

1. **Full-wave rectification**: `abs(input)`
2. **Exponential smoothing**: Different coefficients for rising vs falling signal
   ```
   attack_coeff = exp(-1 / (attack_time * sample_rate))
   release_coeff = exp(-1 / (release_time * sample_rate))

   if rectified > envelope:
       envelope = attack_coeff * envelope + (1 - attack_coeff) * rectified
   else:
       envelope = release_coeff * envelope + (1 - release_coeff) * rectified
   ```

## Example 1: Sidechain Compression (Kick Ducking Bass)

Classic dance music technique where the kick drum's envelope controls the bass level.

### Setup

```rust
use phonon::nodes::{
    ConstantNode, OscillatorNode, Waveform,
    EnvelopeFollowerNode, MultiplicationNode, SubtractionNode
};
use phonon::audio_node::{AudioNode, ProcessContext};
use phonon::pattern::Fraction;

// Audio sources
let kick_drum = ...; // NodeId 0 - Kick drum sample or synth
let bass_synth = ...; // NodeId 1 - Sub bass (saw wave at 55 Hz)

// Envelope follower parameters
let attack = ConstantNode::new(0.005);   // 5ms attack - fast response to kick
let release = ConstantNode::new(0.3);    // 300ms release - smooth ducking tail

// Extract kick's envelope
let kick_envelope = EnvelopeFollowerNode::new(
    0,  // kick_drum
    2,  // attack (NodeId 2)
    3   // release (NodeId 3)
);

// Create ducking signal: 1.0 - envelope (inverted)
// When kick hits (envelope=1.0), ducking=0.0 (silence bass)
// When kick fades (envelope=0.0), ducking=1.0 (full bass)
let one = ConstantNode::new(1.0);
let ducking_signal = SubtractionNode::new(
    5,  // one (NodeId 5)
    4   // kick_envelope (NodeId 4)
);

// Apply ducking to bass
let ducked_bass = MultiplicationNode::new(
    1,  // bass_synth
    6   // ducking_signal (NodeId 6)
);

// Mix kick and ducked bass
let output = AdditionNode::new(0, 7);  // kick + ducked_bass
```

### Parameter Tuning

**Attack time** (how fast to respond to kick hit):
- **1-5ms**: Tight, punchy ducking - preserves kick transient
- **10-20ms**: Softer ducking - less abrupt
- **50ms+**: Slow ducking - unnatural, use only for special effects

**Release time** (how long to stay ducked after kick):
- **50-100ms**: Fast pumping - techno/trance energy
- **200-400ms**: Medium pumping - house/progressive house
- **500ms-1s**: Slow pumping - ambient/downtempo feel
- **2-3s**: Very slow - creates long breathing effect

### Musical Examples

**Techno (128 BPM)**:
```
Kick: 4/4 pattern every beat
Attack: 3ms
Release: 150ms
Effect: Tight, rhythmic pumping that drives the groove
```

**Deep House (120 BPM)**:
```
Kick: 4/4 pattern with varying velocity
Attack: 5ms
Release: 300ms
Effect: Smooth ducking that lets bass breathe between kicks
```

**Dubstep (140 BPM)**:
```
Kick: Sparse pattern (every other beat)
Attack: 2ms
Release: 500ms
Effect: Extreme pumping creates dramatic dynamics
```

## Example 2: Auto-Wah Effect

The envelope of an audio signal modulates a filter cutoff frequency.

### Setup

```rust
// Guitar or synth input
let guitar = ...; // NodeId 0

// Extract envelope with moderate settings
let attack = ConstantNode::new(0.01);   // 10ms
let release = ConstantNode::new(0.08);  // 80ms
let envelope = EnvelopeFollowerNode::new(0, 1, 2); // NodeId 3

// Scale envelope to filter frequency range
// envelope (0.0-1.0) -> filter cutoff (200-5000 Hz)
let min_freq = ConstantNode::new(200.0);
let freq_range = ConstantNode::new(4800.0);  // 5000 - 200
let scaled_env = MultiplicationNode::new(3, 5);  // envelope * range
let filter_freq = AdditionNode::new(6, 4);       // scaled + min_freq

// Apply resonant lowpass filter
let filter_q = ConstantNode::new(4.0);  // High resonance for wah effect
let wah_output = RLPFNode::new(0, 7, 8);  // input, freq, Q
```

### Parameter Tuning

**Attack** (response to pick/pluck):
- **1-5ms**: Instant response - percussive, funky
- **10-20ms**: Slight delay - smoother, vocal-like
- **50ms+**: Very slow - unusual, ambient

**Release** (tail after note):
- **20-50ms**: Quick close - tight, rhythmic
- **100-200ms**: Medium tail - classic wah sound
- **500ms+**: Long tail - ambient, pad-like

**Filter range**:
- **Low**: 100-1000 Hz - Dark, bassy wah
- **Mid**: 200-3000 Hz - Classic guitar wah
- **High**: 500-8000 Hz - Bright, vocal-like sweep

## Example 3: Envelope-Following Synthesis

Use one sound's envelope to control another's amplitude.

### Setup

```rust
// Use drum loop's envelope to trigger synth
let drums = ...; // NodeId 0 - Full drum mix
let synth_osc = OscillatorNode::new(1, Waveform::Sine); // NodeId 2

// Extract drum envelope
let attack = ConstantNode::new(0.003);   // 3ms - catch transients
let release = ConstantNode::new(0.05);   // 50ms - tight response
let drum_env = EnvelopeFollowerNode::new(0, 3, 4); // NodeId 5

// Apply to synth - synth follows drum rhythm
let synth_output = MultiplicationNode::new(2, 5);
```

### Creative Applications

**Rhythmic gating**: Fast attack/release creates staccato synth following drums

**Breathing pads**: Slow attack/release creates swelling pads that breathe with drums

**Vocoder-style**: Use vocal envelope to shape synth (basic vocoder building block)

## Example 4: Adaptive Compression Threshold

Use envelope follower as part of a dynamics processor.

### Setup

```rust
// Detect signal level
let input = ...; // NodeId 0
let attack = ConstantNode::new(0.01);    // 10ms
let release = ConstantNode::new(0.1);    // 100ms
let level = EnvelopeFollowerNode::new(0, 1, 2); // NodeId 3

// Compare to threshold
let threshold = ConstantNode::new(0.5);  // NodeId 4
let over_threshold = GreaterThanNode::new(3, 4); // NodeId 5

// Calculate gain reduction when over threshold
// (This is simplified - real compressor has ratio, makeup gain, etc.)
let gain_reduction = ConstantNode::new(0.5);  // Reduce to 50%
let normal_gain = ConstantNode::new(1.0);
let gain = WhenNode::new(5, 6, 7); // If over_threshold, use reduction, else normal

// Apply compression
let compressed = MultiplicationNode::new(0, 8);
```

## Performance Characteristics

### Computational Cost

**Per-sample operations**:
- 1x abs() - Full-wave rectification
- 2x max() - Parameter clamping
- 2x exp() - Coefficient calculation
- 1x conditional - Attack vs release selection
- 2-3x multiply-add - Exponential smoothing

**Total**: ~10-12 operations per sample

**For 512-sample block at 44.1kHz**: ~5,000-6,000 operations

This is **very efficient** compared to:
- FFT-based analysis: ~50,000 operations
- RMS calculation: ~15,000 operations (requires windowing)

### Memory Usage

**Per-instance state**: 4 bytes (one f32 for envelope_state)

**Stack usage**: Negligible (<100 bytes per process_block call)

### Latency

**Zero latency** - Processes current sample, no lookahead required.

Attack/release times create perceptual delay but no algorithmic latency.

## Tips and Best Practices

### 1. Attack Time Selection

**For transient detection** (drums, percussive):
- Use 1-5ms to capture sharp attacks
- Too slow misses transients (sounds laggy)
- Too fast tracks noise (sounds jittery)

**For smooth material** (pads, sustained notes):
- Use 10-50ms for musical response
- Faster isn't always better - can cause pumping

### 2. Release Time Selection

**Follow the music tempo**:
```
Release time ≈ (60 / BPM) * beat_fraction

Examples:
- 120 BPM, 1/4 note: (60/120) * 1 = 0.5s = 500ms
- 140 BPM, 1/8 note: (60/140) * 0.5 = 214ms
- 90 BPM, 1/2 note: (60/90) * 2 = 1333ms
```

This ensures pumping is rhythmic and musical.

### 3. Combining with Other Effects

**Before distortion**: Envelope following before distortion captures clean dynamics

**After reverb**: Envelope following reverb tail creates ambient ducking

**Parallel processing**: Envelope follow dry signal, apply to wet effect

### 4. Avoiding Common Pitfalls

**Problem**: Envelope too jittery
- **Solution**: Increase attack time (smooths response)
- **Or**: Add small pre-filter (gentle lowpass at 10-20 Hz)

**Problem**: Not responding to quiet signals
- **Solution**: Apply gain before envelope follower
- **Or**: Use compressor to normalize input level first

**Problem**: Release too short, sounds choppy
- **Solution**: Increase release time to match musical phrase length

**Problem**: Sidechain pumping too extreme
- **Solution**: Reduce ducking depth (don't go all the way to 0)
  ```rust
  // Instead of: ducking = 1.0 - envelope
  // Use: ducking = 0.3 + 0.7 * (1.0 - envelope)
  // This ducks from 100% to 30% instead of 0%
  ```

## Testing the Implementation

The `envelope_follower.rs` implementation includes 14 comprehensive tests:

1. **test_envelope_follower_tracks_rising_signal** - Verifies attack response
2. **test_envelope_follower_tracks_falling_signal** - Verifies release response
3. **test_envelope_follower_fast_attack_vs_slow_attack** - Compares attack times
4. **test_envelope_follower_fast_release_vs_slow_release** - Compares release times
5. **test_envelope_follower_with_sine_wave** - Tests with continuous waveform
6. **test_envelope_follower_with_square_wave** - Tests with discontinuous waveform
7. **test_envelope_follower_negative_values** - Verifies full-wave rectification
8. **test_envelope_follower_handles_impulse** - Tests transient response
9. **test_envelope_follower_dependencies** - Verifies node graph structure
10. **test_envelope_follower_state_persistence** - Tests state across blocks
11. **test_envelope_follower_reset** - Verifies reset functionality
12. **test_envelope_follower_very_fast_attack** - Extreme attack time
13. **test_envelope_follower_very_slow_release** - Extreme release time
14. **test_envelope_follower_pattern_modulated_times** - Per-sample parameter variation

Run tests with:
```bash
cargo test envelope_follower
```

## Related Nodes

- **PeakDetectorNode**: Similar but only tracks peaks (instant attack, no exponential smoothing)
- **RMSNode**: Measures average power over a window (different from amplitude envelope)
- **CompressorNode**: Uses envelope follower internally for gain reduction
- **GateNode**: Simple threshold-based dynamics (no smooth envelope)

## References

- "Designing Audio Effect Plugins in C++" by Will Pirkle (Chapter 6: Dynamics Processors)
- "The Art of Mixing" by David Gibson (Chapter on Sidechain Compression)
- "Computer Music: Synthesis, Composition, and Performance" by Dodge & Jerse (Chapter 3: Envelope Following)
- SuperCollider's `Amplitude.ar` UGen documentation
- Ableton Live's Sidechain Compression tutorial

## License

This implementation and documentation are part of the Phonon project, licensed under MIT.
