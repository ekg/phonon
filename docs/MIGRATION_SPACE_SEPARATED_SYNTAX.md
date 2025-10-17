# Migration Guide: Space-Separated Function Arguments

## ⚠️ BREAKING CHANGE

This update removes parenthesized function call syntax and adopts Tidal/Haskell-style space-separated arguments.

**Parentheses are now ONLY for grouping expressions**, not for function application.

## Why This Change?

1. **Tidal Compatibility**: Matches TidalCycles syntax exactly
2. **Cleaner Syntax**: Reduces visual clutter, easier to read
3. **Consistent Philosophy**: Functions apply like Haskell, parentheses group like math
4. **Future-Proof**: Enables systematic nesting and composition

## Migration Examples

### Sample Patterns

**OLD (NO LONGER WORKS):**
```phonon
s("bd sn hh")
s("bd*4")
```

**NEW (REQUIRED):**
```phonon
s "bd sn hh"
s "bd*4"
```

### DSP Parameters

**OLD:**
```phonon
s("bd") # gain(0.5)
s("hh*16") # gain("0.5 1.0") # pan("-1 1")
```

**NEW:**
```phonon
s "bd" # gain 0.5
s "hh*16" # gain "0.5 1.0" # pan "-1 1"
```

### Oscillators

**OLD:**
```phonon
sine(440)
saw("110 220")
square(220, 0.3)  // with duty cycle
```

**NEW:**
```phonon
sine 440
saw "110 220"
square 220 0.3
```

### Filters

**OLD:**
```phonon
lpf(1000, 0.8)
s("bd") # lpf(2000, 0.5)
```

**NEW:**
```phonon
lpf 1000 0.8
s "bd" # lpf 2000 0.5
```

### Effects

**OLD:**
```phonon
delay(0.25, 0.5, 0.3)
reverb(0.8, 0.5, 0.3)
distortion(5.0)
```

**NEW:**
```phonon
delay 0.25 0.5 0.3
reverb 0.8 0.5 0.3
distortion 5.0
```

### Complete Example

**OLD:**
```phonon
bpm 120

~lfo: sine(0.25)
~bass: saw("55 82.5") # lpf(~lfo * 2000 + 500, 0.8) # gain(0.6)
~drums: s("bd sn hh*4") # gain("1.0 0.8 0.5")

out: (~bass + ~drums) * 0.7
```

**NEW:**
```phonon
bpm 120

~lfo: sine 0.25
~bass: saw "55 82.5" # lpf (~lfo * 2000 + 500) 0.8 # gain 0.6
~drums: s "bd sn hh*4" # gain "1.0 0.8 0.5"

out: (~bass + ~drums) * 0.7
```

## Important Notes

### Parentheses Are Still Used!

Parentheses are **essential for grouping** expressions:

```phonon
# Grouping arithmetic
out: (a + b) * c        # ✓ Correct: multiply sum by c
out: a + b * c          # ✗ Different: add a to product

# Grouping in function arguments
lpf (~lfo * 2000 + 500) 0.8    # ✓ Correct: LFO modulates cutoff
lpf ~lfo * 2000 + 500 0.8      # ✗ Parse error
```

### Pattern Strings Still Use Quotes

Pattern strings **always** use double quotes:

```phonon
s "bd sn"              # ✓ Pattern string
gain "0.5 1.0"         # ✓ Pattern values
sine "110 220 440"     # ✓ Pattern frequencies
```

### Multi-Argument Functions

Space-separated arguments work naturally:

```phonon
# Two arguments
lpf 1000 0.8           # cutoff=1000, q=0.8

# Three arguments
delay 0.25 0.5 0.3     # time=0.25, feedback=0.5, mix=0.3
reverb 0.8 0.5 0.3     # room=0.8, damp=0.5, mix=0.3

# Pattern + constant
s "bd*4" # gain 0.8
```

## Automatic Migration

A conversion script is provided to update files automatically:

```bash
# Convert all files in your project
./scripts/convert_to_space_separated.sh

# Manual pattern (examples)
sed -i 's/s("\([^"]*\)")/s "\1"/g' your_file.ph
sed -i 's/gain(\([0-9.]\+\))/gain \1/g' your_file.ph
sed -i 's/lpf(\([0-9.]\+\), \([0-9.]\+\))/lpf \1 \2/g' your_file.ph
```

## Rollback

If you need to revert, check out the commit before this change:

```bash
git log --oneline | grep "space-separated"
git checkout <commit-before-change>
```

## Questions?

See the [Phonon Language Reference](../PHONON_LANGUAGE_REFERENCE.md) for complete syntax details.

## Test Results

After migration:
- ✓ 201/208 library tests passing (96.6%)
- ✓ All integration tests passing
- ✓ All examples successfully converted
- ⚠️ 7 advanced synth tests need updates (SuperDirt synths)
