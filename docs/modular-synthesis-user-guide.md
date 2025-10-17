# Phonon Modular Synthesis DSL - User Guide

## Overview

The Phonon Modular Synthesis DSL enables you to create complex audio synthesis patches using a text-based language. It supports full cross-modulation between patterns and audio signals, allowing any signal to modulate any parameter in real-time.

## Core Concepts

### 1. Buses (~)
Buses are named signal routes that can carry audio or control signals:

```phonon
~lfo: sine(2)           // Create an LFO bus
~osc: saw(440)          // Create an oscillator bus
~mixed: ~lfo * ~osc     // Combine buses
```

### 2. Signal Chains (>>)
Use the chain operator to process signals through effects:

```phonon
~filtered: ~osc # lpf(1000, 0.7)     // Low-pass filter
~delayed: ~filtered # delay(0.25)     // Add delay
~reverbed: ~delayed # reverb(0.8)     // Add reverb
```

### 3. Arithmetic Operations
Combine signals using standard math operators:

```phonon
~lfo: sine(0.5) * 0.5 + 0.5          // Scale and offset LFO
~modulated: 440 + ~lfo * 100         // Modulate frequency
~mixed: ~bass * 0.4 + ~drums * 0.6   // Mix signals
```

## Audio Sources

### Oscillators
```phonon
~sine: sine(440)        // Sine wave at 440 Hz
~saw: saw(220)          // Sawtooth wave
~square: square(330)    // Square wave
~triangle: tri(550)     // Triangle wave
~noise: noise()         // White noise
```

### Pattern Integration
Embed Strudel patterns directly:

```phonon
~kick: "bd ~ ~ bd"              // Kick drum pattern
~hats: "hh*8"                   // Hi-hat pattern
~bass: "c3 e3 g3 e3"           // Bass notes
```

### Envelope Generators
```phonon
~env1: perc(0.01, 0.3)          // Percussive envelope
~env2: adsr(0.01, 0.1, 0.7, 0.5) // ADSR envelope
~env3: ar(0.1, 0.5)              // Attack-Release
```

## Audio Processors

### Filters
```phonon
~lpf: ~input # lpf(1000, 0.7)   // Low-pass (cutoff, Q)
~hpf: ~input # hpf(2000, 0.8)   // High-pass
~bpf: ~input # bpf(1500, 0.5)   // Band-pass
```

### Effects
```phonon
~delayed: ~input # delay(0.25)           // Delay (time in seconds)
~reverbed: ~input # reverb(0.7)          // Reverb (mix)
~distorted: ~input # distortion(0.5)     // Distortion (amount)
~compressed: ~input # compress(0.3, 4)   // Compressor (threshold, ratio)
```

## Audio Analysis

Extract features from audio signals for modulation:

```phonon
~rms: ~bass # rms(0.05)        // RMS level (window size)
~pitch: ~voice # pitch          // Pitch detection
~transient: ~drums # transient  // Transient detection
~centroid: ~input # centroid    // Spectral brightness
```

## Cross-Modulation

### Pattern → Audio
Use pattern events to modulate synthesis parameters:

```phonon
~bass_pattern: "c2 e2 g2 e2"
~filter_freq: 500 + ~bass_pattern * 1000
~filtered: ~osc # lpf(~filter_freq, 0.8)
```

### Audio → Pattern
Use audio analysis to affect patterns:

```phonon
~bass_level: ~bass # rms(0.05)
~hats: "hh*16" # hpf(~bass_level * 5000 + 2000, 0.8)
```

### Sidechain Compression
Duck one signal based on another:

```phonon
~kick_transient: ~kick # transient
~bass_ducked: ~bass * (1 - ~kick_transient * 0.5)
```

## Modulation Routing

Route a single modulation source to multiple targets:

```phonon
route ~lfo -> {
    bass.filter.cutoff: 0.3,
    lead.delay.feedback: 0.2,
    reverb.mix: 0.1
}
```

Or single routes:

```phonon
route ~bass_transient -> ~hats.gain: -0.5
route ~kick_transient -> ~bass.gain: -0.3
```

## Conditional Processing

Gate signals based on conditions:

```phonon
~gate: ~input # when(~bass_rms > 0.5)
```

## Synthdef Definitions

Define reusable synthesis patches:

```phonon
synthdef kick sine(60) * perc(0.001, 0.1) + noise() * perc(0.001, 0.05) * 0.2
synthdef bass saw(55) # lpf(800, 0.9)
synthdef pad saw(220) + saw(221) # lpf(2000, 0.5) # reverb(0.7)
```

Use synthdefs in patterns:

```phonon
~rhythm: "kick ~ kick ~"
~bassline: "bass*4"
```

## Complete Example

Here's a complete patch demonstrating all features:

