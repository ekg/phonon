# Phonon Language Reference

## Overview

Phonon is a live coding language that combines TidalCycles-style pattern manipulation with modular synthesis DSP chains. It features two distinct but complementary domains:

1. **Pattern Domain**: Event sequences that evolve over time (cycles)
2. **DSP Domain**: Audio signal processing chains

These domains use different operators to maintain clarity:
- `$` for pattern transformations (events/cycle)
- `#` for DSP signal flow (samples/second)

## Language Grammar

### EBNF Grammar

```ebnf
(* Top-level structure *)
program         = { line } ;
line            = bus_definition | output_definition | empty_line ;
bus_definition  = "~" identifier ":" expression ;
output_definition = ("o" | "out") ":" expression ;

(* Expressions *)
expression      = pattern_expr ;
pattern_expr    = chain_expr { "|>" pattern_transform } ;
chain_expr      = additive_expr { ">>" additive_expr } ;
additive_expr   = multiplicative_expr { ("+" | "-") multiplicative_expr } ;
multiplicative_expr = primary_expr { ("*" | "/") primary_expr } ;
primary_expr    = bus_ref | pattern | node | number | "(" expression ")" ;

(* Basic elements *)
bus_ref         = "~" identifier ;
pattern         = string_literal | "s" string_literal ;
node            = identifier { argument } ;
argument        = number | bus_ref | pattern | "(" expression ")" ;
number          = float | integer ;
identifier      = letter { letter | digit | "_" } ;
string_literal  = '"' { any_char_except_quote } '"' ;

(* Pattern transformations *)
pattern_transform = simple_transform | parameterized_transform | nested_transform ;
simple_transform  = "rev" | "palindrome" ;
parameterized_transform = 
    ("fast" | "slow" | "rotate" | "degrade" | "degradeBy" | 
     "gain" | "pan" | "speed" | "crush" | "legato" | "shape" | 
     "squiz" | "accelerate") number |
    ("chop" | "striate" | "shuffle" | "scramble" | "coarse" | "cut") integer |
    "scale" string_literal ;
nested_transform  = 
    "every" integer pattern_transform |
    ("sometimes" | "rarely" | "often" | "jux") pattern_transform |
    "chunk" integer pattern_transform ;
```

## Operator Precedence (highest to lowest)

1. Function application (nodes and transforms)
2. `*` `/` (multiplication, division)
3. `+` `-` (addition, subtraction)
4. `#` (DSP chaining)
5. `$` (pattern operations)
6. `:` (assignment)

## Core Concepts

### 1. Buses (Signal References)

Buses are named signal paths prefixed with `~`. They can contain patterns, DSP chains, or combinations:

```phonon
~lfo: sin 0.5               # Low-frequency oscillator
~drums: "bd sn hh cp"       # Pattern
~bass: saw 55 # lpf 1000   # DSP chain
```

### 2. Patterns

Patterns define sequences of events that repeat every cycle:

```phonon
# Mini-notation patterns
"bd sn"                     # Basic pattern
"bd*4 sn . hh*8"           # Repetition and rests
"[bd sn] hh"               # Grouping
"bd <sn cp> hh"            # Alternation
"0 3 7 10"                 # Numeric patterns
```

### 3. Pattern Transformations

Pattern operations transform event sequences using the `$` operator:

```phonon
# Time transformations
"bd sn" $ fast 2          # Double speed
"bd sn" $ slow 2          # Half speed
"bd sn" $ rotate 0.25     # Shift by 1/4 cycle

# Structural transformations
"bd sn hh cp" $ rev       # Reverse
"bd sn" $ palindrome      # Forward then backward
"bd sn" $ chop 4          # Slice into 4 parts
"bd sn" $ shuffle 3       # Shuffle with seed 3

# Conditional transformations
"bd sn" $ every 3 rev     # Reverse every 3rd cycle
"bd sn" $ sometimes rev   # Randomly reverse (~50%)
"bd sn" $ rarely rev      # Rarely reverse (~10%)
"bd sn" $ often rev       # Often reverse (~90%)

# Degradation
"bd*16" $ degrade         # Random dropout (50%)
"bd*16" $ degradeBy 0.3   # 30% chance of dropout

# Chaining transformations
"bd sn" $ fast 2 $ every 4 rev $ rotate 0.125
```

