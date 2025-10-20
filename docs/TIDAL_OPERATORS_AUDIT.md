# Tidal Operators Audit - What Phonon Supports

## ✅ IMPLEMENTED IN CORE (src/pattern.rs)

### Time Operations
- ✅ `fast(factor)` - Speed up pattern
- ✅ `slow(factor)` - Slow down pattern
- ✅ `every(n, f)` - Apply function every n cycles
- ✅ `rev()` - Reverse pattern
- ✅ `rotate_left(n)` - Rotate pattern left
- ✅ `rotate_right(n)` - Rotate pattern right

### Pattern Combinators
- ✅ `stack(patterns)` - Play patterns simultaneously (like Tidal's `stack`)
- ✅ `cat(patterns)` - Concatenate patterns in sequence (fastcat)
- ✅ `slowcat(patterns)` - Alternate between patterns each cycle

### Euclidean Rhythms
- ✅ `euclid(pulses, steps, rotation)` - Euclidean rhythm generation

## ✅ IMPLEMENTED IN pattern_ops.rs

### Time Transformations
- ✅ `early(amount)` - Shift pattern earlier
- ✅ `late(amount)` - Shift pattern later
- ✅ `offset(amount)` - Offset pattern timing

### Probabilistic
- ✅ `degrade()` - Randomly remove events (50%)
- ✅ `degrade_by(probability)` - Remove events by probability
- ✅ `sometimes(f)` - Apply function sometimes (50%)
- ✅ `sometimes_by(prob, f)` - Apply function with custom probability
- ✅ `rarely(f)` - Apply function rarely (25%)
- ✅ `often(f)` - Apply function often (75%)
- ✅ `always(f)` - Apply function always (100%)

### Structure
- ✅ `overlay(other)` - Overlay two patterns
- ✅ `append(other)` - Append pattern to end
- ✅ `dup(n)` - Duplicate pattern n times
- ✅ `stutter(n)` - Repeat each event n times
- ✅ `palindrome()` - Play pattern forward then backward
- ✅ `chunk(n, f)` - Apply function to chunks
- ✅ `when_mod(n, m, f)` - Apply when cycle % n == m
- ✅ `swap(n, other)` - Swap every n cycles

### Stereo
- ✅ `jux(f)` - Apply function to one channel
- ✅ `jux_rev()` - Reverse one channel

### Math (for numeric patterns)
- ✅ `add(amount)`
- ✅ `mul(amount)`
- ✅ `sub(amount)`
- ✅ `div(amount)`

## ✅ IMPLEMENTED IN pattern_ops_extended.rs

### Crazy Advanced Stuff!
- ✅ `chop(n)` - Chop samples into pieces
- ✅ `striate(n)` - Granular synthesis effect
- ✅ `shuffle(n)` - Shuffle segments
- ✅ `scramble(n)` - Randomize order
- ✅ `weave(n, f)` - Weave patterns together
- ✅ `spin(n)` - Rotate pan position
- ✅ `swing(amount)` - Add swing feel
- ✅ `humanize(time_var, velocity_var)` - Humanize timing
- ✅ `compress(begin, end)` - Compress pattern to time window
- ✅ `zoom(begin, end)` - Zoom into pattern segment
- ✅ `mask(pattern)` - Mask events
- ✅ `struct_pattern(bool_pattern)` - Use boolean pattern as structure
- ✅ `chunk_with(n, f)` - Chunk with custom function
- ✅ `splice(n, pattern)` - Splice pattern into chunks

### Waveforms (for control patterns)
- ✅ `sine()` - Sine wave
- ✅ `saw()` - Saw wave
- ✅ `square()` - Square wave
- ✅ `tri()` - Triangle wave
- ✅ `cosine()` - Cosine wave
- ✅ `exp()` - Exponential
- ✅ `log()` - Logarithmic

### More Pattern Operations
- ✅ `segment(n)` - Segment into n steps
- ✅ `smooth()` - Smooth transitions
- ✅ `quantize(n)` - Quantize to steps
- ✅ `staccato(amount)` - Shorten events
- ✅ `legato(amount)` - Lengthen events
- ✅ `gap(amount)` - Add gaps
- ✅ `range(min, max)` - Scale to range
- ✅ `fit(min, max)` - Fit to range
- ✅ `focus(begin, end)` - Focus on segment

### Advanced Combinators
- ✅ `rand_cat(patterns)` - Random concatenation
- ✅ `wrand_cat(patterns, weights)` - Weighted random cat

## ✅ EXPOSED TO DSL (Newly Added!)

### Pattern Combinators - **NOW WORKING!**
- ✅ **stack** ← **CRITICAL** for per-voice operations! **EXPOSED 2025-10-20**
  ```phonon
  ~kick: s "bd" * 0.8
  ~snare: s "~ sn" * 1.0
  ~hh: s "hh*4" * 0.4
  ~drums: stack [~kick, ~snare, ~hh]
  out: ~drums
  ```

## ❌ NOT YET EXPOSED TO DSL

Most of these operations exist in Rust but **aren't callable from .ph files yet!**

The compositional compiler needs to be updated to expose:
- `cat` / `slowcat` ← **ESSENTIAL** combinators
- `chop`, `striate`, `shuffle`, `scramble`
- `ply` (need to implement)
- `ur` (need to implement)
- All the waveform functions for control patterns
- `jux`, `stutter`, `palindrome`, `degrade` (implemented but not exposed!)

## ❌ MISSING FROM TIDAL (Need Implementation)

### High Priority
- ❌ `ply(n)` - Repeat each event n times per cycle
- ❌ `ur(n, pattern_map)` - Tidal's "ur" combinator
- ❌ `fit(length, pattern_list)` - Fit patterns to length
- ❌ `stripe(n)` - Repeat pattern n times per cycle
- ❌ `wedge(t, a, b)` - Wedge two patterns together

### Effects (need to wire to audio engine)
- ❌ `gain`, `pan`, `speed`, `cut` (as pattern modulation)
- ❌ `begin`, `end` - Sample start/end points
- ❌ `loop`, `unit` - Sample looping

## 🎯 NEXT STEPS

### Phase 1: Expose Existing Operations to DSL
1. **stack** - MOST IMPORTANT for per-voice operations!
   ```phonon
   # Instead of s("bd:0,gain:0.8 bd:1,gain:1.0")
   # Use: stack [s("bd:0") * 0.8, s("bd:1") * 1.0]
   ```

2. **cat/slowcat** - Essential pattern sequencing
3. **degrade/stutter/palindrome** - Already implemented!
4. **chop/striate** - Sample manipulation

### Phase 2: Implement Missing Tidal Core
5. `ply` - Event repetition
6. `ur` - Pattern combination
7. Pattern-based DSP params (gain, pan, speed, cut)

### Phase 3: Advanced Features
8. Granular operations (splice, striate)
9. Control waveforms for modulation
10. Advanced timing (swing, humanize)

## Pattern-Based Per-Voice Control

With **stack**, you can do per-voice operations:

```phonon
# Tidal style:
~drums: stack [
  s "bd" * 0.8,           # Kick at 80%
  s "sn" * 1.0,           # Snare at 100%
  s "hh*4" * 0.4          # Hi-hats at 40%
]

# Or individual patterns:
~kick: s "bd" * 0.8
~snare: s "sn" * 1.0
~hh: s "hh*4" * 0.4
~drums: stack [~kick, ~snare, ~hh]
```

This is WAY better than trying to add kwargs syntax!
