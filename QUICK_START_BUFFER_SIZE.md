# Quick Start: Buffer Size Configuration

## TL;DR

**Lower latency** (faster response, more CPU):
```bash
PHONON_BUFFER_SIZE=64 phonon live your_code.ph
```

**Default** (good balance, 3ms latency):
```bash
phonon live your_code.ph
```

**More CPU headroom** (higher latency, more stable):
```bash
PHONON_BUFFER_SIZE=512 phonon live your_code.ph
```

---

## What is Buffer Size?

Buffer size controls how many audio samples are processed at once:
- **Smaller** = faster response (low latency) but needs more CPU power
- **Larger** = slower response (high latency) but more stable on slower CPUs

## Quick Reference Table

| Buffer Size | Latency | Best For | CPU Usage |
|-------------|---------|----------|-----------|
| 64          | 1.5ms   | Live performance (powerful CPU) | High |
| **128**     | **3ms** | **Recommended default** | Medium |
| 256         | 6ms     | Complex patches | Low |
| 512         | 12ms    | Maximum stability | Very Low |

## When to Change Buffer Size

### Use 64 samples (ultra-low latency) if:
- You're performing live and need instant response
- Your CPU is modern (2020+) and powerful
- You're doing live-coding with simple-to-medium patches

### Use 128 samples (DEFAULT) if:
- You want a good balance (most users)
- Your CPU is decent (2015+)
- You're unsure what to choose

### Use 256-512 samples (more headroom) if:
- You hear crackling/glitches with smaller buffers
- Your patch has many effects (reverb, delay, filters, etc.)
- Your CPU is older or slower
- You're running other CPU-intensive apps

## How to Set Buffer Size

Just add `PHONON_BUFFER_SIZE=<number>` before your Phonon command:

```bash
# Live performance mode
PHONON_BUFFER_SIZE=64 phonon live examples/live_session.ph

# Studio production mode
PHONON_BUFFER_SIZE=256 phonon live examples/house_complete.ph
```

## How to Tell If It's Working

When Phonon starts, look for this line:
```
🔧 Buffer size: 128 samples (2.9ms latency)
```

The number should match what you set with `PHONON_BUFFER_SIZE`.

## Troubleshooting

### Problem: Audio is crackling/glitching
**Solution**: Increase buffer size
```bash
PHONON_BUFFER_SIZE=512 phonon live your_code.ph
```

### Problem: Response feels delayed/sluggish
**Solution**: Decrease buffer size (needs powerful CPU)
```bash
PHONON_BUFFER_SIZE=64 phonon live your_code.ph
```

### Problem: Changed buffer size but nothing happened
**Solution**: Make sure you're setting it BEFORE the phonon command:
```bash
# ✅ Correct
PHONON_BUFFER_SIZE=64 phonon live code.ph

# ❌ Wrong
phonon live code.ph PHONON_BUFFER_SIZE=64
```

## Advanced: Set Default Permanently

Edit `src/bin/phonon-audio.rs` line 28:
```rust
const DEFAULT_BUFFER_SIZE: usize = 64; // Change from 128
```

Then rebuild:
```bash
cargo build --release
```

Now you don't need to set the environment variable every time.

## More Information

For detailed documentation, see:
- `docs/BUFFER_SIZE_CONFIGURATION.md` - Complete guide
- `BUFFER_SIZE_IMPLEMENTATION.md` - Technical details
- `examples/buffer_size_demo.sh` - Interactive demo

## Questions?

1. **Does smaller = better?**
   - No! Smaller = lower latency but needs more CPU. Use what works for your system.

2. **What if I set it too small?**
   - Audio will crackle/glitch. Just increase it.

3. **What if I set it too large?**
   - Nothing breaks, but response will feel laggy. Just decrease it.

4. **What if I set an invalid value?**
   - Phonon automatically clamps to safe range (32-2048) or uses default (128).

5. **Does this work in render mode?**
   - No, buffer size only affects live playback. Render mode is offline (no real-time constraints).

## Bottom Line

**For most users**: Don't change anything. The default (128 samples, 3ms) works great.

**For live performers**: Try 64 samples for ultra-low latency if your CPU can handle it.

**For complex patches**: Use 256-512 samples if you hear glitches.

**Experiment!** Try different values and find what works best for your system and workflow.
