# Feedback Routing Examples

This directory contains examples demonstrating various feedback routing and signal flow patterns in Phonon.

## Examples

### 01_dub_delay.ph
**Classic dub techno delay with HPF in feedback loop**

Demonstrates how to use delay feedback with high-pass filtering to prevent low-frequency buildup. This is the signature sound of dub techno and dub reggae.

**Key Concepts:**
- Delay feedback parameter
- HPF in feedback path to control tone
- Wet/dry mixing

**Run it:**
```bash
cargo run --bin phonon -- render --cycles 8 docs/examples/feedback_routing/01_dub_delay.ph output.wav
```

---

### 02_multi_tap_delay.ph
**Multiple delay taps create dense rhythmic textures**

Shows how to create multiple delay taps from a single source, similar to diffusion in reverb algorithms. Each tap has its own delay time and feedback amount.

**Key Concepts:**
- Multiple delay taps from one source
- Different delay times create rhythmic complexity
- Mixing multiple paths together

**Run it:**
```bash
cargo run --bin phonon -- render --cycles 8 docs/examples/feedback_routing/02_multi_tap_delay.ph output.wav
```

---

### 03_parallel_effects.ph
**Split signal into parallel paths with different processing**

Demonstrates parallel signal routing: splitting a signal, processing each path differently, and mixing back together. Common in professional mixing.

**Key Concepts:**
- Signal splitting (one input, multiple outputs)
- Different processing on each path
- Balanced mixing of parallel paths

**Run it:**
```bash
cargo run --bin phonon -- render --cycles 8 docs/examples/feedback_routing/03_parallel_effects.ph output.wav
```

---

### 04_send_return_reverb.ph
**Classic mixing technique: multiple sources share one reverb**

Shows the send/return mixing pattern where multiple sources feed a shared effect (like an aux send on a mixing console).

**Key Concepts:**
- Multiple sources to one effect
- Send/return routing
- Wet/dry balance

**Run it:**
```bash
cargo run --bin phonon -- render --cycles 8 docs/examples/feedback_routing/04_send_return_reverb.ph output.wav
```

---

### 05_filter_sweep_feedback.ph
**LFO-modulated filter combined with delay feedback**

Demonstrates how to modulate filter parameters with LFOs while using delay feedback, creating evolving, rhythmic textures.

**Key Concepts:**
- LFO modulation of filter cutoff
- Pattern-controlled parameters
- Combining modulation with feedback

**Run it:**
```bash
cargo run --bin phonon -- render --cycles 8 docs/examples/feedback_routing/05_filter_sweep_feedback.ph output.wav
```

---

### 06_fm_synthesis.ph
**Frequency modulation creates complex harmonic spectra**

Shows how to use FM synthesis with multiple modulators to create rich, evolving timbres.

**Key Concepts:**
- Frequency modulation (FM)
- Multiple modulators affecting carrier
- Audio-rate vs. control-rate modulation

**Run it:**
```bash
cargo run --bin phonon -- render --cycles 8 docs/examples/feedback_routing/06_fm_synthesis.ph output.wav
```

---

## ✨ Circular Bus Dependencies Now Supported!

**Good news:** Circular bus dependencies ARE fully supported via two-pass compilation!

```phonon
-- ✅ Self-referential feedback WORKS:
~feedback: ~feedback * 0.7 + ~input * 0.3

-- ✅ Two-bus cycles WORK:
~a: ~b # lpf 1000 0.8
~b: ~a # delay 0.1 0.5

-- ✅ Three-bus cycles WORK:
~a: ~b # lpf 1000 0.8
~b: ~c # delay 0.1 0.5
~c: ~a # reverb 0.8 0.4 0.7
```

Both approaches work - choose based on your needs:
1. **Effect feedback parameters** (simpler for single delay/reverb)
2. **Circular bus dependencies** (more flexible for complex routing)
3. **Cascaded buses** (still useful for acyclic graphs)
4. **Parallel routing** (split, process, mix)

## Common Patterns

### Delay Feedback
```phonon
~delayed: ~input # delay 0.25 0.7  -- 0.7 = feedback amount
```

### Cascaded Effects
```phonon
~path1: ~input # delay 0.125 0.5
~path2: ~path1 # reverb 0.8 0.4 0.7
~output: ~path2 # lpf 2000 0.8
```

### Parallel Routing
```phonon
~dry: ~input
~wet: ~input # delay 0.125 0.5 # reverb 0.8 0.4 0.7
out: ~dry * 0.5 + ~wet * 0.5
```

### Signal Splitting
```phonon
~source: saw 110
~tap1: ~source # delay 0.037 0.7
~tap2: ~source # delay 0.043 0.7
~tap3: ~source # delay 0.051 0.7
out: (~tap1 + ~tap2 + ~tap3) * 0.3
```

## Testing

All patterns are comprehensively tested:

### Circular Dependency Tests (16 tests)
```bash
cargo test --test test_circular_dependencies
```

Covers:
- Self-referential feedback
- Two-bus cycles
- Three-bus cycles
- Complex patterns (FM in feedback, cross-feedback networks, Karplus-Strong)

### General Feedback Routing Tests (24 tests)
```bash
cargo test --test test_feedback_routing_patterns
```

Covers:
- Delay feedback
- Reverb feedback
- Parallel effects routing
- FM synthesis
- Mix operators
- Production scenarios

**Total: 40 tests, 100% passing**

## See Also

- `docs/FEEDBACK_ROUTING.md` - Complete documentation on feedback routing architecture
- `tests/test_feedback_routing_patterns.rs` - Comprehensive tests for all patterns
