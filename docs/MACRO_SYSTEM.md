# Phonon Macro System

The Phonon macro system provides compile-time code generation for programmatic DSL creation. It enables loops, conditionals, and arithmetic to generate multiple buses, effects, and complex audio graphs.

## Overview

Macros are expanded **before** the main parser runs, transforming high-level constructs into regular Phonon DSL code. This means:
- Zero runtime overhead
- Full compile-time evaluation
- Works with all existing DSL features

## Features

### 1. For Loops

Generate multiple buses with a single loop construct.

**Syntax:**
```phonon
for VAR in START..END:
    BODY (indented)
```

**Example - Generate 10 oscillators:**
```phonon
for i in 1..10:
    ~osc[i] $ sine (110 * i)
out $ sum(~osc[1..10]) * 0.1
```

**Expands to:**
```phonon
~osc1 $ sine 110
~osc2 $ sine 220
~osc3 $ sine 330
~osc4 $ sine 440
~osc5 $ sine 550
~osc6 $ sine 660
~osc7 $ sine 770
~osc8 $ sine 880
~osc9 $ sine 990
~osc10 $ sine 1100
out $ (~osc1 + ~osc2 + ~osc3 + ~osc4 + ~osc5 + ~osc6 + ~osc7 + ~osc8 + ~osc9 + ~osc10) * 0.1
```

### 2. Indexed Buses

Reference buses with loop variable indices using bracket notation.

**Syntax:**
```phonon
~name[VAR]
```

**Example:**
```phonon
for i in 1..4:
    ~voice[i] $ saw (55 * i) # lpf (200 * i) 0.7
```

**Expands to:**
```phonon
~voice1 $ saw 55 # lpf 200 0.7
~voice2 $ saw 110 # lpf 400 0.7
~voice3 $ saw 165 # lpf 600 0.7
~voice4 $ saw 220 # lpf 800 0.7
```

### 3. Sum Function

Mix all indexed buses in a range.

**Syntax:**
```phonon
sum(~name[START..END])
```

**Example:**
```phonon
~a1 $ sine 110
~a2 $ sine 220
~a3 $ sine 330
out $ sum(~a[1..3]) * 0.3
```

**Expands to:**
```phonon
~a1 $ sine 110
~a2 $ sine 220
~a3 $ sine 330
out $ (~a1 + ~a2 + ~a3) * 0.3
```

### 4. Arithmetic Expressions

Full arithmetic with loop variables, respecting operator precedence.

**Supported operators:** `+`, `-`, `*`, `/`

**Examples:**
```phonon
for i in 1..5:
    ~h[i] $ sine (110 * i)           -- 110, 220, 330, 440, 550
    ~f[i] $ saw (220 + i * 55)       -- 275, 330, 385, 440, 495
    ~a[i] $ tri 440 * (1.0 / i)      -- amplitude: 1, 0.5, 0.33, 0.25, 0.2
```

**Nested arithmetic:**
```phonon
for i in 1..4:
    ~s[i] $ sine ((110 * i) + 55)    -- 165, 275, 385, 495
```

### 5. If/Else Conditionals

Compile-time conditional expansion.

**Syntax:**
```phonon
if CONDITION then EXPR else EXPR
```

**Comparison operators:** `==`, `!=`, `<`, `>`, `<=`, `>=`

**Examples:**
```phonon
-- Simple condition
~sound $ if 1 == 1 then saw 110 else sine 110

-- With loop variable
for i in 1..8:
    ~v[i] $ if i < 4 then sine (110 * i) else saw (110 * i)
out $ sum(~v[1..8])
```

**Expands to:**
```phonon
~v1 $ sine 110
~v2 $ sine 220
~v3 $ sine 330
~v4 $ saw 440
~v5 $ saw 550
~v6 $ saw 660
~v7 $ saw 770
~v8 $ saw 880
out $ (~v1 + ~v2 + ~v3 + ~v4 + ~v5 + ~v6 + ~v7 + ~v8)
```

### 6. Modulo Conditions

Check even/odd or periodic patterns with modulo.

**Syntax:**
```phonon
if VAR % DIVISOR == REMAINDER then ... else ...
```

**Example - Alternating waveforms:**
```phonon
for i in 1..8:
    ~osc[i] $ if i % 2 == 0 then sine (110 * i) else saw (110 * i)
out $ sum(~osc[1..8]) * 0.1
```

