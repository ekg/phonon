# Phonon: Actual Working Syntax (2025-10-18)

## ✅ This Is What ACTUALLY Works

### Basic Syntax

```phonon
# Use COLONS for assignment
tempo: 0.5

# Use SPACE-SEPARATED arguments (NOT parentheses!)
out: sine 440 * 0.2
```

### Working Example - Synthesis

```phonon
tempo: 0.5
~bass: saw 55
out: ~bass * 0.4
```

**Result**: ✅ Renders successfully (RMS: -14.8 dB, Peak: -9.9 dB)

### Working Example - Pattern Modulation

```phonon
tempo: 0.5
~lfo: sine 0.5 * 0.5 + 0.5
~bass: saw 55 # lpf (~lfo * 2000 + 500) 0.8
out: ~bass * 0.4
```

**Result**: ✅ Works (from parser tests)

---

## ❌ What Documentation Says (WRONG)

### README.md Claims

```phonon
# WRONG - Uses equals and parentheses
tempo 2.0              # ❌ Missing colon
~lfo = sine(0.25)      # ❌ Uses = instead of :
out = ~lfo * 0.2       # ❌ Uses = instead of :
```

**Result**: ❌ Produces NO OUTPUT

### CLAUDE.md Claims

```phonon
# WRONG - Uses equals
tempo 2.0              # ❌ Missing colon
~lfo = sine(0.25)      # ❌ Uses = instead of :
```

**Result**: ❌ Produces NO OUTPUT

---

## 🔑 Key Discoveries

1. **Assignment**: Use `:` not `=`
   - ✅ `out: value`
   - ❌ `out = value`

2. **Function calls**: Use SPACES not parentheses
   - ✅ `sine 440`
   - ❌ `sine(440)`

3. **Multiple arguments**: Space-separated
   - ✅ `lpf 1000 0.8`
   - ❌ `lpf(1000, 0.8)`

4. **Bus references**: Still use `~` prefix
   - ✅ `~lfo`

5. **Operators**: Standard math
   - ✅ `* 0.5`
   - ✅ `+ 200`

6. **Signal chains**: Use `#`
   - ✅ `saw 55 # lpf 1000 0.8`

---

## 📝 Correct Syntax Reference

### Assignment

```phonon
tempo: 0.5                    # Set tempo
~busname: expression          # Create bus
out: expression               # Set output
```

### Oscillators

```phonon
sine 440                      # Sine wave at 440 Hz
saw 110                       # Sawtooth
square 220                    # Square wave
noise                         # White noise
```

### Filters

```phonon
lpf 1000 0.8                  # Low-pass (cutoff, Q)
hpf 2000 0.5                  # High-pass (cutoff, Q)
```

### Signal Chain

```phonon
saw 55 # lpf 1000 0.8        # Chain with #
```

### Math

```phonon
~a * 0.5                      # Multiply
~a + ~b                       # Add
~osc * ~lfo + 200             # Complex
```

### Samples (NOT YET TESTED)

```phonon
s "bd sn hh*4"                # Pattern string
```

---

## 🎯 Minimal Working Examples

### 1. Simple Tone

```phonon
tempo: 0.5
out: sine 440 * 0.2
```

### 2. Bass Synth

```phonon
tempo: 0.5
~bass: saw 55
out: ~bass * 0.4
```

### 3. LFO Modulation

```phonon
tempo: 0.5
~lfo: sine 0.5 * 0.5 + 0.5
~bass: saw 55 # lpf (~lfo * 2000 + 500) 0.8
out: ~bass * 0.4
```

### 4. Two Oscillators

```phonon
tempo: 0.5
~bass: saw 55 * 0.3
~lead: square 440 * 0.2
out: ~bass + ~lead
```

---

## 🚨 What Still Doesn't Work

1. **Auto-routing**: `~d1`, `~d2` don't automatically route to output
2. **Pattern modulation**: `sine "110 220 440"` - NOT YET TESTED
3. **Sample playback**: `s "bd sn"` - NOT YET TESTED
4. **Multi-output**: `out1:`, `out2:` - NOT IMPLEMENTED
5. **Transform operators**: `$` - NOT IMPLEMENTED

---

## 📊 Test Results

| Syntax | Result | Output |
|--------|--------|--------|
| `tempo: 0.5` + `out: sine 440 * 0.2` | ✅ WORKS | RMS: -19.0 dB |
| `tempo: 0.5` + `out: saw 55 * 0.4` | ✅ WORKS | RMS: -14.8 dB |
| `tempo 2.0` + `out = sine(440) * 0.2` | ❌ FAILS | No output |
| `~d1: sine 440` (auto-route) | ❌ FAILS | No output |

---

## 🎯 Action Required

1. **Update README.md**: Change all examples to use `:` and space-separated syntax
2. **Update CLAUDE.md**: Fix syntax examples
3. **Rewrite all examples/**: 32 files using wrong Glicol syntax
4. **Update QUICK_START.md**: Use correct syntax
5. **Test sample playback**: Verify `s "bd sn"` works
6. **Test pattern modulation**: Verify `sine "110 220"` works

---

**Last Updated**: 2025-10-18
**Tested With**: Phonon commit b4e8038 (parser unification)
**Status**: ✅ BASIC SYNTHESIS WORKS, many features untested