### 4. DSP Nodes

DSP nodes process audio signals and chain with `#`:

#### Oscillators
```phonon
sin 440                    # Sine wave
saw 220                    # Sawtooth wave
square 110                 # Square wave
tri 880                    # Triangle wave
noise                      # White noise
pink                       # Pink noise
brown                      # Brown noise
impulse 2                  # Impulse train
```

#### Filters
```phonon
lpf 1000 0.8              # Low-pass (cutoff, Q)
hpf 500 0.7               # High-pass
bpf 1500 2                # Band-pass
notch 1000 10             # Notch filter
```

#### Effects
```phonon
delay 0.25 0.5 0.3        # Delay (time, feedback, mix)
reverb 0.8 0.5 0.3        # Reverb (room, damping, mix)
chorus 1.5 0.8 0.5        # Chorus (rate, depth, mix)
phaser 0.5 0.9 0.3        # Phaser
distortion 2.0            # Distortion (gain)
clip -0.8 0.8            # Hard clipper
```

#### Math Operations
```phonon
mul 0.5                   # Multiply signal
add 0.25                  # Add to signal
sub 0.1                   # Subtract from signal
div 2                     # Divide signal
```

#### Envelopes & Modulation
```phonon
env 0.01 0.1 0.7 0.5      # ADSR envelope
lfo 0.5                   # LFO (as modulation source)
```

### 5. Signal Math

Arithmetic operations on buses and values:

```phonon
~mod: ~lfo * 2000 + 500    # Scale and offset LFO
~mix: ~bass * 0.4 + ~lead * 0.3  # Mix signals
~cutoff: 1000 + ~env * 3000      # Envelope modulation
```

### 6. Output

The output must be defined with `o:` or `out:`:

```phonon
o: ~bass # mul 0.5        # Output with gain
out: ~mix # reverb 0.3    # Output with reverb
```

## Complete Examples

### Example 1: Basic Drum Pattern
```phonon
~drums: "bd sn [bd bd] sn" $ fast 2
o: ~drums # gain 0.8
```

### Example 2: Modulated Bass
```phonon
~lfo: sin 0.5 # mul 0.5 # add 0.5
~bass: saw 55 # lpf ~lfo * 2000 + 500 0.8
o: ~bass # mul 0.4
```

### Example 3: Complex Rhythm
```phonon
~kick: "bd*4" $ every 4 (slow 2)
~hats: "hh*16" $ degradeBy 0.3 $ pan 0.7
~snare: ". sn . sn" $ rotate 0.125
~drums: ~kick + ~hats * 0.5 + ~snare * 0.8
o: ~drums # lpf 8000 0.5 # reverb 0.2 0.7 0.15
```

### Example 4: Melodic Pattern with Scale
```phonon
~melody: "0 3 7 10 7 3" $ slow 2 $ scale "minor"
~voice: ~melody # saw # lpf 2000 0.6
~delay: ~voice # delay 0.375 0.4 0.3
o: ~voice * 0.7 + ~delay * 0.3
```

### Example 5: Live Coding Session
```phonon
# Define rhythm section
~kick: "bd . . bd . . bd ." $ fast 2
~snare: ". . sn . . . sn ." $ fast 2 $ every 8 rev
~hats: "hh*16" $ degradeBy 0.2 $ pan "0.3 0.7" $ fast 2

# Bass line
~bassline: "0 0 12 7" $ slow 4
~bass: ~bassline # saw # lpf 800 0.9 # mul 0.3

# Lead synth
~lead_pattern: "0 3 7 12 10 7 3 0" $ slow 2 $ every 4 (rotate 0.25)
~lead: ~lead_pattern # square # hpf 400 0.5 # lpf 3000 0.7

# Modulation
~lfo: sin 0.25 # mul 0.3 # add 0.7
~filtered_lead: ~lead # lpf ~lfo * 2000 + 1000 0.6

# Mix everything
~rhythm: ~kick + ~snare * 0.8 + ~hats * 0.4
~mix: ~rhythm * 0.6 + ~bass * 0.4 + ~filtered_lead * 0.3

# Output with master effects
o: ~mix # reverb 0.3 0.6 0.2 # mul 0.8
```

