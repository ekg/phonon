# Untested Functions Report

**Generated**: 2025-11-23
**Total Functions**: 165
**Functions in Tests**: 111
**Completely Untested**: 39 unique functions (24%)

## Critical Untested Functions

These are high-priority functions that should be tested:

### Oscillators & Generators
- `sine` - Sine wave oscillator
- `saw` - Sawtooth wave oscillator
- `square` - Square wave oscillator
- `cosine` - Cosine wave oscillator
- `sine_trig` - Triggered sine
- `saw_trig` - Triggered sawtooth
- `square_trig` - Triggered square
- `tri_trig` - Triggered triangle

### Filters
- `lpf` - Lowpass filter
- `hpf` - Highpass filter
- `bpf` - Bandpass filter
- `notch` - Notch filter

### Effects
- `reverb` - Reverb effect
- `delay` - Delay effect
- `multitap` - Multi-tap delay
- `pingpong` - Pingpong delay
- `plate` - Plate reverb

### Pattern Parameters (Sample Modifiers)
- `gain` - Gain/volume control
- `pan` - Stereo panning
- `speed` - Playback speed
- `note` - Note/pitch
- `n` - Note number
- `begin` - Sample start point
- `end` - Sample end point
- `loop` - Loop enable
- `unit` - Time unit control
- `cut` - Cut group

### Pattern Transforms
- `every_effect` - Apply effect every N cycles
- `sometimes_effect` - Randomly apply effect
- `sometimes_by_val` - Random transform with probability
- `sometimes_val` - Random value
- `whenmod_effect` - Conditional effect
- `whenmod_val` - Conditional value
- `every_val` - Value every N cycles

### Envelopes & Triggers
- `attack` - ADSR attack time
- `release` - ADSR release time
- `env_trig` - Envelope trigger

### Utilities
- `ar` - Attack-release envelope
- `irand` - Integer random
- `wedge` - Wedge function

## Testing Strategy

### High Priority (Core Features)
1. **Oscillators** - Test all waveforms produce correct frequency/phase
2. **Filters** - Test frequency response and resonance
3. **Sample Parameters** - Test gain, pan, speed, begin/end
4. **Effects** - Test reverb, delay with known inputs

### Medium Priority (Pattern System)
1. **Pattern Transforms** - Test every_effect, sometimes_effect
2. **Envelopes** - Test attack/release shaping
3. **Note Control** - Test note, n parameter routing

### Low Priority (Advanced Features)
1. **Specialized Generators** - Test tri_trig, sine_trig
2. **Advanced Effects** - Test multitap, pingpong, plate
3. **Utilities** - Test wedge, irand edge cases

## Test Template

For each function, create tests following the three-level methodology:

```rust
#[test]
fn test_FUNCTION_NAME_level1_pattern_query() {
    // LEVEL 1: Pattern logic verification
    let pattern = parse_mini_notation("...");
    // Verify event counts over cycles
}

#[test]
fn test_FUNCTION_NAME_level2_onset_detection() {
    // LEVEL 2: Audio event verification
    let audio = render_dsl("...");
    let onsets = detect_audio_events(&audio);
    // Verify timing and event count
}

#[test]
fn test_FUNCTION_NAME_level3_audio_quality() {
    // LEVEL 3: Signal quality verification
    let audio = render_dsl("...");
    // Verify RMS, frequency content, envelope shape
}
```

## Notes

- Some functions may be tested but not detected by simple word matching
- Functions like `lpf`, `hpf` are likely tested via `# lpf` modifiers
- Manual review needed to confirm actual test coverage
- Priority should be given to functions users report as broken

## Next Steps

1. Audit each "untested" function to verify it's truly untested
2. Create test files for high-priority functions first
3. Use TDD workflow: write failing test, implement, verify
4. Update tab completion metadata as tests are added
5. Track progress in this document

## Progress Tracking

- [ ] Oscillators (8 functions)
- [ ] Filters (4 functions)
- [ ] Effects (5 functions)
- [ ] Sample Parameters (10 functions)
- [ ] Pattern Transforms (7 functions)
- [ ] Envelopes (3 functions)
- [ ] Utilities (2 functions)

**Current Status**: 0/39 functions tested (0%)
