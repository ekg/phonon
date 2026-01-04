# Phonon Doc Comment Specification

This document defines the standard format for documenting Phonon functions so they can be automatically extracted by `build.rs` for tab completion, help, and documentation.

## Format Overview

```rust
/// Short description (first line - used in completion popup)
///
/// Longer description with more details about behavior, use cases,
/// and any important notes. This is shown in the documentation panel.
///
/// # Parameters
/// * `param_name` - Description (type, default: value)
/// * `param_name` - Description (type, required)
///
/// # Example
/// ```phonon
/// ~bass $ saw 55 # lpf 800 :q 1.5
/// ```
///
/// # Category
/// CategoryName
pub fn function_name(...) -> ...
```

## Field Specifications

### Short Description (Required)
- First line of the doc comment
- Should be concise (< 60 chars)
- Used in completion popup and search
- No period at end

### Long Description (Optional)
- Additional paragraphs after the first line
- Can include multiple paragraphs
- Shown in documentation panel

### Parameters (Required if function has params)
- Use `# Parameters` header
- Format: `* \`name\` - Description (type, default: value)`
- Type can be: `Hz`, `float`, `int`, `cycles`, `pattern`, `string`
- Mark required params with `required` instead of default
- Order must match function signature

### Example (Required)
- Use `# Example` header
- Code block with `phonon` language tag
- Should be a working, copy-pasteable example
- Show common use case

### Category (Required)
- Use `# Category` header
- Single word on next line
- Valid categories:
  - `Oscillators` - sine, saw, tri, square, pulse, noise, blip
  - `Filters` - lpf, hpf, bpf, notch
  - `Effects` - reverb, delay, chorus, distortion, compressor
  - `Transforms` - fast, slow, rev, every, jux
  - `Patterns` - s, n, note, sound
  - `Dynamics` - gain, pan, amp, compressor, limiter
  - `Time` - late, early, swing, press, rotL, rotR
  - `Structure` - stack, cat, seq, layer
  - `Generators` - rand, choose, irand

## Examples

### Filter Function
```rust
/// Low-pass filter - removes frequencies above cutoff
///
/// Attenuates frequencies above the cutoff point. Use higher Q values
/// for resonance at the cutoff frequency. Great for bass sounds and
/// removing harshness.
///
/// # Parameters
/// * `cutoff` - Filter cutoff frequency (Hz, required)
/// * `q` - Resonance/Q factor 0.1-10 (float, default: 1.0)
///
/// # Example
/// ```phonon
/// ~bass $ saw 55 # lpf 800 :q 1.5
/// ```
///
/// # Category
/// Filters
```

### Pattern Transform
```rust
/// Speed up pattern by factor
///
/// Compresses the pattern in time, making events happen faster.
/// A factor of 2 plays the pattern twice as fast (twice per cycle).
///
/// # Parameters
/// * `factor` - Speed multiplier (float, required)
///
/// # Example
/// ```phonon
/// ~drums $ s "bd sn" $ fast 2
/// ```
///
/// # Category
/// Transforms
```

### Time Manipulation
```rust
/// Shift pattern forward in time
///
/// Delays all events in the pattern by the specified amount.
/// Useful for creating offbeat patterns or phase effects.
///
/// # Parameters
/// * `amount` - Shift amount in cycles (cycles, required)
///
/// # Example
/// ```phonon
/// ~offbeat $ s "hh*4" $ late 0.5
/// ```
///
/// # Category
/// Time
```

## Parsing Notes for build.rs

The parser should:
1. Extract first line as `description`
2. Collect remaining paragraphs before `# Parameters` as `long_description`
3. Parse `# Parameters` section into `Vec<ParamMetadata>`
4. Extract code from `# Example` section
5. Extract category from `# Category` section

Parameter parsing regex pattern:
```
\* `(\w+)` - (.+?) \((\w+), (default: .+|required)\)
```

## Migration Strategy

1. Start with high-traffic functions (lpf, hpf, fast, slow, s, reverb)
2. Add doc comments following this spec
3. Update build.rs to parse and generate metadata
4. Deprecate manual FUNCTION_METADATA entries as functions get documented
5. Add CI check to ensure all functions have proper doc comments
