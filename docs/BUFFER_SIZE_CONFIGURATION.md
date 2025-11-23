# Audio Buffer Size Configuration

Phonon supports configurable audio buffer size for tuning latency vs CPU usage.

## Overview

**Buffer Size** determines how many samples are processed at once:
- **Smaller buffers** = Lower latency but higher CPU usage
- **Larger buffers** = Higher latency but more CPU headroom

**Default**: 128 samples (~3ms at 44.1kHz) - Good balance for most systems

## Latency Reference (44.1kHz Sample Rate)

| Buffer Size | Latency | Use Case |
|-------------|---------|----------|
| 32 samples  | 0.7ms   | Extreme low-latency (may cause glitches) |
| 64 samples  | 1.5ms   | Ultra-low latency (requires powerful CPU) |
| **128 samples** | **3ms** | **Low latency (recommended default)** |
| 256 samples | 6ms     | Medium latency (complex patches) |
| 512 samples | 12ms    | High latency (maximum headroom) |
| 1024 samples | 23ms   | Very high latency (slow systems) |
| 2048 samples | 46ms   | Extreme latency (offline rendering) |

## Configuration Methods

### Runtime Configuration (No Rebuild Required)

Set the `PHONON_BUFFER_SIZE` environment variable:

```bash
# Ultra-low latency (1.5ms)
PHONON_BUFFER_SIZE=64 phonon live examples/simple_working_beat.ph

# Low latency - Default (3ms)
PHONON_BUFFER_SIZE=128 phonon live examples/simple_working_beat.ph

# Medium latency (6ms)
PHONON_BUFFER_SIZE=256 phonon live examples/simple_working_beat.ph

# High latency (12ms)
PHONON_BUFFER_SIZE=512 phonon live examples/simple_working_beat.ph
```

### Compile-Time Configuration

Edit `src/bin/phonon-audio.rs` and change:

```rust
const DEFAULT_BUFFER_SIZE: usize = 128; // Change to desired size
```

Then rebuild:

```bash
cargo build --release --bin phonon-audio
```

## Automatic Bounds Checking

The buffer size is automatically clamped to safe values:
- **Minimum**: 32 samples (0.7ms)
- **Maximum**: 2048 samples (46ms)

Invalid values (non-numeric, negative) fall back to the default (128).

## How to Choose Buffer Size

### For Live Performance
- Start with **128 samples** (default)
- If you experience audio glitches/dropouts → increase to 256 or 512
- If latency feels too high → decrease to 64 (requires powerful CPU)

### For Studio Production
- Use **256-512 samples** for complex patches with many effects
- More headroom for CPU-intensive processing

### For Low-Latency Applications
- Use **64 samples** for responsive live-coding or performance
- Requires modern CPU (2020+) with good single-thread performance

### For Slow/Old Systems
- Use **512-1024 samples** if you hear crackling/glitches
- Trades latency for stability

## Verifying Buffer Size

When you start Phonon, it prints the active buffer size:

```
🎵 Phonon Audio Engine starting...
🎵 Audio: 44100 Hz, 2 channels
🔧 Buffer size: 128 samples (2.9ms latency)
🔧 Using ring buffer architecture for parallel synthesis
```

## Troubleshooting

### Audio Glitches/Crackling
**Problem**: Buffer size too small, CPU can't keep up

**Solution**: Increase buffer size
```bash
PHONON_BUFFER_SIZE=256 phonon live your_code.ph
```

### High Latency (Delayed Response)
**Problem**: Buffer size too large

**Solution**: Decrease buffer size (requires powerful CPU)
```bash
PHONON_BUFFER_SIZE=64 phonon live your_code.ph
```

### No Audio Output
**Problem**: Unrelated to buffer size

**Solution**: Check audio device, sample paths, code syntax

## Technical Details

### Implementation

The buffer size is used in two places:

1. **Synthesis Thread**: Renders audio in chunks of `buffer_size` samples
2. **Audio Stream**: cpal is configured with `BufferSize::Fixed(buffer_size)`

This ensures consistent latency and prevents buffer size mismatches.

### Ring Buffer Architecture

Phonon uses a 2-second ring buffer between synthesis and audio output:
- Synthesis thread fills buffer at its own pace
- Audio callback reads from buffer (faster, lock-free)
- Decouples synthesis from real-time audio (prevents glitches)

The configured buffer size affects synthesis chunk size, not ring buffer size.

## Examples

### Live Coding Session (Low Latency)
```bash
PHONON_BUFFER_SIZE=128 phonon live examples/live_session.ph
```

### Complex FX Chain (More Headroom)
```bash
PHONON_BUFFER_SIZE=512 phonon live examples/synth_and_effects_complete.ph
```

### Performance Mode (Ultra-Low Latency)
```bash
PHONON_BUFFER_SIZE=64 phonon live examples/house_complete.ph
```

## Performance Benchmarks

On a modern CPU (2020+ Intel i7/Ryzen 7):

| Buffer Size | CPU Usage | Stability |
|-------------|-----------|-----------|
| 64 samples  | ~15-20%   | Occasional glitches on complex patches |
| 128 samples | ~10-15%   | Stable for most patches |
| 256 samples | ~8-12%    | Very stable |
| 512 samples | ~5-8%     | Maximum stability |

*Actual performance depends on patch complexity and CPU power*

## Future Enhancements

Planned improvements:
- **Auto-detection**: Measure CPU usage and suggest optimal buffer size
- **Dynamic adjustment**: Change buffer size without restarting
- **Per-device configuration**: Save preferences for different audio interfaces

## See Also

- [Phonon Architecture](ARCHITECTURE.md)
- [Real-Time Audio Performance](PERFORMANCE.md)
- [Live Coding Guide](LIVE_CODING.md)
