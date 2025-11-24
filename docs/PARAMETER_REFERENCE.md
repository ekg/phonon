# Phonon Parameter Reference

Quick reference for ALL function parameters. Use kwargs like `: gain :amount 0.8` or positional `gain 0.8`.

## Common Functions You're Asking About

### gain
- **Parameters**: `:amount` (float)
- **Example**: `gain 0.8` or `gain :amount 0.8`
- **Note**: Not `:level` - it's `:amount`!

### reverb (Freeverb-style)
- **Required**: `:room_size` (0-1), `:damping` (0-1)
- **Optional**: `:mix` (0-1, default 0.3)
- **Example**: `reverb 0.8 0.5` or `reverb :room_size 0.8 :damping 0.5 :mix 0.4`
- **Note**: Not `:wet` - it's `:mix`!

### plate (Dattorro plate reverb - the "sweet reverb")
- **Required**: `:pre_delay` (seconds), `:decay` (seconds)
- **Optional**: `:diffusion` (0-1, default 0.7), `:damping` (0-1, default 0.3), `:mod_depth` (0-1, default 1.0), `:mix` (0-1, default 1.0)
- **Example**: `plate 0.02 3.0` or `plate :pre_delay 0.02 :decay 3.0 :mix 0.4`
- **Note**: This is the lush, dense reverb you're looking for!

## Filters

### lpf (Low-pass filter)
- **Required**: `:cutoff` (Hz)
- **Optional**: `:q` (float, default 1.0)
- **Example**: `lpf 800` or `lpf :cutoff 800 :q 1.5`

### hpf (High-pass filter)
- **Required**: `:cutoff` (Hz)
- **Optional**: `:q` (float, default 1.0)
- **Example**: `hpf 200 :q 0.8`

### bpf (Band-pass filter)
- **Required**: `:cutoff` (Hz)
- **Optional**: `:q` (float, default 1.0)
- **Example**: `bpf 1000 :q 2.0`

## Effects

### delay
- **Required**: `:input` (signal), `:time` (seconds)
- **Optional**: `:feedback` (0-1, default 0.5), `:mix` (0-1, default 0.5)
- **Example**: `delay 0.25 :feedback 0.6 :mix 0.4`

### chorus
- **Required**: `:input` (signal), `:rate` (Hz), `:depth` (0-1)
- **Optional**: `:mix` (0-1, default 0.5)
- **Example**: `chorus 0.5 0.3 :mix 0.3`

### distortion
- **Required**: `:input` (signal), `:drive` (float)
- **Optional**: `:mix` (0-1, default 1.0)
- **Example**: `distortion 5.0 :mix 0.8`

### compressor
- **Required**: `:input` (signal), `:threshold` (dB), `:ratio` (float), `:attack` (seconds), `:release` (seconds)
- **Optional**: `:makeup_gain` (dB, default 0.0)
- **Example**: `compressor -20 4.0 0.01 0.1 :makeup_gain 6.0`

### bitcrush
- **Required**: `:input` (signal), `:bits` (int), `:sample_rate` (Hz)
- **Example**: `bitcrush 8 8000`

## Oscillators

### sine
- **Required**: `:freq` (Hz or pattern)
- **Optional**: `:semitone_offset` (float, default 0.0)
- **Example**: `sine 440` or `sine "220 330 440" :semitone_offset 12.0`

### saw
- **Required**: `:freq` (Hz or pattern)
- **Optional**: `:semitone_offset` (float, default 0.0)
- **Example**: `saw 110 :semitone_offset -12.0`

### square
- **Required**: `:freq` (Hz or pattern)
- **Optional**: `:semitone_offset` (float, default 0.0)
- **Example**: `square 220`

### triangle
- **Required**: `:freq` (Hz or pattern)
- **Optional**: `:semitone_offset` (float, default 0.0)
- **Example**: `triangle 440`

## Envelopes

### adsr
- **Required**: `:trigger` (signal), `:attack` (seconds), `:decay` (seconds), `:sustain` (0-1), `:release` (seconds)
- **Example**: `adsr ~trigger 0.01 0.1 0.7 0.2`

### ar (Attack-Release)
- **Required**: `:trigger` (signal), `:attack` (seconds), `:release` (seconds)
- **Example**: `ar ~trigger 0.01 0.3`

## Pattern Functions

### s (Sample playback)
- **Required**: `:samples` (mini-notation string)
- **Example**: `s "bd sn hh*4 cp"`

### fast
- **Required**: `:factor` (float or pattern)
- **Example**: `fast 2` or `fast "2 3 4"`

### slow
- **Required**: `:factor` (float or pattern)
- **Example**: `slow 2`

### rev (Reverse)
- **Example**: `rev`

### every
- **Required**: `:n` (int), `:f` (function)
- **Example**: `every 4 rev`

## Mixing

### pan
- **Required**: `:input` (signal), `:position` (-1 to 1)
- **Example**: `pan 0.5`  # Center, -1 = left, 1 = right

### xfade (Crossfade)
- **Required**: `:a` (signal), `:b` (signal), `:mix` (0-1)
- **Example**: `xfade ~drums ~bass 0.5`

## Quick Discovery in Editor

When editing in the modal editor:

1. **Tab completion**: Type function name and hit Tab - shows functions
2. **Kwargs completion**: Type `function :` and hit Tab - shows parameter names
3. **Shift+Tab**: Expands function with ALL kwargs and defaults
   - Example: `plate<Shift+Tab>` → `plate :pre_delay 1.0 :decay 1.0 :diffusion 0.7 :damping 0.3 :mod_depth 1.0 :mix 1.0`

## Common Mistakes

- ❌ `gain :level 0.8` → ✅ `gain :amount 0.8`
- ❌ `reverb :wet 0.5` → ✅ `reverb :mix 0.5`
- ❌ `lpf 800 :cutoff` → ✅ `lpf :cutoff 800` (kwarg needs value!)
- ❌ `sine(440)` → ✅ `sine 440` (no parentheses/commas in Phonon!)

## Pattern-Controlled Parameters

**ALL parameters can be patterns**! This is Phonon's superpower:

```phonon
-- Modulate filter cutoff with pattern
~bass: saw 55 # lpf "500 2000 1000" 0.8

-- Modulate with LFO
~lfo: sine 0.25
~bass: saw 55 # lpf (~lfo * 2000 + 500) 0.8

-- Fast-changing effects
~drums: s "bd sn" # gain "0.8 1.2 0.6 1.0"
```

