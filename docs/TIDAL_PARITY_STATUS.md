# Tidal Cycles Parity Status

**Last updated**: 2025-11-10 (Updated after P0/P1 fixes)

This document tracks implementation status of Tidal Cycles transforms and functions in Phonon.

## Recent Fixes (2025-11-10)

All P0 and P1 critical issues have been resolved! ✅

- **jux/juxBy** - ✅ IMPLEMENTED - Stereo panning with transforms
- **loopAt** - ✅ ENHANCED - Now supports pattern parameters
- **striate** - ✅ FIXED - Sample slicing with begin/end context
- **slice** - ✅ FIXED - Sample slicing with begin/end context
- **legato** - ✅ FIXED - ADSR envelope with auto-release
- **Transform chains** - ✅ WORKING - Parenthesized chains like `jux (fast 2 $ rev)`

## Time/Pattern Transforms

| Transform | Status | Notes |
|-----------|--------|-------|
| fast | ✅ Works | |
| slow | ✅ Works | |
| rev | ✅ Works | Reverses event order |
| palindrome | ✅ Works | Pattern + reverse |
| iter | ✅ Works | |
| iterBack | ✅ Works | |
| loopAt | ✅ Works | Supports both constant and pattern parameters |
| chop | ✅ Works | |
| striate | ✅ Works | Fixed via begin/end context |
| slice | ✅ Works | Fixed via begin/end context |
| splice | ❌ Missing | Like slice but adjusts speed |
| stut | ❌ Missing | Stutter/echo |
| echo | ⚠️ Defined | Not tested |
| jux | ✅ Works | Stereo panning with pan context |
| juxBy | ✅ Works | Pan amount controllable |
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
| speed | ✅ Works | Negative works! Context override supported |
| gain | ✅ Works | |
| pan | ✅ Works | Context override for jux |
| legato | ✅ Works | ADSR envelope with auto-release |
| sustain | ❌ Missing | |
| begin | ✅ Works | Sample slice start (context override) |
| end | ✅ Works | Sample slice end (context override) |
| cut | ✅ Works | Voice choking |
| n | ✅ Works | Sample number |
| attack | ✅ Works | ADSR attack phase |
| release | ✅ Works | ADSR release phase |

## Critical Bugs

### ✅ ALL RESOLVED (2025-11-10)

1. ✅ **legato** - FIXED via ADSR envelope with auto-release
2. ✅ **striate** - FIXED via begin/end sample slicing context
3. ✅ **slice** - FIXED via begin/end sample slicing context
4. ✅ **jux/juxBy** - IMPLEMENTED with pan context override
5. ✅ **loopAt patterns** - IMPLEMENTED with pattern-based durations
6. ✅ **Transform chains** - IMPLEMENTED via Transform::Compose

## Testing Status

✅ = Tested and works
⚠️ = Defined in enum but not tested
🔴 = Tested and broken
❌ = Not implemented at all

## Priority for Implementation

### ✅ P0 - Critical (ALL COMPLETE!)
1. ✅ Fix legato
2. ✅ Fix striate
3. ✅ Fix slice
4. ✅ Implement jux/juxBy

### ✅ P1 - High (ALL COMPLETE!)
5. ✅ Implement begin/end parameters
6. ✅ Enhance loopAt for patterns
7. ✅ Implement transform chains

### P2 - High (Missing features from livecode - by frequency)
1. **struct** - Apply structure from pattern to another (284 uses in livecode!)
2. **stut** - Stutter/echo effect (132 uses in livecode)
3. **hurry** - Speed up and pitch (37 uses)
4. **off** - Offset transform for delays (35 uses)
5. **foldEvery** - Conditional transform (7 uses: `foldEvery [2,3,4] (fast 2)`)

### P3 - Medium (Less common but useful)
6. **compress** - Window into pattern (6 uses)
7. **sew** - Pattern switcher (5 uses)
8. **sustain** - Sample parameter
9. **splice** - Like slice but adjusts speed
10. **when** - Conditional transform
11. Test all ⚠️ transforms (sometimes, often, rarely, etc.)

## Notes

- Many transforms are defined in the Transform enum but not tested
- Some may work but haven't been verified
- Some may be partially implemented in pattern operations but not exposed to DSL
- Negative speed DOES work for reverse playback (confirmed)
