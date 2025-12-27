# Hadamard Diffuser Implementation

## Overview

The Hadamard Diffuser is a high-quality multi-channel diffusion network designed for reverb applications in Phonon. It transforms a single impulse into a dense cloud of reflections, creating natural-sounding reverberation.

**File**: `src/nodes/diffuser.rs`

## Architecture

### Core Design

- **8 parallel channels** for spatial decorrelation
- **4 cascaded diffusion steps** for exponential echo density growth
- **Fast Hadamard Transform** for efficient mixing
- **Variable delays** with prime-like spacing to avoid resonances

### Per-Step Processing

Each of the 4 diffusion steps performs:

1. **Variable Delays** - Each channel has a different delay time:
   - Step 1: base delays (23, 41, 59, 73, 89, 107, 127, 149 samples @ 44.1kHz)
   - Step 2: base × 1.5
   - Step 3: base × 2.0
   - Step 4: base × 3.0

2. **Allpass Diffusion** - Controlled by diffusion parameter (0-1):
   - `diffusion=0`: Pure delays only (minimal spreading)
   - `diffusion=1`: Full allpass character (maximum spreading)
   - Formula: `y = x[n-d] + g*(x[n-d] - x[n])`

3. **Channel Shuffle** - Different pattern per step:
   - Rotates channels
   - Flips polarity on some channels
   - Breaks up phase coherence

4. **Hadamard Transform** - 8×8 orthogonal mixing:
   - Fast algorithm: O(n log n) instead of O(n²)
   - Energy-preserving (normalized by 1/√8)
   - Uniform mixing with ±1 coefficients

## Echo Density Growth

Starting with a single impulse, the diffuser creates:
- After step 1: ~8 reflections
- After step 2: ~64 reflections
- After step 3: ~512 reflections
- After step 4: ~4096 reflections

This exponential growth creates the dense, natural sound of a real room.

## Parameters

### Input Signal (NodeId)
The mono audio signal to diffuse (typically a dry sound source).

### Diffusion Amount (0.0 - 1.0)
Controls the allpass feedback coefficient:
- **0.0** - Pure delays only, minimal spreading
- **0.3-0.5** - Light diffusion, preserves transients
- **0.7-0.9** - Heavy diffusion, dense reverb tail
- **1.0** - Maximum spreading, longest decay

Values outside [0, 1] are automatically clamped.

## Technical Features

### Delay Time Scaling
Automatically scales with sample rate:
```rust
let scale = sample_rate / 44100.0;
delay_time = base_delay * step_scale * scale;
```

### Channel Shuffle Patterns
Each step uses a unique shuffle to maximize decorrelation:
- **Step 0**: Rotate right by 1, flip odd channels
- **Step 1**: Rotate right by 3, flip even channels
- **Step 2**: Reverse order, flip channels 0,1,4,5
- **Step 3**: Rotate right by 2, flip channels 2,3,6,7

### Energy Characteristics
- Hadamard transform preserves energy (orthogonal)
- Allpass feedback can temporarily increase energy (normal behavior)
- Typical energy variation: 0.5x to 3x of input
- Energy spreads over time due to delays

## Usage Examples

### Basic Diffusion
```phonon
~dry $ s "bd"
~wet $ ~dry # diffuser 0.7
out $ ~dry * 0.4 + ~wet * 0.6
```

### Modulated Diffusion
```phonon
~lfo $ sine 0.25
~kick $ s "bd"
~diffused $ ~kick # diffuser (~lfo * 0.5 + 0.5)
out $ ~diffused
```

### Multiple Diffusion Stages
```phonon
~signal $ s "sn"
~light $ ~signal # diffuser 0.3
~heavy $ ~light # diffuser 0.8
out $ ~signal * 0.3 + ~light * 0.4 + ~heavy * 0.3
```

## Implementation Details

### State Management
```rust
pub struct DiffuserNode {
    delay_buffers: [[Vec<f32>; 8]; 4],    // 32 delay lines total
    write_indices: [[usize; 8]; 4],        // Circular buffer pointers
    delay_times: [[usize; 8]; 4],          // Delay lengths in samples
}
```

### Fast Hadamard Transform
Recursive butterfly structure for O(n log n) performance:
```rust
// Level 1: pairs
// Level 2: quads
// Level 3: octets
// Normalize: divide by √8
```

### Mono-to-Multichannel Distribution
Input signal is distributed across all 8 channels:
```rust
let mut channels = [input_sample / 8.0_f32.sqrt(); 8];
```

This prevents energy loss when mixing back to mono at the output.

## Test Coverage

All tests pass with comprehensive verification:

1. **Impulse Spreading** - Verifies echo density increases
2. **Zero Diffusion** - Pure delay passthrough
3. **High Diffusion** - Maximum spreading
4. **Energy Conservation** - Reasonable energy range (0.3x - 4.0x)
5. **No NaN/Inf** - Numerical stability
6. **Hadamard Transform** - Energy preservation
7. **Channel Shuffle** - Correct reordering
8. **Clear/Reset** - Buffer clearing
9. **Dependencies** - Graph node connections
10. **Sample Rate Scaling** - Works at 22.05kHz to 96kHz
11. **Parameter Clamping** - Handles out-of-range values

**Total**: 11 tests, all passing

## Performance

- **CPU Efficient**: Fast Hadamard Transform, no matrix multiplication
- **Memory**: ~4ms of delay buffering @ 44.1kHz (~176 samples total per channel)
- **Latency**: Minimal (~3ms through all stages)

## References

Based on techniques from:
- **Signalsmith Audio** - Multi-channel diffusion networks
- **Jon Dattorro** - "Effect Design" papers (reverb architectures)
- **Manfred Schroeder** - Allpass diffusion theory

## Future Enhancements

Potential improvements:
- [ ] Stereo diffusion (16 channels, L/R separation)
- [ ] Feedback matrix (recirculation for longer tails)
- [ ] Modulated delay times (subtle chorus effect)
- [ ] Damping filters (frequency-dependent decay)

## Integration with Reverb

The diffuser is a building block for full reverb algorithms. Typical usage:

```
Input → Early Reflections → Diffuser → Late Reverb Tank → Output
```

The diffuser handles the critical task of transforming discrete early reflections into a dense, smooth reverb tail.
