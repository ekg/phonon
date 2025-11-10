# Fixing Progress Tracker

**Started**: 2025-11-10
**Status**: In Progress

## Priority 0: Critical Bugs (Blocks common patterns)

### P0.1: Fix legato envelope ⏳ IN PROGRESS
**Problem**: legato causes fade-off instead of sharp cut
**Root cause**: PercussionEnvelope uses slow exponential decay `exp(-5.0 * t)`
**Solution**: Make envelope ultra-sharp:
- Attack: 0.001s (instant)
- Sustain: 100% amplitude for legato duration
- Release: 0.003s (brick-wall)

**Implementation plan**:
1. Modify PercussionEnvelope or use ADSR with sharp settings
2. For legato, set: attack=0.001, decay=0.001, sustain=1.0, release=0.003
3. Calculate sustain duration from legato value: `sustain_time = (legato_duration_cycles / cps) - attack - release`

**Testing**:
- `legato 0.1`: Very short staccato notes
- `legato 1.0`: Exact event duration, sharp cut
- `legato 2.0`: 2x duration with overlap

---

### P0.2: Fix striate ⏸️ PENDING
**Problem**: Produces complete silence
**Root cause**: Pattern zoom works but doesn't set begin/end for sample slicing
**Solution**: Connect striate to begin/end parameters

**Implementation plan**:
1. When striate is applied to sample patterns
2. Add begin/end values to event context
3. unified_graph.rs already handles begin/end (lines 6175-6190)

---

### P0.3: Fix slice ⏸️ PENDING
**Problem**: Produces complete silence
**Root cause**: Same as striate
**Solution**: Same as striate

---

### P0.4: Implement jux ⏸️ PENDING
**Problem**: Missing entirely
**Solution**: Apply transform to right channel only

**Implementation plan**:
1. Add Jux transform to enum
2. Implement in pattern operations
3. Connect to stereo panning

---

## Progress Log

### 2025-11-10 Evening Session

**Completed**:
- ✅ Deep research into Tidal Cycles sample playback behavior
- ✅ Created SAMPLE_PLAYBACK_BEHAVIOR.md (comprehensive spec)
- ✅ Created SAMPLE_PLAYBACK_FIXES_NEEDED.md (implementation guide)
- ✅ Created TIDAL_PARITY_STATUS.md (complete audit)
- ✅ Tested all major transforms
- ✅ Identified root causes of all P0 bugs

**Discovered**:
- Negative speed DOES work (buffer reversal already implemented)
- 11 transforms confirmed working
- legato issue is envelope shape, not architecture
- striate/slice have infrastructure but need connection

**In Progress**:
- Fixing legato envelope sharpness

**Next Steps**:
1. Finish legato fix
2. Test legato thoroughly
3. Fix striate/slice
4. Implement jux

---

## Testing Checklist

After each fix, verify with:

### Legato Tests
```phonon
# Test 1: Staccato (legato 0.1)
tempo: 2.0
out: s "bd sn hh cp" $ legato 0.1
# Expected: Very short clicks, lots of silence

# Test 2: Exact duration (legato 1.0)
tempo: 2.0
out: s "bd sn hh cp" $ legato 1.0
# Expected: Each sample exactly 0.125s, sharp cut, no fade

# Test 3: Overlap (legato 2.0)
tempo: 2.0
out: s "bd sn hh cp" $ legato 2.0
# Expected: Each sample 0.25s, overlapping
```

### Striate/Slice Tests
```phonon
# Test 1: Striate
tempo: 2.0
out: s "amen" $ striate 8
# Expected: Amen break chopped into 8 pieces, played together

# Test 2: Slice
tempo: 2.0
out: s "amen" $ slice 8 "0 7 2 5"
# Expected: Slices 0, 7, 2, 5 of amen break in sequence
```

### Jux Test
```phonon
# Test 1: Basic jux
tempo: 2.0
out: s "bd sn" $ jux rev
# Expected: Left channel normal, right channel reversed
```

---

## Notes

- User preference: Study → Implement → Test → Mark Progress
- All documentation in docs/ folder
- Test files in /tmp for quick iteration
- Commit after each completed fix
