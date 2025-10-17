# Pattern Modulation in Phonon

## Overview

In Phonon, pattern modulation can use either the `$` operator (TidalCycles-style) or the `$` operator (pipe-forward style). Both operators work identically - use whichever you prefer!

## Basic Syntax

### Both Styles Work!

| TidalCycles Style | Pipe-Forward Style |
|-------------------|-------------------|
| `"bd sn" $ fast 2` | `"bd sn" \|> fast 2` |
| `"bd sn" $ fast 2 $ rev` | `"bd sn" \|> fast 2 \|> rev` |
| `"100 200" $ slow 2` | `"100 200" \|> slow 2` |
| `"bd" $ every 4 rev` | `"bd" \|> every 4 rev` |

You can use either operator - they're completely equivalent:

```phonon
# TidalCycles style with $
"bd sn" $ fast 2 $ rev

# Pipe-forward style with |>
"bd sn" $ fast 2 $ rev

# Both produce the exact same result!
```

## How They Work

Both operators bind tightly to patterns and allow chaining:

```phonon
"100 200 300 400" $ fast 2        # Speed up 2x
"100 200 300 400" $ slow 2        # Slow down 2x
"100 200 300 400" $ rev           # Reverse
"100 200 300 400" $ fast 2 $ rev # Chain operations
```

## Available Pattern Operations

### Time Operations
- `fast N` - Speed up by factor N
- `slow N` - Slow down by factor N
- `early N` - Shift earlier by N cycles
- `late N` - Shift later by N cycles
- `offset N` - Offset by N cycles
- `rotate N` - Rotate by N cycles

### Structural Operations
- `rev` - Reverse pattern
- `palindrome` - Forward then backward
- `iter N` - Iterate shifted copies
- `chunk N F` - Apply F to chunks
- `chop N` - Chop into N pieces

### Probability Operations
- `degrade` - Random 50% dropout
- `degradeBy N` - Random N% dropout
- `sometimes F` - Apply F 50% of cycles
- `often F` - Apply F 75% of cycles
- `rarely F` - Apply F 25% of cycles
- `every N F` - Apply F every N cycles

### Combination Operations
- `overlay P` - Overlay with pattern P
- `append P` - Append pattern P
- `fastcat [P]` - Concatenate patterns
- `slowcat [P]` - Alternate patterns
- `stack [P]` - Stack patterns (or use `|` operator)

## Complex Examples

### Drums with Operations
```phonon
~kick: "bd . . bd . . bd ." $ every 8 (slow 2)
~snare: ". . sn . . . sn ." $ rotate 0.125
~hats: "hh*16" $ degradeBy 0.3 $ pan 0.7
~perc: "cp? rim?" $ sometimes rev

out: ~kick + ~snare + ~hats + ~perc
```

### Pattern Operations in DSP Chains
```phonon
# Pattern operations bind before DSP chains
~drums: "bd*4" $ fast 2 # lpf 1000 0.8

# Pattern operations as parameters
o: sin ("220 440" $ slow 2) # mul 0.5
```

### Nested Operations with Parentheses
```phonon
# Use parentheses for clarity in nested operations
"bd sn" $ every 4 (fast 2)
"bd sn" $ sometimes (fast 2)
"bd sn" $ chunk 4 (rev)
```

## Order of Operations

The precedence order in Phonon is:
1. Pattern operations (`$`)
2. DSP chains (`#`)
3. Arithmetic (`+`, `*`, etc.)

Examples:
```phonon
"bd sn" $ fast 2 # lpf 1000     # ((pattern $ fast 2) # lpf 1000)
"100 200" $ slow 2 * 0.5         # ((pattern $ slow 2) * 0.5)
```

## In Rust Code

When writing Rust code directly, you can use method chaining:

```rust
use phonon::mini_notation_v3::parse_mini_notation;

let pattern = parse_mini_notation("100 200 300")
    .fast(2)      // Speed up 2x
    .rev()        // Reverse
    .degrade();   // Random dropout
```

## Implementation Notes

The `$` operator is implemented in the nom parser (`src/nom_parser.rs`) as a pattern operation that creates an AST node of type `PatternOp`. Each operation is applied to the pattern in sequence during evaluation.

Pattern operations are defined as methods on `Pattern<T>` in `src/pattern.rs` and `src/pattern_ops.rs`, making them composable and chainable.