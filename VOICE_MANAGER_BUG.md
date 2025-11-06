# Critical Bug: 2x Event Duplication in Audio Rendering

**Status:** 🔴 CRITICAL - Events are being rendered twice in audio output

**Discovered:** 2025-11-05 during unit/loop implementation

## Symptoms

- `tempo: 0.5` sounds like 240 BPM instead of 120 BPM
- Audio contains 2x the expected number of sound events  
- Events come in pairs with consistent ~0.21s spacing
- Affects all sample playback (bd, cp, etc.)

## Evidence

Test: `tempo: 0.5` with `s "bd"` over 4 cycles

**Expected:**
- Duration: 8 seconds ✓
- Triggers: 4 ✓  
- Audio events: 4 kicks ✓

**Actual:**
- Duration: 8 seconds ✓ (correct)
- Debug triggers: 4 ✓ (correct)
- Audio events: 8 kicks ❌ (2x too many!)

Onset times: 0.00s, 0.21s, 2.00s, 2.21s, 3.99s, 4.20s, 5.99s, 6.20s
(Pairs spaced 0.21s, pairs themselves 2s apart)

## Root Cause

Bug is in audio rendering/voice playback stage, NOT in:
- Event triggering (correct)
- Tempo calculation (correct)  
- Sample structure (BD has 1 peak)
- Multi-output mixing (single output has same issue)

## Reproduction

```bash
cat > test.ph << 'TEST'
tempo: 0.5
out: s "bd"
TEST

cargo run --release --bin phonon -- render test.ph out.wav --cycles 4
# Audio will have 8 kicks instead of 4
```

## Impact

**CRITICAL** - All rhythms play at 2x intended speed

## Debugging Progress (2025-11-05)

### Tests Performed:

1. **Voice Trigger Count**: ✅ Only ONE call to `trigger_sample_with_envelope()` per event
2. **Active Voice Count**: ✅ Only 1 voice active throughout playback
3. **Voice Processing**: ✅ Each voice's `process_stereo()` called once per sample, position advances correctly
4. **WAV Writing**: ✅ Each sample written once to output buffer

### Remaining Mystery:

Despite all checks passing, onset detection finds 2x events in the audio. Theories:
1. Voice audio rendering has a subtle bug multiplying output
2. Onset detection algorithm is flawed (picking up echoes/reverb?)
3. Sample rate or timing issue causing apparent duplication

### Next Steps:

1. Add debug logging to `voice_output_cache` value
2. Log final `mixed_output` value before writing
3. Compare raw sample values between expected and actual
4. Test with extremely simple case (1 sample, no effects)
5. Check if issue exists with older commits
