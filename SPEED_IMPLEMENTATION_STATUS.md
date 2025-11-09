# Speed Parameter Implementation Status

## Summary

Implemented reverse playback support via negative `speed` parameter values, matching TidalCycles functionality. Core implementation is complete but needs debugging.

## What's Been Implemented

### 1. Voice Manager (src/voice_manager.rs)

**Changes to all trigger methods:**
- Removed `.max(0.01)` speed clamping
- Added smart initial position based on speed direction:
  - Forward (speed > 0): Start at position 0
  - Reverse (speed < 0): Start at end of sample (len - 1)

**Changes to `process_stereo()`:**
- Added boundary checking for both forward and reverse playback
- Added reverse interpolation logic (interpolates backward for negative speed)
- Added reverse looping support

**Files Modified:**
- `trigger_with_envelope()` - lines 209-232
- `trigger_with_adsr()` - lines 246-268
- `trigger_with_segments()` - lines 280-300
- `trigger_with_curve()` - lines 314-334
- `process_stereo()` - lines 390-456

### 2. Unified Graph (src/unified_graph.rs)

**Speed Value Evaluation:**
- Changed from `.max(0.01).min(10.0)` to `.clamp(-10.0, 10.0)`
- Now allows negative values for reverse playback
- Line 5922

## Current Status

### ✅ Working

- Forward playback: `s "bd"` → Peak: 0.513 ✓
- Speed parameter infrastructure exists
- Reverse playback logic implemented

### ⚠️ Needs Debugging

**Issue**: When using speed modifier (`# speed`), output volume is very low

```phonon
s "bd"              # Peak: 0.513 (normal) ✓
s "bd" # speed 1.0  # Peak: ???  (should be ~0.5)
s "bd" # speed -1.0 # Peak: 0.010 (too low!) ✗
```

**Debug Output:**
```
[VOICE_MGR] trigger_sample_with_envelope called: sample_len=12532, gain=1.000, pan=0.000, speed=1.000
[VOICE_MGR] trigger_sample_with_envelope called: sample_len=12532, gain=1.000, pan=0.000, speed=0.010
```

The second call shows `speed=0.010` instead of `-1.0`, suggesting the speed modifier function may have an issue.

## Syntax

Currently working:
```phonon
# Normal playback
out: s "bd"

# Forward at different speeds
out: s "bd" # speed 2.0   # Double speed (pitch up)
out: s "bd" # speed 0.5   # Half speed (pitch down)

# Reverse playback
out: s "bd" # speed -1.0  # Reverse at normal speed
out: s "bd" # speed -0.5  # Reverse at half speed

# Pattern-based speed
out: s "bd*4" # speed "1 2 -1 0.5"  # Different speed per hit
```

## What Still Needs Work

1. **Debug speed modifier** - Fix the issue causing low volume when using `# speed`
2. **Test forward playback** - Verify `# speed 2.0` works correctly
3. **Test reverse playback** - Verify `# speed -1.0` produces reversed audio
4. **Add unit tests** - Write tests for both forward and reverse playback
5. **Add `begin` and `end` parameters** - Sample slicing (next priority from TidalCycles comparison)

## Testing

To test once debugged:

```bash
# Test normal vs reverse
cat > /tmp/test.ph << 'EOF'
tempo: 2.0
~fwd: s "bd" # speed 1.0
~rev: s "bd" # speed -1.0
out: ~fwd + ~rev
EOF

cargo run --release --bin phonon -- render /tmp/test.ph /tmp/test.wav --cycles 2

# Should hear kick drum forward then backward
```

## Related Documentation

- **TIDAL_FEATURE_COMPARISON.md**: List of missing TidalCycles features
- **REVERSAL_AND_TRANSFORM_COMPOSITION.md**: Original exploration of reversal concepts
- **voice_manager.rs**: Voice playback engine
- **unified_graph.rs**: Signal evaluation and sample triggering

## Next Steps

1. Debug why `compile_speed_modifier` is producing `speed=0.010`
2. Check if `modify_sample_param()` is correctly updating the sample node
3. Verify that the modified sample node is being used for triggering
4. Once speed works, add tests
5. Move on to `begin` and `end` parameters
