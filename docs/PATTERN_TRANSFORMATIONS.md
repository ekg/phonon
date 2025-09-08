# Pattern Transformations in Phonon

Based on TidalCycles and Strudel, here's a comprehensive list of pattern transformations and their implementation status in Phonon.

## Currently Implemented

### Time Transformations
- ✅ `fast(n)` - Speed up pattern by factor n
- ✅ `slow(n)` - Slow down pattern by factor n
- ✅ `rev()` - Reverse pattern within each cycle
- ✅ `palindrome()` - Pattern forward then backward
- ✅ `iter(n)` - Iterate through subdivisions
- ✅ `iter_back(n)` - Iterate backwards through subdivisions

### Repetition & Echo
- ✅ `stutter(n)` - Repeat each event n times
- ✅ `echo(times, time, feedback)` - Echo with decay
- ✅ `ply(n)` - Repeat each event n times (like stutter)

### Combination
- ✅ `stack(patterns)` - Play patterns simultaneously  
- ✅ `cat(patterns)` - Concatenate patterns sequentially
- ✅ `overlay(other)` - Layer two patterns
- ✅ `append(other)` - Append pattern after another

### Conditional & Cyclic
- ✅ `every(n, f)` - Apply function f every n cycles
- ✅ `when(test, f)` - Apply f when test is true
- ✅ `chunk(n, f)` - Apply f to chunks of pattern
- ✅ `chunk_back(n, f)` - Apply f to chunks backwards

### Stereo & Spatial
- ✅ `jux(f)` - Apply function to right channel only
- ✅ `jux_rev()` - Reverse on right channel
- ✅ `pan(position)` - Pan position (0-1)

### Probability & Randomness
- ✅ `degrade()` - Randomly drop 50% of events
- ✅ `degrade_by(prob)` - Drop events with probability
- ✅ `sometimes(f)` - Apply f 50% of the time
- ✅ `often(f)` - Apply f 75% of the time
- ✅ `rarely(f)` - Apply f 25% of the time
- ✅ `almostNever(f)` - Apply f 10% of the time
- ✅ `almostAlways(f)` - Apply f 90% of the time
- ✅ `choose(values)` - Random choice from values
- ✅ `choose_with(weights, values)` - Weighted random choice
- ✅ `shuffle(n)` - Shuffle pattern in n subdivisions
- ✅ `scramble(n)` - Randomly reorder n events

### Structure Manipulation
- ✅ `struct(binary_pattern)` - Apply binary pattern as mask
- ✅ `mask(binary_pattern)` - Mask with another pattern
- ✅ `euclid(pulses, steps)` - Euclidean rhythm
- ✅ `euclidOff(pulses, steps, offset)` - Offset euclidean
- ✅ `euclidRot(pulses, steps, rotation)` - Rotated euclidean

### Value Transformations
- ✅ `add(n)` - Add value to numeric pattern
- ✅ `sub(n)` - Subtract value
- ✅ `mul(n)` - Multiply value
- ✅ `div(n)` - Divide value
- ✅ `mod(n)` - Modulo value
- ✅ `range(min, max)` - Scale to range
- ✅ `segment(n)` - Sample continuous pattern n times per cycle

### Pattern Effects
- ✅ `gain(amount)` - Set gain/volume
- ✅ `speed(rate)` - Playback speed
- ✅ `begin(pos)` - Start position
- ✅ `end(pos)` - End position
- ✅ `cut(group)` - Cut group
- ✅ `cutoff(freq)` - Filter cutoff
- ✅ `resonance(q)` - Filter resonance
- ✅ `delay(time)` - Delay effect
- ✅ `room(size)` - Reverb room size
- ✅ `distort(amount)` - Distortion

### Alignment & Timing
- ✅ `early(time)` - Shift pattern earlier
- ✅ `late(time)` - Shift pattern later
- ✅ `off(time, f)` - Offset and transform
- ✅ `nudge(amount)` - Small time shift
- ✅ `legato(ratio)` - Note duration ratio
- ✅ `swing(amount)` - Swing timing
- ✅ `swingBy(amount, subdivision)` - Swing with subdivision

## Not Yet Implemented (from Tidal/Strudel)

### Advanced Time
- ❌ `compress(start, end)` - Compress pattern into time range
- ❌ `zoom(start, end)` - Zoom into portion of pattern
- ❌ `rotL(amount)` - Rotate left in time
- ❌ `rotR(amount)` - Rotate right in time
- ❌ `spin(n)` - Spin pattern n times

### Advanced Structure
- ❌ `weave(patterns)` - Weave patterns together
- ❌ `weaveWith(f, patterns)` - Weave with function
- ❌ `layer(functions)` - Layer multiple transformations
- ❌ `superimpose(f)` - Superimpose transformed copy

### Advanced Combination
- ❌ `fastcat(patterns)` - Fast concatenation
- ❌ `slowcat(patterns)` - Slow concatenation  
- ❌ `randcat(patterns)` - Random concatenation
- ❌ `timeCat(pairs)` - Time-weighted concatenation

### Advanced Effects
- ❌ `chop(n)` - Chop sample into n pieces
- ❌ `striate(n)` - Striate across pattern
- ❌ `slice(n, i)` - Select slice i of n
- ❌ `splice(n, pat)` - Splice with pattern
- ❌ `loopAt(n)` - Loop at n cycles
- ❌ `hurry(n)` - Speed up pattern and playback

### Pattern Queries
- ❌ `within(start, end, f)` - Apply f within time range
- ❌ `outside(start, end, f)` - Apply f outside time range
- ❌ `fit(n, values, pat)` - Fit values to pattern

## Usage in Phonon

Currently, pattern transformations are applied through method chaining:

```rust
let pattern = parse_mini_notation("bd sn hh cp")
    .fast(2.0)
    .rev()
    .every(4, |p| p.slow(2.0));
```

## Integration with DSP

Pattern transformations can be combined with the DSP chains:

```rust
// Define a synth
~kick: sin 60 >> mul 0.5

// Use in pattern with transformations
o: s "~kick ~kick ~kick ~kick" 
   // Would need syntax extension for:
   // .fast(2).rev().every(4, |p| p.slow(2))
```

## Next Steps

1. Implement missing high-priority transformations
2. Add DSL syntax for applying transformations in Glicol/Phonon code
3. Create comprehensive test suite
4. Add documentation and examples