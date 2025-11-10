# Tidal Cycles Parity Status

**Last updated**: 2025-11-10

This document tracks implementation status of Tidal Cycles transforms and functions in Phonon.

## Critical Missing Transforms

These are commonly used in Tidal patterns and must be implemented:

- **jux** - Apply transform to one stereo channel (jux rev = reverse on right channel)
- **weave** - Weave pattern with transform
- **striate** - IMPLEMENTED but BROKEN (produces silence)
- **slice** - IMPLEMENTED but BROKEN (produces silence)

## Time/Pattern Transforms

| Transform | Status | Notes |
|-----------|--------|-------|
| fast | ✅ Works | |
| slow | ✅ Works | |
| rev | ✅ Works | Reverses event order |
| palindrome | ✅ Works | Pattern + reverse |
| iter | ✅ Works | |
| iterBack | ✅ Works | |
| loopAt | ✅ Works | |
| chop | ✅ Works | |
| striate | 🔴 BROKEN | Produces silence |
| slice | 🔴 BROKEN | Produces silence |
| splice | ❌ Missing | Like slice but adjusts speed |
| stut | ❌ Missing | Stutter/echo |
| echo | ⚠️ Defined | Not tested |
| jux | ❌ Missing | Essential for stereo |
| juxBy | ❌ Missing | |
| weave | ⚠️ Defined | Not exposed to DSL |

## Event Modification

| Transform | Status | Notes |
|-----------|--------|-------|
| stutter | ✅ Works | Repeat events |
| ply | ⚠️ Defined | Not tested |
| shuffle | ✅ Works | Randomize timing |
| scramble | ✅ Works | Randomize order |
| degrade | ✅ Works | Random removal |
| degradeBy | ✅ Works | |
| degradeSeed | ⚠️ Defined | Not tested |
| undegrade | ⚠️ Defined | Not tested |

## Conditional Transforms

| Transform | Status | Notes |
|-----------|--------|-------|
| every | ✅ Works | Apply every N cycles |
| whenmod | ⚠️ Defined | Not tested |
| foldEvery | ❌ Missing | |
| when | ❌ Missing | |
| sometimes | ⚠️ Defined | Not tested |
| often | ⚠️ Defined | Not tested |
| rarely | ⚠️ Defined | Not tested |
| almostAlways | ⚠️ Defined | Not tested |
| almostNever | ⚠️ Defined | Not tested |

## Timing Adjustments

| Transform | Status | Notes |
|-----------|--------|-------|
| early | ⚠️ Defined | Not tested |
| late | ⚠️ Defined | Not tested |
| swing | ⚠️ Defined | Not tested |
| hurry | ❌ Missing | |

## Structure

| Transform | Status | Notes |
|-----------|--------|-------|
| chunk | ✅ Works | Apply transform to chunks |
| inside | ⚠️ Defined | Not tested |
| outside | ⚠️ Defined | Not tested |
| within | ⚠️ Defined | Not tested |
| superimpose | ⚠️ Defined | Not tested |

## Spatial/Time Windows

| Transform | Status | Notes |
|-----------|--------|-------|
| zoom | ⚠️ Defined | Not tested |
| compress | ⚠️ Defined | Not tested |
| focus | ⚠️ Defined | Not tested |
| fastGap | ⚠️ Defined | Not tested |
| gap | ⚠️ Defined | Not tested |

## Sample Parameters

| Parameter | Status | Notes |
|-----------|--------|-------|
| speed | ⚠️ Partial | Negative works! But see below |
| gain | ✅ Works | |
| pan | ✅ Works | |
| legato | 🔴 BROKEN | Has no effect |
| sustain | ❌ Missing | |
| begin | ❌ Missing | |
| end | ❌ Missing | |
| cut | ✅ Works | Voice choking |
| n | ✅ Works | Sample number |
| attack | ⚠️ Partial | Works but see legato issue |
| release | ⚠️ Partial | Works but causes fade |

## Critical Bugs

### 1. legato has no effect
**Status**: BROKEN
**Impact**: HIGH - can't control note duration
**Fix needed**: Implement proper legato per SAMPLE_PLAYBACK_BEHAVIOR.md

### 2. striate produces silence
**Status**: BROKEN
**Impact**: HIGH - essential sample chopping feature
**Fix needed**: Debug striate implementation

### 3. slice produces silence
**Status**: BROKEN
**Impact**: HIGH - essential sample slicing feature
**Fix needed**: Debug slice implementation

### 4. jux missing
**Status**: MISSING
**Impact**: HIGH - essential for stereo patterns
**Fix needed**: Implement jux transform

## Testing Status

✅ = Tested and works
⚠️ = Defined in enum but not tested
🔴 = Tested and broken
❌ = Not implemented at all

## Priority for Implementation

### P0 - Critical (Blocks common patterns)
1. Fix legato (currently has no effect)
2. Fix striate (produces silence)
3. Fix slice (produces silence)
4. Implement jux (essential for stereo)

### P1 - High (Commonly used)
5. Implement begin/end parameters
6. Implement sustain parameter
7. Implement stut/echo properly
8. Test and fix all ⚠️  transforms

### P2 - Medium (Less common but useful)
9. Implement juxBy
10. Implement splice
11. Implement hurry
12. Implement when/foldEvery

## Notes

- Many transforms are defined in the Transform enum but not tested
- Some may work but haven't been verified
- Some may be partially implemented in pattern operations but not exposed to DSL
- Negative speed DOES work for reverse playback (confirmed)
