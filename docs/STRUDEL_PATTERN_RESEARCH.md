# Strudel Pattern Research for Phonon

**Date**: 2025-01-28
**Purpose**: Document Strudel patterns, idioms, and functions to inform Phonon development

---

## Executive Summary

Strudel is the JavaScript port of TidalCycles, running in the browser. This research documents its pattern language, functions, and idioms that are relevant for achieving Tidal/Strudel parity in Phonon.

**Key Finding**: Phonon already implements most core features. The main gaps are in:
1. Sample manipulation functions (splice, fit, scrub)
2. Random/probability functions (choose, wchoose)
3. Some audio effects (tremolo, phaser controls)
4. Music theory functions (voicing, chord symbols)

---

## 1. Mini-Notation Syntax Comparison

### Phonon ✅ Already Has

| Syntax | Symbol | Example | Status |
|--------|--------|---------|--------|
| Sequences | space | `"bd sn hh"` | ✅ |
| Groups | `[ ]` | `"[bd sn] hh"` | ✅ |
| Rests | `~` | `"bd ~ sn ~"` | ✅ |
| Repeat | `*n` | `"bd*4"` | ✅ |
| Slow | `/n` | `"bd/2"` | ✅ |
| Alternation | `< >` | `"<bd sn cp>"` | ✅ |
| Polyphony | `,` | `"[bd, hh hh]"` | ✅ |
| Degrade | `?` | `"bd?"` | ✅ |
| Replicate | `!` | `"bd!"` | ✅ |
| Euclidean | `(n,m)` | `"bd(3,8)"` | ✅ |

### Missing/Different in Phonon

| Syntax | Strudel | Phonon Status |
|--------|---------|---------------|
| Elongation weight | `@n` | ❌ Missing - assigns temporal weight |
| Random choice | `\|` | ❌ Missing - random choice between options |
| Degrade probability | `?0.8` | ⚠️ Check implementation |

---

## 2. Time Modifiers

### Phonon ✅ Already Has

| Function | Example | Status |
|----------|---------|--------|
| `fast` | `s("bd sn").fast(2)` | ✅ |
| `slow` | `s("bd sn").slow(2)` | ✅ |
| `rev` | `s("bd sn").rev()` | ✅ |
| `early` | `s("bd").early(0.1)` | ⚠️ Defined, not tested |
| `late` | `s("bd").late(0.1)` | ⚠️ Defined, not tested |
| `inside` | `.inside(4, rev)` | ⚠️ Defined, not tested |
| `outside` | `.outside(4, rev)` | ⚠️ Defined, not tested |
| `swing` | `s("hh*8").swing(4)` | ⚠️ Defined, not tested |
| `rotL`/`rotR` | Shift query + results | ✅ |
| `fastGap` | Compress with gap | ✅ |
| `zoom` | Extract time span | ✅ |
| `press`/`pressBy` | Delay by fraction | ✅ |

### Missing in Phonon

| Function | Strudel Usage | Priority |
|----------|---------------|----------|
| `swingBy` | `s("hh*8").swingBy(1/3, 4)` | P3 |
| `off` | `.off(1/8, x=>x.add(7))` | P2 - 35 uses in livecode |

---

## 3. Conditional Modifiers

### Phonon ✅ Already Has

| Function | Example | Status |
|----------|---------|--------|
| `every` | `s("bd sn").every(3, rev)` | ✅ |
| `firstOf` | `.firstOf(4, rev)` | ⚠️ |
| `lastOf` | `.lastOf(4, rev)` | ⚠️ |
| `chunk` | `.chunk(4, x=>x.add(7))` | ✅ |
| `whenmod` | Conditional on cycle | ⚠️ |
| `sometimes` | `.sometimesBy(0.5, fn)` | ⚠️ |
| `often` | `.sometimesBy(0.75, fn)` | ⚠️ |
| `rarely` | `.sometimesBy(0.25, fn)` | ⚠️ |
| `degradeBy` | `.degradeBy(0.2)` | ✅ |
| `degrade` | `.degradeBy(0.5)` | ✅ |

### Missing in Phonon

| Function | Strudel Usage | Priority |
|----------|---------------|----------|
| `foldEvery` | Apply at multiple intervals | P2 |
| `when` | Conditional on pattern | P3 |
| `mask` | Silence when mask=0 | P3 |
| `struct` | Apply structure | **P1** - 284 uses! |
| `almostNever` | `.sometimesBy(0.1, fn)` | Low |
| `almostAlways` | `.sometimesBy(0.9, fn)` | Low |
| `never` | `.sometimesBy(0, fn)` | Low |
| `always` | `.sometimesBy(1, fn)` | Low |

