# Strudel Operator Audit for Phonon

This document tracks our progress implementing all Strudel/TidalCycles operators in Rust.

## ✅ Implemented Core Operators (pattern.rs, pattern_ops.rs)

### Time Manipulation
- [x] fast - Speed up pattern
- [x] slow - Slow down pattern  
- [x] rev - Reverse pattern
- [x] late - Delay pattern
- [x] early - Advance pattern
- [x] offset - Shift in time
- [x] rotate_left - Rotate pattern left
- [x] rotate_right - Rotate pattern right

### Structural
- [x] stack - Layer patterns
- [x] cat - Concatenate patterns
- [x] slowcat - One pattern per cycle
- [x] fastcat - All patterns in one cycle
- [x] overlay - Combine two patterns
- [x] append - Add pattern to end
- [x] palindrome - Forward then backward
- [x] loop_pattern - Loop n times

### Conditional
- [x] every - Apply function every n cycles
- [x] when_mod - Apply when cycle mod n equals m
- [x] sometimes - Apply 50% of the time
- [x] rarely - Apply 25% of the time
- [x] often - Apply 75% of the time
- [x] always - Apply always

### Probabilistic  
- [x] degrade - Remove 50% randomly
- [x] degrade_by - Remove by probability
- [x] degrade_seed - Degrade with seed

### Pattern Manipulation
- [x] dup - Duplicate events n times
- [x] stutter - Repeat each event
- [x] chunk - Apply function to chunks
- [x] jux - Stereo split
- [x] jux_rev - Jux with reverse

### Numeric Operations
- [x] add - Add value
- [x] mul - Multiply value
- [x] sub - Subtract value
- [x] div - Divide value

## ✅ Implemented Extended Operators (pattern_ops_extended.rs)

### Advanced Time
- [x] zoom - Focus on portion
- [x] focus - Focus on specific cycle
- [x] within - Apply within time range
- [x] compress - Compress to range
- [x] compress_to - Compress and repeat
- [x] legato - Stretch note durations
- [x] stretch - Fill gaps
- [x] staccato - Shorten notes
- [x] swing - Add swing feel
- [x] shuffle - Randomize timing
- [x] humanize - Add human feel

### Effects
- [x] echo - Add echoes/delays
- [x] striate - Slice and spread
- [x] chop - Chop into n parts
- [x] spin - Rotate versions

### Pattern Combination
- [x] weave - Interleave patterns
- [x] binary - Binary pattern control
- [x] mask - Apply boolean mask
- [x] mask_inv - Inverse mask
- [x] struct_pattern - Euclidean structure

### Control Flow
- [x] reset - Reset every n cycles
- [x] restart - Restart pattern
- [x] fit - Fit to n cycles
- [x] chunk_with - Apply indexed function
- [x] gap - Insert silence gaps
- [x] trim - Trim to length
- [x] splice - Insert at position
- [x] scramble - Randomize order
- [x] segment - Divide into segments
- [x] loopback - Forward then backward
- [x] mirror - Palindrome within cycle

### Numeric Patterns
- [x] range - Scale to range
- [x] quantize - Quantize values
- [x] smooth - Smooth transitions
- [x] exp - Exponential scaling
- [x] log - Logarithmic scaling
- [x] sine - Sine wave shape
- [x] cosine - Cosine wave
- [x] saw - Sawtooth wave
- [x] tri - Triangle wave
- [x] square - Square wave
- [x] walk - Random walk

### Utility
- [x] trace - Debug print
- [x] count - Count events
- [x] filter - Filter by predicate
- [x] map - Transform values
- [x] flat_map - Transform and flatten

### String Operations
- [x] append_str - Append string
- [x] prepend_str - Prepend string
- [x] replace_str - Replace substring

### Random Pattern Selection
- [x] rand_cat - Random choice each cycle
- [x] wrand_cat - Weighted random choice

### Control/Effect Stubs
- [x] gain - Amplitude control (stub)
- [x] pan - Stereo position (stub)
- [x] speed - Playback rate (stub)
- [x] accelerate - Speed up over time (stub)
- [x] cutoff - Filter cutoff (stub)
- [x] resonance - Filter resonance (stub)
- [x] delay - Delay send (stub)
- [x] room - Reverb send (stub)
- [x] distort - Distortion amount (stub)

