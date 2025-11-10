# Sample Playback Fixes Needed in Phonon

Based on research into Tidal Cycles and Strudel implementations, here are the bugs that need fixing in Phonon.

**Created**: 2025-11-10

## Current Issues

### 🔴 CRITICAL: Legato 1 Fades Off (Should Play Full Event Duration)

**Current behavior**: `s "bd" $ legato 1` causes the sample to fade off

**Expected behavior**: Sample should play for exactly the event duration with a clean cut at the end

**Root cause**: Unknown - needs investigation. Likely:
- Envelope is being applied incorrectly
- Release time is not set to a sharp cut
- Attack/release defaults are interfering

**Fix required**:
1. When legato is specified, calculate: `duration_seconds = event_duration_cycles / cps * legato`
2. Apply envelope with minimal attack (0.001s for anti-click) and sharp release
3. Do NOT use a fade-out - use a brick-wall cut or very short release (<0.005s)

### 🔴 CRITICAL: Negative Speed Doesn't Work

**Current behavior**: `s "bd" # speed "-1"` doesn't play the sample backwards (or doesn't play at all)

**Expected behavior**: Sample should play in reverse at normal pitch

**Root cause**: Speed parameter implementation doesn't handle negative values

**Fix required**:
1. Check if speed < 0
2. If yes: reverse the audio buffer before playback
3. Use `abs(speed)` for playback rate calculation
4. Implement buffer reversal function if not present

**Reference implementation** (from Strudel):
```javascript
if (hapValue.speed < 0) {
  buffer = reverseBuffer(buffer);
}
let playbackRate = Math.abs(speed) * Math.pow(2, transpose / 12);
```

### 🟡 IMPORTANT: Legato Default Behavior

**Current behavior**: Needs verification - samples may not play full duration when legato is unspecified

**Expected behavior**:
- When legato is NOT specified: samples play their full duration
- When legato IS specified: samples are cut to `event_duration * legato`

**Fix required**:
1. Distinguish between "legato not specified" (None) vs "legato specified" (Some(value))
2. If None: no envelope duration control (sample plays naturally)
3. If Some(value): apply duration control as above

### 🟡 IMPORTANT: Legato Implementation is Wrong

**Current implementation**: Sets release envelope based on legato duration

**Problems**:
1. Using release time causes fade-out (wrong)
2. No distinction between "legato unspecified" vs "legato specified"
3. Envelope behavior doesn't match Tidal/SuperDirt

**Correct implementation**:
1. legato should control SUSTAIN time, not just release
2. Envelope should be: Attack (very short for anti-click) -> Full amplitude -> Sharp cut at sustain time
3. Release should be extremely short (< 5ms) for a clean cut
4. Sample amplitude stays at 100% until the sustain time, then cuts

**Pseudo-code**:
```rust
if let Some(legato_val) = event.context.get("legato_duration") {
    // legato was specified
    let duration_cycles = legato_val.parse::<f32>()?;
    let duration_seconds = duration_cycles / self.cps;

    // Envelope: instant attack, sustain at full volume, brick-wall release
    let attack = 0.001;  // 1ms anti-click
    let sustain_time = duration_seconds - attack;
    let release = 0.003; // 3ms brick-wall cut

    // Play sample with sharp cutoff at duration_seconds
    trigger_with_envelope(sample, attack, sustain_time, release);
} else {
    // legato not specified - play full sample
    trigger_without_duration_control(sample);
}
```

## Verification Tests Needed

After fixing, these tests must pass:

### Test 1: No Legato = Full Duration
```phonon
tempo: 2.0
out: s "bev ~ ~ ~"
```
**Expected**: "bev" sample plays for its full length (likely >1 second), extending past the next event

### Test 2: Legato 1 = Exact Event Duration
```phonon
tempo: 2.0
out: s "bev bev bev bev" $ legato 1
```
**Expected**: Each "bev" plays for exactly 0.25 cycles (0.125 seconds at tempo 2), with clean cuts (NO fade)

