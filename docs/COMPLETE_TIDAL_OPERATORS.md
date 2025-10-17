# Complete Tidal/Strudel Operators Catalog

Based on analysis of strudel/packages/core/pattern.mjs - 169 total exports

## Core Pattern Constructors
- `pure(value)` - Create a pattern from a single value
- `silence` / `gap(steps)` - Create silence
- `nothing` - Empty pattern
- `reify(thing)` - Convert to pattern

## Combination & Layering
- `stack(...pats)` / `polyrhythm` / `pr` - Play patterns simultaneously
- `stackLeft/Right/Centre/By` - Stack with alignment
- `cat(...pats)` - Concatenate sequentially
- `slowcat(...pats)` - Slow concatenation (one per cycle)
- `slowcatPrime(...pats)` - Slow cat variant
- `fastcat(...pats)` - Fast concatenation (all in one cycle)
- `sequence(...pats)` / `seq` - Create sequence
- `arrange(...sections)` - Arrange sections
- `seqPLoop(...parts)` - Sequence loop
- `polymeter` / `pm` - Polymetric patterns
- `superimpose(funcs)` - Superimpose transformed copies

## Time Manipulation
- `fast(factor)` / `density` - Speed up
- `slow(factor)` / `sparsity` - Slow down
- `hurry(factor)` - Speed up pattern and samples
- `early(time)` - Shift earlier
- `late(time)` - Shift later
- `compress(begin, end)` - Compress into time range
- `compressSpan(span)` - Compress with span
- `focus(begin, end)` - Focus on time range
- `focusSpan(span)` - Focus with span
- `zoom(start, end)` - Zoom into portion
- `zoomArc(arc)` - Zoom with arc
- `fastGap(factor)` - Speed up with gaps
- `inside(factor, func)` - Apply function inside subdivision
- `outside(factor, func)` - Apply function outside subdivision

## Pattern Transformation
- `rev` - Reverse pattern
- `invert` / `inv` - Invert values
- `palindrome` - Forward then backward
- `iter(n)` - Iterate through subdivisions
- `iterBack(n)` - Iterate backwards
- `rot(n)` - Rotate pattern
- `rotL(amount)` - Rotate left
- `rotR(amount)` - Rotate right

## Repetition & Echo
- `ply(factor)` - Repeat each event
- `stutter(n)` - Stutter events
- `echo(times, time, feedback)` - Echo with decay
- `echoWith(times, time, func)` / `stutWith` - Echo with function
- `linger(factor)` - Linger on subdivisions

## Conditional & Cyclic
- `every(n, func)` / `firstOf` - Apply every n cycles
- `lastOf(n, func)` - Apply on last of n cycles
- `when(test, func)` - Conditional application
- `off(time, func)` - Offset and apply
- `apply(func)` - Apply pattern of functions

## Randomness & Probability
- `degrade` - Random dropout (50%)
- `degradeBy(amount)` - Controlled dropout
- `undegrade` - Inverse degrade
- `sometimes(func)` - 50% chance
- `often(func)` - 75% chance  
- `rarely(func)` - 25% chance
- `almostNever(func)` - 10% chance
- `almostAlways(func)` - 90% chance
- `never` - Never apply (0%)
- `always` - Always apply (100%)
- `someCycles(func)` - Some cycles
- `someCyclesBy(prob, func)` - Controlled cycles
- `rand` - Random values 0-1
- `irand(max)` - Random integers
- `chooseWith(weights, values)` - Weighted choice
- `choose(values)` - Random choice
- `chooseInWith/Out` - Choose variants
- `perlin` - Perlin noise
- `perlinWith(seed)` - Seeded perlin

## Structure & Rhythm
- `struct(binary)` - Apply structure
- `mask(pattern)` - Mask with pattern
- `euclid(pulses, steps, rotation)` - Euclidean rhythms
- `euclidRot/euclidRotate` - Rotated euclidean
- `euclidOff/euclidOffset` - Offset euclidean
- `bite(n, patterns)` - Bite-sized pieces
- `chunk(n, func)` - Apply to chunks
- `chunkBack(n, func)` - Chunk backwards
- `segment(rate)` / `seg` - Sample at rate

