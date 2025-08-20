# 🎵 Phonon Live Coding Guide

## Semantics & Compatibility

### TidalCycles Compatible ✅
Our implementation follows **TidalCycles semantics** for pattern behavior:
- **Cycle-based time**: Everything repeats every cycle
- **Lazy evaluation**: Patterns only compute when queried
- **Deterministic randomness**: Same seed produces same "random" pattern
- **Functional composition**: All operators return new patterns

### Strudel Compatible (Partial) ⚠️
We support Strudel's **JavaScript syntax** but with TidalCycles semantics:
- ✅ Function names match Strudel (camelCase)
- ✅ Parameter order matches Strudel
- ⚠️ Behavior follows TidalCycles (not always identical to Strudel)
- ❌ No web audio API integration (we use our own engine)

## Pattern Syntax

### Basic Patterns
```javascript
// Pure values
pure("bd")              // Constant pattern
pure(440)               // Frequency pattern
silence()               // Empty pattern

// Sequences
cat(pure("bd"), pure("sn"))           // bd then sn in one cycle
slowcat(pure("bd"), pure("sn"))        // bd for 1 cycle, sn for 1 cycle
stack(pure("bd"), pure("hh"))          // bd and hh simultaneously
```

### Time Manipulation
```javascript
// Speed changes
fast(2, pattern)        // Twice as fast
slow(2, pattern)        // Half speed

// Time shifts
early(0.25, pattern)    // Start 1/4 cycle earlier
late(0.25, pattern)     // Start 1/4 cycle later

// Compression
compress(0.25, 0.75, pattern)  // Fit into middle half of cycle
zoom(0, 0.5, pattern)           // Show only first half
```

### Pattern Operators

#### Structural
```javascript
rev(pattern)            // Reverse within cycle
palindrome(pattern)     // Forward then backward
iter(4, pattern)        // Rotate by 1/4 each cycle
every(3, rev, pattern)  // Apply rev every 3rd cycle
```

#### Randomness (Deterministic)
```javascript
rand()                  // Random 0-1
irand(10)              // Random integer 0-9
choose("bd", "sn", "hh")       // Random choice
degrade(pattern)               // Remove ~50% of events
degradeBy(0.3, pattern)        // Remove 30% of events
sometimes(rev, pattern)        // Apply rev ~50% of time
```

#### Euclidean Rhythms
```javascript
euclid(3, 8)           // 3 pulses in 8 steps: x..x..x.
euclid(5, 8)           // 5 pulses in 8: x.xx.xx.
euclidRot(3, 8, 1)     // Rotated by 1: .x..x..x
euclidLegato(3, 8)     // Extended note durations
```

#### Signals
```javascript
sine()                 // Sine wave 0-1
saw()                  // Sawtooth 0-1
square()               // Square wave 0-1
tri()                  // Triangle 0-1
perlin()              // Smooth noise 0-1

// Bipolar versions (-1 to 1)
sine2(), saw2(), square2(), tri2()
```

#### Math & Mapping
```javascript
add(2, pattern)        // Add 2 to values
mul(0.5, pattern)      // Multiply by 0.5
range(100, 200, sine()) // Map sine to 100-200
rangex(100, 1000, pattern) // Exponential mapping
scale("minor", pattern)    // Apply musical scale
```

#### Effects & Layering
```javascript
jux(rev, pattern)      // Stereo split (L: original, R: reversed)
superimpose(fast(2), pattern)  // Layer with fast version
off(0.125, id, pattern)        // Delayed echo
echo(3, 0.25, 0.8, pattern)    // 3 echoes
```

## Advanced Operators (150+ Total)

### Pattern Generation
```javascript
binary(13)             // Pattern from binary: 1101
ascii("hello")         // Pattern from ASCII codes
run(4)                 // Sequence: 0, 1, 2, 3
steps(16, pattern)     // Set to 16 steps per cycle
```

### Conditional & Control
```javascript
when(v => v > 100, mul(2), pattern)  // Double values > 100
whenmod(4, 2, rev, pattern)           // Rev on cycles 2,6,10...
ifp(c => c % 2 === 0, pat1, pat2)    // Even cycles: pat1, odd: pat2
```

### Time Ranges
```javascript
within(0.25, 0.75, rev, pattern)      // Rev middle half only
trunc(4, pattern)                     // Stop after 4 cycles
linger(2, pattern)                    // Extend events 2x
```

### Audio Metadata
```javascript
// These add metadata for the audio engine
gain(0.8, pattern)     // Volume
pan(0.5, pattern)      // Stereo position
speed(1.5, pattern)    // Playback speed
cutoff(1000, pattern)  // Filter cutoff
delay(0.3, pattern)    // Delay send
room(0.5, pattern)     // Reverb amount
```

### Arpeggiation
```javascript
arp("up", pure(["c", "e", "g"]))      // Ascending
arp("down", pure(["c", "e", "g"]))    // Descending
arp("updown", pure(["c", "e", "g"]))  // Up then down
```

