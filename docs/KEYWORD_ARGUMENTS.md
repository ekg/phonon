# Keyword Arguments in Phonon

## Overview

Phonon supports optional keyword arguments for all function parameters using the `:param value` syntax. This makes the language more ergonomic and discoverable while maintaining fast positional syntax for live coding.

## Syntax

```phonon
-- Positional arguments (traditional, fast)
~filtered: noise # lpf 1000 0.8

-- Keyword arguments (clear, discoverable)
~filtered: noise # lpf 1000 :q 0.8

-- Mixed (required positional, optional keywords)
~filtered: noise # lpf :cutoff 1000 :q 0.8

-- All keywords (most explicit)
~filtered: noise # lpf :cutoff 1000 :q 0.8
```

## Benefits

1. **Discoverability** - Parameter names make code self-documenting
2. **Autocomplete-friendly** - `:` prefix signals parameter names to editors
3. **Optional parameters** - Skip trailing parameters, use defaults
4. **Flexibility** - Mix positional and keyword as needed
5. **Backwards compatible** - All positional syntax still works

## Converted Functions

### Filters (lpf, hpf, bpf, notch)
```phonon
~f1: ~signal # lpf 1000           -- q defaults to 1.0
~f2: ~signal # lpf 1000 0.8       -- positional q
~f3: ~signal # lpf 1000 :q 0.8    -- keyword q
~f4: ~signal # lpf :cutoff 1000 :q 0.8  -- all keywords
```

**Parameters:**
- `cutoff` (required) - Filter cutoff frequency in Hz
- `q` (optional, default 1.0) - Filter resonance/Q factor

### ADSR Envelope
```phonon
~env1: adsr 0.01 0.1                        -- sustain=0.7, release=0.2
~env2: adsr 0.01 0.1 0.8 0.3                -- all positional
~env3: adsr 0.01 0.1 :sustain 0.8           -- release=0.2
~env4: adsr 0.01 0.1 :sustain 0.8 :release 0.3  -- keywords
~env5: adsr :attack 0.01 :decay 0.1 :sustain 0.8 :release 0.3  -- all keywords
```

**Parameters:**
- `attack` (required) - Attack time in seconds
- `decay` (required) - Decay time in seconds
- `sustain` (optional, default 0.7) - Sustain level (0-1)
- `release` (optional, default 0.2) - Release time in seconds

### AD Envelope (Attack-Decay)
```phonon
~env: ad 0.01 0.3                 -- positional
~env: ad :attack 0.01 :decay 0.3  -- keywords
```

**Parameters:**
- `attack` (required) - Attack time in seconds
- `decay` (required) - Decay time in seconds

### ASR Envelope (Attack-Sustain-Release)
```phonon
~env: asr ~trigger 0.02 0.15                         -- positional
~env: asr :gate ~trigger :attack 0.02 :release 0.15  -- keywords
```

**Parameters:**
- `gate` (required) - Gate signal (trigger)
- `attack` (required) - Attack time in seconds
- `release` (required) - Release time in seconds

### Reverb
```phonon
~wet: ~dry # reverb 0.8 0.5          -- mix=0.3 (30% wet)
~wet: ~dry # reverb 0.8 0.5 0.5      -- 50% wet
~wet: ~dry # reverb 0.8 0.5 :mix 0.5 -- keyword mix
~wet: ~dry # reverb :room_size 0.8 :damping 0.5 :mix 0.5  -- all keywords
```

**Parameters:**
- `room_size` (required) - Room size (0-1)
- `damping` (required) - High frequency damping (0-1)
- `mix` (optional, default 0.3) - Wet/dry mix (0-1)

### Chorus
```phonon
~ch: ~dry # chorus 2.0 0.3          -- mix=0.3 (30% wet)
~ch: ~dry # chorus 2.0 0.3 0.5      -- 50% wet
~ch: ~dry # chorus 2.0 0.3 :mix 0.5 -- keyword mix
~ch: ~dry # chorus :rate 2.0 :depth 0.3 :mix 0.5  -- all keywords
```

**Parameters:**
- `rate` (required) - LFO rate in Hz
- `depth` (required) - Modulation depth (0-1)
- `mix` (optional, default 0.3) - Wet/dry mix (0-1)