## Pattern Mini-Notation Reference

| Syntax | Description | Example |
|--------|-------------|---------|
| `a b c` | Sequence | `"bd sn hh"` |
| `a*n` | Repeat n times | `"bd*4"` |
| `.` | Rest/silence | `"bd . sn ."` |
| `[a b]` | Group (plays in one step) | `"[bd sn] hh"` |
| `<a b>` | Alternate each cycle | `"bd <sn cp>"` |
| `a/n` | Slow by factor n | `"bd/2"` |
| `a(x,y)` | Euclidean rhythm | `"bd(3,8)"` |
| `a:n` | Sample selection | `"bd:3"` |
| `a?` | Maybe (50% chance) | `"bd? sn"` |
| `{a,b,c}` | Polyrhythm | `"{bd*4, hh*3}"` |
| `a@n` | Duration | `"bd@2 sn"` |

## Pattern Transformation Reference

### Time Transformations
- `fast n` - Speed up by factor n
- `slow n` - Slow down by factor n  
- `rotate n` - Rotate by n cycles (0-1)
- `brak` - Pattern + pattern shifted by 1/4

### Structural Transformations
- `rev` - Reverse the pattern
- `palindrome` - Play forward then backward
- `iter n` - Iterate through rotations
- `chunk n f` - Apply f to n-sized chunks
- `chop n` - Slice into n parts
- `striate n` - Interleave n slices
- `shuffle n` - Deterministic shuffle with seed n
- `scramble n` - Random reorder with seed n

### Conditional Transformations
- `every n f` - Apply f every n cycles
- `sometimes f` - Apply f ~50% of the time
- `rarely f` - Apply f ~10% of the time
- `often f` - Apply f ~90% of the time
- `someCycles f` - Apply f to random cycles

### Degradation
- `degrade` - Random 50% dropout
- `degradeBy n` - n probability of dropout
- `unDegradeBy n` - Inverse of degradeBy

### Combination
- `jux f` - Stereo: original left, f(pattern) right
- `juxBy n f` - Jux with n amount (0-1)
- `stack [p1, p2, ...]` - Layer patterns
- `cat [p1, p2, ...]` - Concatenate patterns
- `overlay p1 p2` - Overlay two patterns

### DSP Parameter Patterns
- `gain n` - Set gain
- `pan n` - Set pan position (-1 to 1)
- `speed n` - Playback speed
- `crush n` - Bit crushing
- `coarse n` - Sample rate reduction
- `cut n` - Cut group
- `legato n` - Note length multiplier
- `shape n` - Waveshaping distortion
- `squiz n` - Downsampling effect
- `accelerate n` - Pitch sweep

## Performance Characteristics

The nom parser achieves excellent performance for live coding:
- **Parse time**: ~4.5 microseconds for complex expressions
- **Throughput**: ~219,000 parses per second
- **Memory**: Zero-copy parsing (no string allocations)

This ensures instant feedback during live coding performances with no perceptible latency.

## Best Practices

1. **Use meaningful bus names**: `~kick`, `~bassline`, `~lead_melody`
2. **Layer complexity gradually**: Start simple, add transformations
3. **Balance patterns and DSP**: Patterns for rhythm, DSP for timbre
4. **Modulate parameters**: Use LFOs and envelopes for movement
5. **Test incrementally**: Build up complex patches step by step
6. **Comment your patches**: Use `#` for comments in saved files

## Integration with Rust

The parser generates an AST that can be processed by the Rust backend:

```rust
use phonon::nom_parser::{parse_dsl, Expr, PatternTransform};

let code = r#"
    ~drums: "bd sn" $ fast 2
    o: ~drums # lpf 1000 0.8
"#;

let env = parse_dsl(code)?;
// Process env.ref_chains and env.output_chain
```

This enables real-time audio generation with sub-millisecond latency.