---

## 4. Sample Manipulation

### Phonon ✅ Already Has

| Function | Example | Status |
|----------|---------|--------|
| `chop` | `s("rhodes").chop(4)` | ✅ |
| `striate` | `s("numbers").striate(6)` | ✅ |
| `slice` | `s("breaks165").slice(8, "0 1 2 3")` | ✅ |
| `loopAt` | `s("rhodes").loopAt(2)` | ✅ |
| `begin`/`end` | `.begin(0.25).end(0.75)` | ✅ |
| `speed` | `.speed("1 2 -1 -2")` | ✅ |
| `cut` | `s("oh hh").cut(1)` | ✅ |

### Missing in Phonon

| Function | Strudel Usage | Priority |
|----------|---------------|----------|
| `splice` | Like slice but adjusts speed | P2 |
| `fit` | Fit sample to event duration | P3 |
| `scrub` | Scrub audio like tape | P4 |
| `clip` | Multiply duration, truncate | P3 |

---

## 5. Audio Effects

### Phonon ✅ Already Has

| Effect | Example | Status |
|--------|---------|--------|
| `lpf`/`hpf`/`bpf` | `.lpf(2000)` | ✅ |
| `lpq`/`hpq` | `.lpf(2000).lpq(10)` | ⚠️ Check |
| `gain` | `.gain(0.5)` | ✅ |
| `pan` | `.pan(0.5)` | ✅ |
| `room` | `.room(0.8)` | ✅ |
| `delay` | `.delay(0.5)` | ✅ |
| `distort` | `.distort(2)` | ✅ |
| `crush` | `.crush(8)` | ✅ |
| `attack`/`decay`/`sustain`/`release` | ADSR | ✅ |
| `jux`/`juxBy` | `.jux(rev)` | ✅ |

### Missing in Phonon

| Effect | Strudel Usage | Priority |
|--------|---------------|----------|
| `vowel` | Formant filter | P3 |
| `coarse` | Fake resampling | P4 |
| `tremolosync` | Amplitude modulation | P3 |
| `tremolodepth` | Tremolo amount | P3 |
| `phaser` | Phaser effect | P3 |
| `phaserdepth` | Phaser controls | P3 |
| `compressor` | Dynamics | P3 (have compressor, check params) |
| `velocity` | 0-1 scaled gain | P4 |
| `postgain` | Post-effects gain | P4 |
| `ftype` | Filter type (12db/ladder/24db) | P3 |
| Pitch envelope | `penv`, `pattack`, `pdecay` | P3 |
| Filter envelope | `lpenv`, `lpa`, `lpd` | P3 |
| Ducking/sidechain | `duckorbit`, `duckattack` | P3 |

---

## 6. Signal Functions

### Phonon ✅ Already Has

| Signal | Example | Status |
|--------|---------|--------|
| `sine` | `sine 0.25` | ✅ |
| `saw` | `saw 55` | ✅ |
| `square` | `square 110` | ✅ |
| `tri` | `tri 440` | ✅ |

### Missing/Needs Enhancement in Phonon

| Signal | Strudel Usage | Priority |
|--------|---------------|----------|
| `rand` | Continuous random 0-1 | P2 |
| `irand(n)` | Random integer 0 to n-1 | P2 |
| `perlin` | Perlin noise 0-1 | P3 |
| `brand` | Binary random (0 or 1) | P4 |
| `brandBy(p)` | Binary with probability | P4 |
| `.segment(n)` | Discretize signal | P3 |
| `.range(lo, hi)` | Remap signal range | P3 |
| `mouseX`/`mouseY` | Interactive input | P4 |

**Note**: Phonon's signal patterns evaluate at audio rate, which is MORE powerful than Strudel. In Strudel, signals are typically used for parameter control. In Phonon, patterns ARE synthesis.

---

## 7. Pattern Factory Functions

### Phonon ✅ Already Has

| Function | Example | Status |
|----------|---------|--------|
| `stack` | `stack("g3", "b3")` | ✅ |
| `cat`/`fastcat` | Concatenate patterns | ⚠️ Check |

### Missing in Phonon

| Function | Strudel Usage | Priority |
|----------|---------------|----------|
| `seq`/`sequence` | Like cat, cram into cycle | P3 |
| `slowcat` | Each item takes one cycle | P3 |
| `stepcat` | Proportional concatenation | P4 |
| `arrange` | Multi-cycle arrangement | P4 |
| `polymeter` | Align steps, create polymeters | P3 |
| `layer` | Layer patterns with transforms | P3 |
| `superimpose` | Add transformed copy on top | P2 |