### Delay
```phonon
~echo: ~dry # delay 0.25                    -- feedback=0.5, mix=0.5
~echo: ~dry # delay 0.25 0.6 0.4            -- all positional
~echo: ~dry # delay 0.25 :feedback 0.6      -- mix=0.5
~echo: ~dry # delay 0.25 :feedback 0.6 :mix 0.4  -- keywords
~echo: ~dry # delay :time 0.25 :feedback 0.6 :mix 0.4  -- all keywords
```

**Parameters:**
- `time` (required) - Delay time in seconds
- `feedback` (optional, default 0.5) - Feedback amount (0-1)
- `mix` (optional, default 0.5) - Wet/dry mix (0-1)

### Distortion
```phonon
~heavy: ~clean # distort 5.0              -- mix=0.5 (50% wet/dry)
~heavy: ~clean # distort 5.0 0.8          -- 80% wet
~heavy: ~clean # distort 5.0 :mix 0.8     -- keyword mix
~heavy: ~clean # distort :drive 5.0 :mix 0.8  -- all keywords
```

**Parameters:**
- `drive` (required) - Distortion amount/gain
- `mix` (optional, default 0.5) - Wet/dry mix (0-1)

## Design Philosophy

### Positional for Speed
```phonon
-- Live coding: every keystroke counts
~fx: s "bd sn" # lpf 2000 # reverb 0.8 0.5 # delay 0.25
```

### Keywords for Clarity
```phonon
-- Reading later: what do these numbers mean?
~fx: s "bd sn"
  # lpf :cutoff 2000
  # reverb :room_size 0.8 :damping 0.5
  # delay :time 0.25
```

### Mixed Approach
```phonon
-- Best of both: required params positional, optional keywords
~fx: s "bd sn"
  # lpf 2000 :q 0.8
  # reverb 0.8 0.5 :mix 0.4
  # delay 0.25 :feedback 0.6
```

## Implementation Details

### ParamExtractor Helper

All converted functions use the `ParamExtractor` helper in `src/compositional_compiler.rs`:

```rust
let extractor = ParamExtractor::new(args);

// Required parameter
let freq_expr = extractor.get_required(0, "cutoff")?;

// Optional parameter with default
let q_expr = extractor.get_optional(1, "q", 1.0);
```

**Priority order:**
1. Positional argument at index (if present)
2. Keyword argument by name (if present)
3. Default value (for optional params only)

### Parser Support

The parser in `src/compositional_parser.rs` supports `:param value` syntax:

```rust
fn parse_kwarg(input: &str) -> IResult<&str, Expr> {
    // Parse :name value syntax
    let (rest, _) = char(':')(input)?;
    let (rest, name) = parse_identifier(rest)?;
    let (rest, _) = space1(rest)?;
    let (rest, value) = parse_primary_expr(rest)?;

    Ok((rest, Expr::Kwarg { name, value }))
}
```

## Future Work

Functions that could benefit from keyword arguments but haven't been converted yet:

- Oscillators (sine, saw, square, tri) - currently only take frequency
- Pattern transforms (fast, slow, every) - all required params currently
- Other effects (bitcrush, coarse, djf, vowel) - may add optional params

The infrastructure is in place to easily convert any function when optional parameters would be useful.

## Examples

### Full Song Snippet
```phonon
tempo: 2.0

-- Drums with envelope shaping
~drums: s "bd sn hh cp"
~shaped: ~drums # adsr 0.01 0.1 :sustain 0.7 :release 0.2

-- Bass with filtered synthesis
~bass: saw "55 82.5"
  # lpf 800 :q 1.5
  # distort 2.0 :mix 0.6

-- Pad with reverb and chorus
~pad: sine "220 330 440"
  # reverb 0.9 0.6 :mix 0.5
  # chorus 1.5 0.2 :mix 0.3

-- Delay effect on everything
~delayed: (~shaped + ~bass + ~pad)
  # delay 0.375 :feedback 0.4 :mix 0.3

out: ~delayed * 0.25
```

This example shows how keyword arguments make complex signal chains more readable while maintaining the live-coding flow.
