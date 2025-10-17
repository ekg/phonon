# Pattern Language Integration Design

## Current Phonon/Glicol Syntax

Currently we have:
```
~kick: sin 60 # mul 0.5
o: s "bd sn hh cp"
```

Where `#` is used for DSP signal chains and mini-notation is in quotes.

## Proposed Integration Approaches

### Approach 1: Unified Pipeline Operator (RECOMMENDED)
Use `#` for both DSP chains AND pattern transformations:

```
// DSP chain (signals)
~kick: sin 60 # mul 0.5

// Pattern chain (patterns)  
o: s "bd sn hh cp" # fast 2 # rev # every 4 (slow 2)

// Mixed (pattern through DSP)
o: s "bd sn" # fast 2 # lpf 800 0.5
```

**Pros:**
- Consistent with existing `#` operator
- Clear data flow direction
- Familiar to users

**Cons:**
- Need context-aware parsing (is it DSP or pattern?)

### Approach 2: Separate Pattern Operator
Use a different operator for patterns:

```
// DSP uses >>
~kick: sin 60 # mul 0.5

// Patterns use |>
o: s "bd sn hh cp" $ fast 2 $ rev $ every 4 (slow 2)
```

**Pros:**
- Clear distinction between DSP and patterns
- No ambiguity

**Cons:**
- Two operators to learn

### Approach 3: Method Chaining
Keep JavaScript/Strudel style:

```
o: s("bd sn hh cp").fast(2).rev().every(4, slow(2))
```

**Pros:**
- Direct match to Strudel
- Familiar to JS developers

**Cons:**
- Inconsistent with our DSP syntax
- More complex parsing

## Implementation Strategy

### Phase 1: Core Pattern Transformations
Implement the most essential operators first:

**Time:**
- fast, slow, rev, early, late

**Combination:**
- stack, cat, overlay

**Structure:**
- every, chunk, euclid

**Probability:**
- degrade, sometimes, often, rarely

### Phase 2: Extended Transformations
Add the full set:

**Advanced Time:**
- compress, zoom, focus, inside, outside
- hurry, ply, stutter, echo, linger

**Advanced Structure:**
- struct, mask, bite, segment
- iter, palindrome, rot

**Randomness:**
- choose, rand, perlin
- someCycles, almostNever, almostAlways

### Phase 3: Value Operations
Math and logic:

- add, sub, mul, div, mod, pow
- range, rangex, toBipolar
- lt, gt, eq, ne, and, or

### Phase 4: Binding & Advanced
Monadic operations:

- bind, innerBind, outerBind
- arp, jux, pan
- swing, humanize

## Parser Design

The parser needs to:

1. Recognize pattern context vs DSP context
2. Parse transformation functions with arguments
3. Handle nested function calls
4. Support both quoted patterns and references

### Grammar Extension

```
pattern_expr ::= 
    | pattern_source pattern_chain?
    
pattern_source ::=
    | 's' quoted_mini_notation
    | pattern_reference
    | pattern_constructor
    
pattern_chain ::=
    | '>>' pattern_transform pattern_chain?
    
pattern_transform ::=
    | identifier                           // rev, brak
    | identifier number                    // fast 2
    | identifier '(' args ')'              // every(4, slow(2))
    | identifier pattern_transform         // every 4 (slow 2)
    
args ::=
    | arg (',' arg)*
    
arg ::=
    | number
    | quoted_string
    | pattern_transform
    | lambda_expr
    
lambda_expr ::=
    | '|' identifier '|' pattern_transform
```

## Examples in Proposed Syntax

### Basic Transformations
```
// Speed up
o: s "bd sn" # fast 2

// Reverse
o: s "bd sn hh cp" # rev

// Chain multiple
o: s "bd sn" # fast 2 # rev # degrade
```

### Conditional Application
```
// Every 4 cycles, slow down
o: s "bd sn" # every 4 (slow 2)

// Sometimes reverse
o: s "bd sn" # sometimes rev
```

### Complex Patterns
```
// Euclidean rhythm with effects
o: s "bd" # euclid 3 8 # fast 2 # jux rev

// Stacked patterns with different speeds
~drums: stack [
  s "bd*4",
  s "~sn~sn" # late 0.125,
  s "hh*8" # degrade
]
```

### Integration with DSP
```
// Pattern through filter
o: s "bd sn" # fast 2 # lpf 800 0.5

// Multiple outputs with transformations
~bd: s "bd*4" # every 4 (fast 2)
~sn: s "~sn~sn" # sometimes rev
o: mix [~bd, ~sn] # reverb 0.3
```

## Type System Considerations

We need to track types through transformations:

```
Pattern<String> -> fast -> Pattern<String>
Pattern<Number> -> add -> Pattern<Number>  
Pattern<T> -> jux -> Pattern<(T, T)>
Pattern<T> -> every -> Pattern<T>
```

## Performance Considerations

1. **Lazy evaluation**: Patterns should be lazy
2. **Query optimization**: Cache common queries
3. **Transformation fusion**: Combine adjacent transformations
4. **Memory management**: Reuse pattern structures

## Migration Path

1. Start with basic operators working in tests
2. Add parser support for simple chains
3. Gradually add all operators
4. Optimize performance
5. Add syntactic sugar for common patterns

## Open Questions

1. Should we allow inline pattern definitions?
   ```
   o: (s "bd" # fast 2) + (s "sn" # slow 2)
   ```

2. How to handle pattern references in transformations?
   ```
   ~a: s "bd sn"
   ~b: ~a # fast 2
   ```

3. Should transformations work on DSP chains too?
   ```
   ~synth: sin 440 # mul 0.5 # every 4 (mul 2)
   ```

4. Pattern variables and interpolation?
   ```
   let speed = 2
   o: s "bd sn" # fast speed
   ```