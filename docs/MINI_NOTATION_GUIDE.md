# Mini-Notation Guide

The Phonon mini-notation is a powerful pattern language inspired by TidalCycles/Strudel for creating rhythmic and musical patterns. This guide covers all implemented features with examples.

## Basic Patterns

### Simple Sequences
Patterns are sequences of events separated by spaces:
```
"bd sn hh cp"  // Plays: bass drum, snare, hi-hat, clap
```

### Rests/Silence
Use `~` to insert silence:
```
"bd ~ sn ~"  // Plays: bass drum, (silence), snare, (silence)
```

## Grouping and Subdivision

### Groups with Brackets []
Groups play faster - all elements in the group fit into one step:
```
"bd [sn sn] hh"  
// bd takes 1/3 cycle
// [sn sn] takes 1/3 cycle (each sn is 1/6 cycle)
// hh takes 1/3 cycle
```

### Nested Groups
Groups can be nested:
```
"bd [[sn sn] cp] hh"
// Even more complex subdivisions
```

## Polyphony and Layering

### Chord Syntax with Commas in Brackets
Play multiple patterns simultaneously:
```
"[bd cp, hh hh hh, sn ~ sn ~]"
// Three patterns play at once:
// - bd cp (two events)
// - hh hh hh (three events)  
// - sn ~ sn ~ (two events with rests)
```

### Polyrhythm with Parentheses
Another way to create polyphonic patterns:
```
"(bd, sn cp, hh hh hh)"
// bd plays for full cycle
// sn cp plays two events
// hh hh hh plays three events
// All simultaneously
```

### Stacking with Pipe |
Stack patterns on top of each other:
```
"bd sn | hh hh hh hh"
// bd sn plays as one pattern
// hh hh hh hh plays as another
// Both play simultaneously
```

## Pattern Transformations

### Alternation with Angle Brackets <>
Cycle through patterns, one per cycle:
```
"<bd sn cp>"
// Cycle 0: bd
// Cycle 1: sn  
// Cycle 2: cp
// Cycle 3: bd (repeats)
```

Combined with other patterns:
```
"<bd sn cp> hh"
// Alternating pattern plays alongside hh each cycle
```

### Choice with Curly Braces {} 
Randomly choose from options (simplified implementation):
```
"{bd sn cp}"
// Randomly picks one each time
```

## Operators

### Repeat with *
Repeat an element multiple times:
```
"bd*3 sn"     // Three bd's followed by one sn
"bd sn*4"     // One bd followed by four sn's
```

### Slow with /
Stretch a pattern over multiple cycles:
```
"bd/2 sn"     // bd stretches over 2 cycles, sn plays each cycle
"hh/4"        // hh stretches over 4 cycles
```

### Fast with *
Speed up a pattern (when * used without number):
```
"bd* sn"      // bd plays twice as fast
```

### Late with @
Delay pattern by a fraction of a cycle:
```
"bd@0.25 sn"  // bd starts 1/4 cycle late
```

### Degrade with ?
Randomly drop events:
```
"bd? sn"      // bd might not play (50% chance)
"bd?0.8 sn"   // bd has 80% chance to play
```

### Duplicate with !
Shorthand for *2:
```
"bd! sn"      // Same as bd*2 sn
```

### Elongate with _
Stretch pattern (similar to slow):
```
"bd_ sn"      // bd elongated
```

### Reverse with ^
Reverse the pattern:
```
"bd sn hh^"   // hh pattern reversed
```

## Advanced Features

### Euclidean Rhythms with %
Create evenly distributed rhythms:
```
"bd%3,8"      // 3 hits distributed over 8 steps
```

### Method Chaining with .
Apply methods to patterns:
```
"bd sn.fast"  // Speed up the sn pattern
"hh.rev"      // Reverse the hh pattern
```

## Complex Examples

### Combining Multiple Features
```
"[bd*2 ~, hh hh hh hh] | <sn cp>"
// - bd plays twice then rest, alongside
// - hh playing four times, all while
// - alternating between sn and cp each cycle
```

### Polyrhythmic Drum Pattern
```
"(bd bd, sn ~ sn ~, hh*8)"
// Bass drum: 2 hits
// Snare: 2 hits with rests
// Hi-hat: 8 rapid hits
// All in the same cycle
```

### Evolving Pattern
```
"<bd sn cp kick> [hh hh] | rim*3"
// Different bass sound each cycle
// With consistent hi-hat pattern
// And rimshot triplets on top
```

## Testing Patterns

You can visualize patterns using the debug utilities:

```rust
use phonon::mini_notation::parse_mini_notation;
use phonon::pattern_debug::{pattern_to_ascii, describe_pattern};

let pattern = parse_mini_notation("[bd cp, hh hh hh]");
println!("{}", pattern_to_ascii(&pattern, 2, 32));
println!("{}", describe_pattern(&pattern, 1));
```

This will show:
- ASCII visualization of the pattern over time
- Detailed event timings
- Whether the pattern has polyphony

## Audio Rendering

Patterns can be rendered to audio files:

```bash
phonon render 'out: pattern("[bd cp, hh hh hh, sn ~ sn ~]")' output.wav --duration 4
```

This will:
- Parse the mini-notation
- Sequence the samples from the dirt-samples library
- Mix overlapping sounds with proper polyphony
- Output a WAV file

## Implementation Notes

- The parser creates a Pattern<String> structure representing the pattern
- Patterns are functional - they're queried for events in a time range
- The voice manager handles up to 64 simultaneous sounds
- Samples are loaded from the dirt-samples library
- Time is measured in cycles (typically 1 cycle = 1 second at 120 BPM)