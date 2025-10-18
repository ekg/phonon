# Synthesis Quick Reference

## synth() Function

```phonon
synth NOTES WAVEFORM ATTACK DECAY SUSTAIN RELEASE [GAIN] [PAN]
```

### Common Envelope Presets

| Sound Type | Attack | Decay | Sustain | Release | Example |
|-----------|--------|-------|---------|---------|---------|
| **Percussive** | 0.001 | 0.05 | 0.0 | 0.1 | Kick, snare, pluck |
| **Pad** | 0.5 | 0.3 | 0.8 | 1.0 | Strings, atmosphere |
| **Lead** | 0.01 | 0.1 | 0.7 | 0.3 | Synth lead, melody |
| **Organ** | 0.001 | 0.0 | 1.0 | 0.1 | Organ, held notes |
| **Bell** | 0.001 | 0.5 | 0.0 | 0.8 | Bells, metallic |

### Waveforms

| Waveform | Sound Character | Use Case |
|----------|----------------|----------|
| `"sine"` | Pure, smooth | Sub-bass, pads, modulation |
| `"saw"` | Bright, buzzy | Leads, bass, rich timbres |
| `"square"` | Hollow, nasal | Bass, chiptune, retro |
| `"triangle"` | Soft, mellow | Subtle leads, flutes |

## Routing Patterns

### Auto-Routing (TidalCycles Style)

```phonon
~d1: synth "c4" "saw" 0.01 0.1 0.7 0.3    # Auto-routes
~d2: synth "e4" "saw" 0.01 0.1 0.7 0.3    # Auto-routes
# Both automatically sum to master output
```

### Manual Routing

```phonon
~bass: synth "c3" "square" 0.001 0.05 0.0 0.1
~lead: synth "c5" "saw" 0.01 0.1 0.7 0.3
~master: ~bass * 0.6 + ~lead * 0.4         # Explicit mix
```

## Effects Chain Syntax

```phonon
~signal: SOURCE # EFFECT1 # EFFECT2 # EFFECT3
```

### Common Effects

```phonon
# Low-pass filter
... # lpf CUTOFF_HZ Q_FACTOR
... # lpf 800 0.8

# High-pass filter
... # hpf CUTOFF_HZ Q_FACTOR
... # hpf 100 0.5

# Reverb
... # reverb ROOM_SIZE DAMPING MIX
... # reverb 0.6 0.5 0.2

# Distortion
... # dist DRIVE MIX
... # dist 2.0 0.3

# Delay
... # delay TIME FEEDBACK MIX
... # delay 0.5 0.5 0.3
```

## Pattern Triggering

```phonon
# Each event in the pattern triggers a new voice with ADSR
~melody: synth "c4 e4 g4 c5" "saw" 0.01 0.1 0.7 0.3

# Events:  c4    e4    g4    c5
# Voices:  [ADSR][ADSR][ADSR][ADSR]
# All active voices are mixed together
```

## Complete Example Patterns

### Minimal (Auto-Routing)

```phonon
cps: 2.0
~d1: synth "c4 e4 g4" "saw" 0.01 0.1 0.7 0.3
```

### With Effects

```phonon
cps: 2.0
~d1: synth "c4 e4 g4" "saw" 0.01 0.1 0.7 0.3 # lpf 1200 0.8
~master: ~d1 # reverb 0.5 0.5 0.2
```

### Multi-Channel

```phonon
cps: 2.0
~d1: synth "c3*4" "square" 0.001 0.05 0.0 0.1        # Bass
~d2: synth "c5 e5 g5" "saw" 0.01 0.1 0.7 0.3         # Lead
~master: (~d1 * 0.6 + ~d2 * 0.4) # reverb 0.5 0.5 0.15
```

### Send/Return (Parallel Effects)

```phonon
cps: 2.0
~drums: synth "c4*8" "saw" 0.001 0.05 0.0 0.1
~reverb_send: ~drums # reverb 0.8 0.5 1.0           # 100% wet
~master: ~drums * 0.7 + ~reverb_send * 0.3          # Dry/wet blend
```

## Key Differences

### Bare Oscillator
```phonon
~drone: saw 110    # Continuous tone, no envelope, no pattern
```

### Synth Pattern
```phonon
~melody: synth "a3" "saw" 0.01 0.1 0.7 0.3    # Triggered note with envelope
```

## Troubleshooting

**No sound?**
- Check that pattern has notes (not all rests `~`)
- Verify ADSR values are reasonable (not all 0)
- Ensure master routing exists (auto or explicit)

**Clipping (distortion)?**
- Reduce gain values (`* 0.3` or lower)
- Lower sustain level in ADSR
- Reduce number of simultaneous voices

**Sound too short/long?**
- Adjust release time (longer = more tail)
- Check sustain level (0 = stops after decay)
- Verify pattern timing matches tempo

**Filters not working?**
- Check cutoff frequency is reasonable (20-20000 Hz)
- Verify Q factor is 0.5-10.0
- Make sure effect comes AFTER source in chain