## Value Operations
- `add(value)` - Addition
- `sub(value)` - Subtraction
- `mul(value)` - Multiplication
- `div(value)` - Division
- `mod(value)` - Modulo
- `pow(value)` - Power
- `round` - Round to nearest
- `floor` - Round down
- `ceil` - Round up
- `range(min, max)` - Scale to range
- `rangex(min, max)` - Exponential range
- `range2(min, max)` - Bipolar range
- `ratio` - Convert to ratio
- `toBipolar` - Convert to -1 to 1
- `fromBipolar` - Convert from -1 to 1

## Bitwise Operations
- `band(value)` - Bitwise AND
- `bor(value)` - Bitwise OR
- `bxor(value)` - Bitwise XOR
- `blshift(value)` - Left shift
- `brshift(value)` - Right shift

## Comparison Operations
- `lt(value)` - Less than
- `gt(value)` - Greater than
- `lte(value)` - Less than or equal
- `gte(value)` - Greater than or equal
- `eq(value)` - Equal
- `eqt(value)` - Equal (type-safe)
- `ne(value)` - Not equal
- `net(value)` - Not equal (type-safe)

## Logical Operations
- `and(value)` - Logical AND
- `or(value)` - Logical OR

## Pattern Binding
- `bind(func)` - Monadic bind
- `innerBind(func)` - Inner join bind
- `outerBind(func)` - Outer join bind
- `squeezeBind(func)` - Squeeze bind
- `stepBind(func)` - Step bind
- `polyBind(func)` - Polymorphic bind

## Pattern Queries
- `withValue(func)` - Transform values
- `set(value)` - Set all values
- `keep(pattern)` - Keep matching
- `keepif(predicate)` - Keep if true

## Arpeggiators
- `arp(mode)` - Arpeggiate
- `arpWith(func)` - Custom arpeggiator
- Modes: up, down, updown, downup, converge, diverge, random

## Stereo & Spatial
- `jux(func)` - Juxtapose (stereo)
- `juxBy(amount, func)` - Partial jux
- `pan(position)` - Stereo panning

## Swing & Humanization
- `swing(amount)` - Add swing
- `swingBy(amount, subdivision)` - Swing with subdivision
- `humanize` - Add human timing

## Special Effects
- `brak` - Breakbeat pattern
- `roll` - Drum roll
- `chop(n)` - Chop samples
- `striate(n)` - Striate pattern
- `splice(n, pattern)` - Splice with pattern
- `loopAt(n)` - Loop at length
- `fit(n)` - Fit to length
- `chunk/slowchunk/fastchunk` - Chunk variants

## Tempo
- `cpm(cycles_per_minute)` - Set tempo

## Registration & Utilities
- `register(name, func, ...)` - Register new function
- `isPattern(thing)` - Check if pattern
- `setStringParser(parser)` - Set parser
- `calculateSteps(x)` - Calculate steps

## Our Proposed Phonon DSL Syntax

Instead of JavaScript method chaining, we need a syntax that fits our DSL. Here are some options:

### Option 1: Pipe operator (like Elixir/F#)
```
o: s "bd sn hh cp" $ fast 2 $ rev $ every 4 (slow 2)
```

### Option 2: Chain operator (custom)
```
o: s "bd sn hh cp" -> fast(2) -> rev() -> every(4, slow(2))
```

### Option 3: Postfix with dots (like Strudel but in our syntax)
```
o: s "bd sn hh cp".fast(2).rev().every(4, slow(2))
```

### Option 4: Function wrapping
```
o: every(4, slow(2), rev(fast(2, s("bd sn hh cp"))))
```

### Option 5: Modifier blocks
```
o: s "bd sn hh cp" {
  fast 2
  rev
  every 4 (slow 2)
}
```