**Expands to:**
```phonon
~osc1 $ saw 110     -- odd
~osc2 $ sine 220    -- even
~osc3 $ saw 330     -- odd
~osc4 $ sine 440    -- even
~osc5 $ saw 550     -- odd
~osc6 $ sine 660    -- even
~osc7 $ saw 770     -- odd
~osc8 $ sine 880    -- even
out $ (~osc1 + ~osc2 + ~osc3 + ~osc4 + ~osc5 + ~osc6 + ~osc7 + ~osc8) * 0.1
```

## Complete Examples

### Harmonic Series Generator

Generate the first N harmonics with natural amplitude falloff:

```phonon
tempo: 2.0

-- Generate harmonics 1-8 with 1/n amplitude
for n in 1..8:
    ~harmonic[n] $ sine (110 * n) * (1.0 / n)

out $ sum(~harmonic[1..8]) * 0.5
```

### Multi-Voice Synthesizer

Create a polyphonic synthesizer with different timbres per voice:

```phonon
tempo: 2.0

-- Low voices: filtered saw
for i in 1..4:
    ~low[i] $ saw (55 * i) # lpf 400 0.8

-- High voices: sine with chorus
for i in 1..4:
    ~high[i] $ sine (220 * i) # chorus 2.0 0.5 0.3

-- Mix
out $ sum(~low[1..4]) * 0.3 + sum(~high[1..4]) * 0.2
```

### Conditional Effect Routing

Apply different effects based on voice index:

```phonon
tempo: 2.0

for i in 1..6:
    ~src[i] $ saw (110 * i)
    ~fx[i] $ if i <= 3 then ~src[i] # lpf 800 0.8 else ~src[i] # hpf 2000 0.5

out $ sum(~fx[1..6]) * 0.15
```

### Rhythmic Pattern Generation

Create rhythmic variations using modulo:

```phonon
tempo: 2.0

for beat in 1..16:
    -- Every 4th beat gets a kick, others get hi-hat
    ~drum[beat] $ if beat % 4 == 1 then s "bd" else s "hh"

-- Note: This generates buses, actual timing comes from pattern system
```

### Detuned Supersaw

Create a detuned supersaw with multiple slightly-detuned oscillators:

```phonon
tempo: 2.0

-- 7 detuned saws centered around 110Hz
for i in 1..7:
    ~det[i] $ saw (110 + (i - 4) * 2)  -- -6, -4, -2, 0, +2, +4, +6 Hz detune

out $ sum(~det[1..7]) * 0.1 # lpf 2000 0.7
```

## API Reference

### Rust API

```rust
use phonon::macro_expander::expand_macros;
use phonon::compositional_parser::{parse_program, parse_program_with_macros};
use phonon::compositional_compiler::compile_program;

// Option 1: Manual expansion
let expanded = expand_macros(code);
let (_, statements) = parse_program(&expanded)?;
let graph = compile_program(statements, 44100.0, None)?;

// Option 2: Integrated (recommended)
let (_, statements) = parse_program_with_macros(code)?;
let graph = compile_program(statements, 44100.0, None)?;
```

### Functions

| Function | Description |
|----------|-------------|
| `expand_macros(input: &str) -> String` | Expand all macros in input code |
| `parse_program_with_macros(input: &str)` | Parse with automatic macro expansion |
| `expand_for_loops(input: &str) -> String` | Expand only for loops |
| `expand_if_else(input: &str) -> String` | Expand only if/else conditionals |
| `expand_sum_calls(input: &str) -> String` | Expand only sum() calls |

## Limitations

1. **Compile-time only**: Macros are expanded before parsing, not at runtime
2. **Integer loop ranges**: `for i in N..M` requires integer N and M
3. **No nested loops**: Currently single-level loops only
4. **Simple conditions**: Conditions must evaluate to numeric comparisons
5. **No string interpolation**: Variable names in strings not supported

## Best Practices

1. **Keep loops small**: Large loops (>20 iterations) may cause stack issues
2. **Use meaningful names**: `~voice[i]` is clearer than `~v[i]`
3. **Add comments**: Document what the loop generates
4. **Test expansion**: Use `expand_macros()` to verify output before compiling

## Testing

The macro system has comprehensive test coverage:

- **14 unit tests** in `src/macro_expander.rs`
- **23 integration tests** in `tests/test_macro_expansion.rs`

Run tests:
```bash
cargo test --lib macro_expander
cargo test --test test_macro_expansion
```

## Implementation Details

The macro expander is implemented in `src/macro_expander.rs` and processes in order:

1. **For loops** → Expand loops, substitute variables, evaluate arithmetic
2. **If/else** → Evaluate conditions, replace with chosen branch
3. **Sum calls** → Replace `sum(~name[N..M])` with addition expression

All expansion happens as string transformation before the parser runs.