### Weaving & Chunking
```javascript
weave(3, [pat1, pat2])        // Interleave patterns
chunk(4, rev, pattern)        // Rev in 4-cycle chunks
splice(0.5, pat1, pat2)       // Switch at midpoint
```

## Live Coding Workflow

### 1. Basic Beat
```javascript
stack(
  pure("bd").fast(4),           // Four-on-floor kick
  pure("hh").fast(8),           // Hi-hats
  pure("sn").off(0.5, id)       // Snare on backbeat
)
```

### 2. Add Variation
```javascript
stack(
  pure("bd").fast(4),
  pure("hh").fast(8).degradeBy(0.1),  // Random gaps
  pure("sn").off(0.5, id),
  pure("cp").sometimes(fast(2))        // Occasional double clap
)
```

### 3. Euclidean Groove
```javascript
stack(
  euclid(3, 8).fmap(_ => "bd"),       // Tresillo kick
  euclid(5, 8).fmap(_ => "hh"),       // Cinquillo hats
  euclid(7, 16).fmap(_ => "sn")       // Complex snare
)
```

### 4. Melodic Elements
```javascript
const melody = choose(60, 62, 63, 67, 70)  // C D Eb G Bb
  .segment(8)                               // 8 notes per cycle
  .scale("minor")                           // Apply scale
  
const bass = pure(36)                       // C bass
  .someCycles(add(7))                       // Sometimes G
  
stack(drums, melody, bass)
```

## Pattern DSL (String Syntax)

### Mini-notation (TidalCycles style)
```javascript
// Basic sequencing
"bd sn bd sn"          // Four events
"bd*4"                 // Repeat 4 times
"bd/2"                 // Over 2 cycles
"~"                    // Rest/silence

// Grouping
"[bd sn] hh"           // Group as one step
"[bd,sn,hh]"           // Simultaneous (chord)
"<bd sn hh>"           // Alternate each cycle

// Euclidean
"bd(3,8)"              // 3 in 8 euclidean
"sn(5,8,1)"            // With rotation

// Samples with variations
"bd:0 bd:1 bd:2"       // Different samples
"bd:rand"              // Random sample

// Effects inline
"bd*4 # gain 0.8 # pan 0.2"
```

## Performance Tips

1. **Use laziness**: Patterns are lazy - infinite patterns are fine
2. **Deterministic random**: Use cycle-based seeds for reproducibility
3. **Compose functions**: Build complex patterns from simple ones
4. **Query efficiently**: Only query the time range you need
5. **Test patterns**: Use the test suite to verify timing

## Testing Your Patterns

```javascript
// Query a pattern for events
const pattern = euclid(3, 8).fmap(_ => "bd");
const events = pattern.queryArc(0, 1);  // Get first cycle

// Check timing
events.forEach(e => {
  console.log(`Event at ${e.part.begin.toFloat()}`);
});

// Run tests
node test-all.js       // Run full test suite
node test-more.js      // Test additional operators
```

## Common Patterns by Genre

### House (120 BPM)
```javascript
stack(
  pure("bd").fast(4),
  pure("oh").off(0.125, id).fast(2),
  pure("cp").every(2, off(0.0625, id))
)
```

### Techno (130 BPM)
```javascript
stack(
  pure("bd").fast(4),
  euclid(7, 16).fmap(_ => "hh"),
  pure("sn").compress(0.5, 1, pure("sn"))
)
```

### Drum & Bass (174 BPM)
```javascript
stack(
  pure("bd").iter(2).fast(2),
  pure("sn").off(0.25, id).fast(2),
  pure("hh").fast(16).degradeBy(0.3)
)
```

### Ambient
```javascript
const pad = sine()
  .range(200, 800)
  .slow(8)
  .smooth()
  
const texture = perlin()
  .range(0.1, 0.9)
  .slow(16)
  
stack(pad, texture)
```

## Differences from TidalCycles/Strudel

1. **JavaScript syntax**: We use JS function calls, not Haskell
2. **Parameter order**: Matches Strudel (pattern usually last)
3. **No mini-notation parser**: Use JS functions or DSL strings
4. **Fraction math**: All timing uses exact fractions
5. **Metadata-based effects**: Effects are metadata, not audio processing

## Error Handling

```javascript
// Patterns handle edge cases gracefully
fast(0, pattern)       // Returns silence
degrade(silence())     // Still silence
zoom(2, 1, pattern)    // Invalid range = silence

// Type coercion
fast("2", pattern)     // Converts "2" to 2
add(new Fraction(1,2), pattern)  // Accepts Fractions
```

## Complete Operator Reference

See `OPERATORS.md` for the full list of 150+ operators with examples.

## Contributing

To add new operators:
1. Implement in `pattern.js`
2. Add tests to test suite
3. Export in module.exports
4. Document in OPERATORS.md

Remember: Patterns should be **pure**, **lazy**, and **deterministic**!