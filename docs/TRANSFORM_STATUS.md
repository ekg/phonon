# Transform Status - What Works and What's Broken

**Last tested**: 2025-11-10

## ✅ Working Transforms

- **fast** - speeds up pattern
- **slow** - slows down pattern
- **rev** - reverses event order
- **palindrome** - pattern + reverse
- **stutter** - repeat events
- **shuffle** - randomize timing
- **scramble** - randomize event order
- **every** - apply transform every N cycles
- **degradeBy** - randomly remove events with probability
- **chop** - chop samples into pieces
- **chunk** - apply transform to chunks
- **loopAt** - fit sample to N cycles

## 🔴 Broken Transforms (Produce Silence)

- **striate** - produces complete silence
- **slice** - produces complete silence

## ⚠️ Broken Parameters

- **legato** - causes fade-off instead of sharp cut
- **speed (negative values)** - doesn't reverse playback

## Test Results

### striate
```phonon
tempo: 0.5
out: s "amen" $ striate 8
```
**Result**: Complete silence (RMS: 0.000, Peak: 0.000)

### slice
```phonon
tempo: 0.5
out: s "amen" $ slice 8 "0 7 2 5"
```
**Result**: Complete silence (RMS: 0.000, Peak: 0.000)

### All other tested transforms
Produced audible output as expected.

## Priority Fixes

1. **striate** - Core sample slicing feature, shouldn't be silent
2. **slice** - Core sample slicing feature, shouldn't be silent
3. **legato** - Creates fade artifacts (should be sharp cut)
4. **negative speed** - Essential for live coding

## Notes

- Most pattern transforms (rev, palindrome, fast, slow, etc.) work correctly
- Sample chopping transforms (striate, slice) are completely broken
- Envelope/duration controls need rework per SAMPLE_PLAYBACK_BEHAVIOR.md