### Test 3: Legato 0.1 = Staccato
```phonon
tempo: 2.0
out: s "bd bd bd bd" $ legato 0.1
```
**Expected**: Very short sharp clicks with lots of silence between

### Test 4: Legato 4 = Overlap
```phonon
tempo: 2.0
out: s "bd ~ bd ~" $ legato 4
```
**Expected**: Each "bd" plays for 2 cycles (1 second at tempo 2), massive overlap

### Test 5: Negative Speed = Reverse
```phonon
tempo: 2.0
out: s "amen" # speed "-1"
```
**Expected**: Amen break plays backwards at normal pitch

### Test 6: Negative Speed with Different Rate
```phonon
tempo: 2.0
out: s "amen" # speed "-0.5"
```
**Expected**: Amen break plays backwards at half speed (lower pitch)

### Test 7: Speed Pattern with Negative
```phonon
tempo: 2.0
out: s "bd" # speed "1 -1 2 -2"
```
**Expected**: Forward normal, backward normal, forward 2x, backward 2x

### Test 8: Zero Speed = No Playback
```phonon
tempo: 2.0
out: s "bd" # speed 0
```
**Expected**: No sound (silence or error - both acceptable)

## Priority Order

1. **Fix negative speed** (CRITICAL - fundamental feature)
2. **Fix legato envelope** (CRITICAL - currently causes fade-out)
3. **Fix legato default behavior** (IMPORTANT - samples should play full duration by default)

## Implementation Notes

### Voice Manager Changes Needed

The voice manager (`src/voice_manager.rs`) likely needs:

1. **Separate methods**:
   - `trigger_sample_full_duration()` - for when no duration control
   - `trigger_sample_with_duration()` - for legato/sustain control

2. **Envelope application**:
   - Current: probably uses attack/release which causes fade
   - Needed: sharp cutoff at specified time (gain goes to 0 instantly)

3. **Reverse playback**:
   - Add buffer reversal function
   - Check speed sign before playback
   - Store reversed buffers in cache to avoid repeated reversals

### Sample Loader Changes Needed

The sample loader (`src/sample_loader.rs`) may need:

1. **Buffer reversal utility**:
```rust
fn reverse_buffer(buffer: &[f32]) -> Vec<f32> {
    buffer.iter().rev().copied().collect()
}
```

2. **Caching reversed samples** (optional optimization):
   - Store both forward and reverse versions
   - Or compute on-demand and cache

### Unified Graph Changes Needed

In `src/unified_graph.rs` (sample playback section):

1. **Remove current legato implementation** (lines 5974-5982) - it's wrong

2. **Add correct legato logic**:
```rust
let duration_control = if let Some(legato_str) = event.context.get("legato_duration") {
    // legato specified - control duration
    let duration_cycles = legato_str.parse::<f32>()?;
    let duration_seconds = duration_cycles / self.cps;
    Some(duration_seconds)
} else {
    // legato not specified - play full sample
    None
};
```

3. **Check speed sign**:
```rust
let speed_val = self.eval_signal_at_time(&speed, event_start_abs);
let should_reverse = speed_val < 0.0;
let playback_speed = speed_val.abs();
```

4. **Trigger appropriately**:
```rust
if should_reverse {
    // Reverse buffer first
    let reversed = Arc::new(reverse_buffer(&sample_data));
    self.voice_manager.borrow_mut().trigger_sample(
        reversed,
        gain_val,
        pan_val,
        playback_speed,
        duration_control,
    );
} else {
    self.voice_manager.borrow_mut().trigger_sample(
        sample_data,
        gain_val,
        pan_val,
        playback_speed,
        duration_control,
    );
}
```

## Testing Strategy

For each fix:

1. **Unit test** the specific function (buffer reversal, envelope generation)
2. **Integration test** with actual audio rendering
3. **Onset detection** to verify timing
4. **Manual listening** to verify no fades/clicks/artifacts
5. **Compare to Strudel** online REPL with same pattern

## References

See `docs/SAMPLE_PLAYBACK_BEHAVIOR.md` for complete specification based on Tidal Cycles and Strudel research.