## ✅ Implemented Core Pattern Types
- [x] Pattern::pure - Single value pattern
- [x] Pattern::silence - Empty pattern
- [x] Pattern::from_string - Parse from string
- [x] Pattern::euclid - Euclidean rhythms

## ✅ Mini-Notation Parser (mini_notation.rs)
- [x] Basic sequences: "bd sn hh cp"
- [x] Groups: "[bd sn]"
- [x] Alternation: "<bd sn cp>"
- [x] Polyrhythm: "(bd, sn cp, hh)"
- [x] Rests: "~"
- [x] Repeat: "*"
- [x] Slow: "/"
- [x] Shift: "@"
- [x] Degrade: "?"
- [x] Emphasis: "!"
- [x] Sample index: ":"

## 🔍 Missing from Strudel (To Implement)

### Tonal/Musical Operators
- [ ] note - Convert to MIDI note number
- [ ] scale - Apply musical scale
- [ ] transpose - Transpose notes
- [ ] inv - Invert intervals
- [ ] chord - Generate chords
- [ ] arp - Arpeggiate
- [ ] voicing - Chord voicing
- [ ] rootNote - Set root note
- [ ] scaleTranspose - Transpose within scale

### Pattern Query/Analysis
- [ ] firstCycle - Get first cycle
- [ ] queryArc - Query time arc
- [ ] splitQueries - Split into queries
- [ ] withHaps - Process haps
- [ ] withHap - Process single hap
- [ ] withValue - Process values
- [ ] withContext - Add context

### Signal/Continuous Patterns
- [ ] signal - Continuous signal pattern
- [ ] perlin - Perlin noise
- [ ] rand - Random values
- [ ] irand - Integer random
- [ ] choose - Choose from list
- [ ] wchoose - Weighted choose
- [ ] randcat - Random concatenation

### Advanced Structure
- [ ] bite - Take bites from pattern
- [ ] chew - Chew pattern
- [ ] ply - Multiply pattern
- [ ] linger - Extend pattern
- [ ] inside - Apply function inside
- [ ] outside - Apply function outside
- [ ] iter - Iterate pattern
- [ ] iter' - Iterate backwards
- [ ] chunk' - Chunk with gap
- [ ] fast' - Fast with gap
- [ ] compress' - Compress with gap

### Pattern Effects
- [ ] degradeBy - Degrade by pattern
- [ ] sometimesBy - Sometimes by pattern
- [ ] almostNever - 10% chance
- [ ] almostAlways - 90% chance
- [ ] never - 0% chance
- [ ] someCycles - Some cycles
- [ ] someCyclesBy - Some cycles by amount

### MIDI/Control
- [ ] midi - MIDI pattern
- [ ] cc - Control change
- [ ] ccn - CC number
- [ ] ccv - CC value
- [ ] nrpn - NRPN control
- [ ] midichan - MIDI channel
- [ ] progNum - Program change

### Sequencing
- [ ] ur - Unit generator
- [ ] inhabit - Inhabit pattern
- [ ] spaceOut - Space out events
- [ ] discretise - Discretize time
- [ ] timecat - Time concatenation
- [ ] timeCat - Time concatenation
- [ ] steps - Step sequencer
- [ ] wait - Wait cycles

### Analysis/Information
- [ ] show - Show pattern info
- [ ] drawLine - ASCII visualization
- [ ] drawLineSz - Sized visualization

### OSC/External
- [ ] osc - OSC pattern
- [ ] oscprefix - OSC prefix

## Implementation Plan

1. **Phase 1: Tonal Operators** ⬅️ NEXT
   - Implement note, scale, transpose, chord
   - Add music theory utilities
   
2. **Phase 2: Signal Patterns**
   - Implement continuous patterns
   - Add noise generators
   
3. **Phase 3: Advanced Structure**
   - Implement bite, ply, linger
   - Add inside/outside operators
   
4. **Phase 4: MIDI/Control**
   - Add MIDI pattern support
   - Implement CC messages
   
5. **Phase 5: Integration**
   - Connect to synthesis engine
   - Test with live coding

## Statistics
- **Implemented**: ~95 operators
- **Missing**: ~55 operators  
- **Total Target**: ~150 operators
- **Completion**: 63%

## Notes
- Some operators are stubs that need actual audio engine integration
- Focus on pattern logic first, audio integration second
- Prioritize operators commonly used in live coding