```phonon
// === LFOs and Control ===
~lfo_slow: sine(0.25) * 0.5 + 0.5     // Slow LFO for filter sweeps
~lfo_fast: sine(6) * 0.3              // Fast LFO for vibrato

// === Bass Synthesis ===
~bass_env: perc(0.01, 0.3)
~bass_osc: saw(55) * ~bass_env
~bass: ~bass_osc # lpf(~lfo_slow * 2000 + 500, 0.8)

// === Extract Bass Features ===
~bass_rms: ~bass # rms(0.05)
~bass_transient: ~bass # transient

// === Drums with Cross-Modulation ===
~kick: "bd ~ ~ bd" # gain(1.0)
~kick_transient: ~kick # transient

// Snare brightness modulated by bass level
~snare: "~ sn ~ sn" # lpf(~bass_rms * 4000 + 1000, 0.7)

// Hi-hats gated by bass level
~hats: "hh*16" # hpf(~bass_rms * 8000 + 2000, 0.8) # gain(0.3)
~hats_gated: ~hats # when(~bass_rms > 0.3)

// === Sidechain Compression ===
~bass_ducked: ~bass * (1 - ~kick_transient * 0.5)

// === Lead Synthesis ===
~lead_freq: 440 + ~lfo_fast * 20
~lead: square(~lead_freq) * 0.3
~lead_delayed: ~lead # delay(0.375) # lpf(3000, 0.5)

// === Chord Progression ===
synthdef c_major sine(261.63) + sine(329.63) + sine(392.0)
synthdef f_major sine(349.23) + sine(440.0) + sine(523.25)
synthdef g_major sine(392.0) + sine(493.88) + sine(587.33)

~chords: "c_major ~ f_major ~ g_major ~" # gain(0.2)

// === Modulation Routing ===
route ~lfo_slow -> {
    bass.filter.cutoff: 0.3,
    lead.delay.feedback: 0.2,
    reverb.mix: 0.1
}

route ~bass_transient -> ~hats.gain: -0.5    // Duck hats on bass hits
route ~kick_transient -> ~bass.gain: -0.3    // Sidechain compression

// === Master Mix ===
~reverb_send: (~lead * 0.3) + (~chords * 0.5)
~reverb_out: ~reverb_send # reverb(0.7, 0.8)

~pre_master: (~bass_ducked * 0.4) + (~kick * 0.5) + (~snare * 0.3) + 
             (~hats_gated * 0.2) + (~lead_delayed * 0.3) + 
             (~chords * 0.2) + (~reverb_out * 0.2)

~master: ~pre_master # compress(0.3, 4) # limit(0.95)

// === Output ===
out: ~master
```

## Tips and Best Practices

1. **Start Simple**: Begin with basic oscillators and filters, then add modulation
2. **Use Meaningful Names**: Name your buses descriptively (e.g., `~bass_filter` not `~f1`)
3. **Normalize LFOs**: Scale LFOs to 0-1 range for predictable modulation
4. **Monitor Levels**: Use RMS analysis to track signal levels
5. **Layer Effects**: Build complex sounds by chaining multiple processors
6. **Cross-Modulate**: Use audio features to create dynamic, reactive patches
7. **Comment Your Patches**: Use `//` comments to document complex routing

## Common Patterns

### Vibrato
```phonon
~vibrato: sine(5) * 10
~pitch: 440 + ~vibrato
~voice: sine(~pitch)
```

### Tremolo
```phonon
~tremolo: sine(4) * 0.5 + 0.5
~modulated: ~input * ~tremolo
```

### Auto-Wah
```phonon
~envelope: ~input # rms(0.01)
~wah: ~input # lpf(~envelope * 2000 + 200, 3)
```

### Stereo Spread
```phonon
~left: ~mono # delay(0.020) * 0.5
~right: ~mono # delay(0.023) * 0.5
```

## Performance Considerations

- **CPU Usage**: Complex patches with many analysis nodes may increase CPU load
- **Latency**: Analysis features introduce small amounts of latency
- **Buffer Size**: Smaller buffers reduce latency but increase CPU usage
- **Feedback**: Be careful with feedback loops to avoid runaway oscillation

## Troubleshooting

### No Sound
- Check that your patch has an `out:` statement
- Verify signal levels aren't too low (use gain)
- Ensure filters aren't cutting all frequencies

### Distortion/Clipping
- Reduce gain stages
- Add a limiter to the output chain
- Check for feedback loops

### Unexpected Modulation
- Verify bus names are spelled correctly
- Check modulation amounts aren't too high
- Use the visualization helpers to monitor signals

## Advanced Topics

### Parallel Processing
Process a signal through multiple paths simultaneously:

```phonon
~dry: saw(220)
~low: ~dry # lpf(800, 0.7)
~high: ~dry # hpf(2000, 0.7)
~mid: ~dry # bpf(1000, 0.5)
~mixed: ~low * 0.3 + ~mid * 0.4 + ~high * 0.3
```

### Feedback Networks
Create feedback loops (use with caution):

```phonon
~feedback: ~delay_out * 0.7
~delay_out: (~input + ~feedback) # delay(0.25) # lpf(2000)
```

### Multi-tap Delays
Create complex delay effects:

```phonon
~tap1: ~input # delay(0.125) * 0.5
~tap2: ~input # delay(0.250) * 0.3
~tap3: ~input # delay(0.375) * 0.2
~multitap: ~input + ~tap1 + ~tap2 + ~tap3
```

## Live Coding

The DSL supports hot-swapping for live performance:
- Edit your .phonon file
- Changes are applied in real-time
- No need to restart the engine
- Smooth transitions between patches

## Integration with Strudel

The DSL seamlessly integrates with Strudel patterns:
- Pattern strings are automatically converted to signals
- Note names become frequencies
- Sample triggers generate envelopes
- Pattern timing syncs with global tempo

Happy patching!