---

## 8. Random/Choice Functions

### Missing in Phonon (High Priority Gap)

| Function | Strudel Usage | Priority |
|----------|---------------|----------|
| `choose` | `s(choose("bd", "sn", "hh"))` | P2 |
| `wchoose` | Weighted random choice | P2 |
| `chooseCycles`/`randcat` | Pick one each cycle | P2 |
| `wchooseCycles` | Weighted per-cycle choice | P3 |
| `undegradeBy` | Inverse of degradeBy | P4 |

---

## 9. Music Theory Functions

### Missing in Phonon

| Function | Strudel Usage | Priority |
|----------|---------------|----------|
| `scale` | `n("0 2 4").scale("C:major")` | P2 |
| `voicing` | Chord voicings | P3 |
| `transpose` | Shift semitones | P2 |
| `scaleTranspose` | Shift in scale steps | P3 |
| `rootNotes` | Chord root notes | P4 |
| `chord` | Chord symbols | P3 |
| `arp` | Arpeggiate voicing | P3 |

---

## 10. Priority Implementation Recommendations

### P1 - Critical (High frequency in livecode, major functionality gaps)

1. **`struct`** - 284 uses in livecode! Apply structure from pattern
2. **`stut`** - 132 uses - Stutter/echo effect
3. **`off`** - 35 uses - Offset transform for delays

### P2 - High (Common patterns, enhance expressiveness)

4. **`choose`/`wchoose`** - Random selection
5. **`superimpose`** - Layer transformed copies
6. **`scale`** - Scale quantization
7. **`transpose`** - Pitch shifting
8. **`splice`** - Sample slicing with speed adjust
9. **`rand`/`irand`** - Random signal patterns
10. **`hurry`** - Speed up and pitch

### P3 - Medium (Nice to have, complete parity)

11. **`foldEvery`** - Multiple interval conditions
12. **`when`** - Pattern-based conditions
13. **Tremolo/phaser controls** - Effect parameters
14. **Filter envelopes** - `lpenv`, `lpa`, etc.
15. **`polymeter`** - Polymetric patterns
16. **`segment`/`range`** - Signal manipulation
17. **Music theory** - voicing, chord, arp

### P4 - Low (Specialized, niche usage)

18. **`scrub`** - Tape-style scrubbing
19. **Interactive inputs** - mouseX, mouseY
20. **Ducking/sidechain** - Dynamic effects
21. **Various convenience aliases**

---

## 11. Phonon Advantages Over Strudel

### Audio-Rate Pattern Evaluation
In Strudel, patterns trigger discrete events. In Phonon, patterns ARE continuous signals:

```phonon
-- This is UNIQUE to Phonon:
~lfo $ sine 0.25
out $ saw "55 82.5" # lpf (~lfo * 2000 + 500) 0.8
-- Pattern modulates filter cutoff at audio rate!
```

Strudel cannot do this - it can only use signals for parameter interpolation, not true audio-rate synthesis modulation.

### Compiled Performance
Phonon compiles to a signal graph evaluated in Rust. Strudel runs in JavaScript. Phonon will be significantly more performant for complex synthesis.

### Native Integration
Phonon can integrate with the native audio stack, VST plugins, and MIDI hardware. Strudel is browser-only (though supports OSC/MIDI via WebMIDI).

---

## 12. Recommended Testing Approach

For each implemented function, use Phonon's three-level testing:

1. **Pattern Query Test** - Verify correct event generation
2. **Onset Detection Test** - Verify audio events at correct times
3. **Audio Characteristics Test** - Verify signal quality

---

## Sources

- [Strudel Patterns Technical Manual](https://strudel.cc/technical-manual/patterns/)
- [Strudel Mini-Notation](https://strudel.cc/learn/mini-notation/)
- [Strudel Time Modifiers](https://strudel.cc/learn/time-modifiers/)
- [Strudel Conditional Modifiers](https://strudel.cc/learn/conditional-modifiers/)
- [Strudel Random Modifiers](https://strudel.cc/learn/random-modifiers/)
- [Strudel Samples](https://strudel.cc/learn/samples/)
- [Strudel Effects](https://strudel.cc/learn/effects/)
- [Strudel Signals](https://strudel.cc/learn/signals/)
- [Strudel Pattern Factories](https://strudel.cc/learn/factories/)
- [Strudel Tonal Functions](https://strudel.cc/learn/tonal/)
- [Strudel Stepwise Patterning](https://strudel.cc/learn/stepwise/)
- [Strudel Workshop](https://strudel.cc/workshop/getting-started/)

---

*Research completed: 2025-01-28*
