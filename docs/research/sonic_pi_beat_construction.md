# Sonic Pi Beat Construction Research

## Overview

Sonic Pi is a Ruby-based live coding environment created by Sam Aaron at the University of Cambridge. This research analyzes its beat construction approach to identify patterns and techniques that could inform Phonon's development.

## Core Beat Construction Mechanisms

### 1. Live Loops - The Foundation

Sonic Pi's primary beat construction mechanism is the `live_loop`:

```ruby
live_loop :drums do
  sample :drum_heavy_kick
  sleep 1
  sample :drum_snare_hard
  sleep 1
end
```

**Key characteristics:**
- Endless repeating loops
- Named for identity (`:drums`, `:bass`, etc.)
- Must contain at least one `sleep` call
- Can be redefined on-the-fly during performance
- Multiple loops run concurrently and can sync with each other

### 2. Timing Model - Sleep-Based Sequential

Unlike Tidal's cycle-based approach, Sonic Pi uses **imperative sequential timing**:

```ruby
live_loop :beat do
  sample :bd_haus     # Play now
  sleep 0.5           # Wait 0.5 beats
  sample :sn_dolf     # Play after wait
  sleep 0.5           # Wait again
end
```

**Contrast with Phonon/Tidal:**
- Sonic Pi: Events are scheduled sequentially with explicit waits
- Tidal/Phonon: Events are positioned within a cycle, queried at render time

### 3. Rings and Tick - Cycling Through Values

Sonic Pi's ring system enables cycling through sequences:

```ruby
live_loop :arp do
  play (scale :e3, :minor_pentatonic).tick, release: 0.1
  sleep 0.125
end
```

**Ring creation methods:**
```ruby
(ring 52, 55, 59)           # Function syntax
[52, 55, 59].ring           # Convert from list
(range 0, 10, 2)            # Range with step
(bools 1, 0, 1, 0, 1, 0)    # Boolean pattern
(knit :e3, 2, :fs3, 1)      # Knit repeated values
(spread 3, 8)               # Euclidean distribution
```

**Tick behavior:**
- `.tick` returns current value AND advances counter
- `.look` returns current value without advancing
- Named ticks: `.tick(:foo)` for multiple independent counters

### 4. String-Based Drum Patterns

A popular community pattern uses strings as visual drum sequences:

```ruby
define :pattern do |pattern|
  return pattern.ring.tick == "x"
end

live_loop :kick do
  sample :drum_heavy_kick if pattern "x--x--x---x--x--"
  sleep 0.25
end
```

**Amplitude variation with numbers:**
```ruby
live_loop :kick do
  sample :drum_heavy_kick, amp: "600009900002".ring.tick.to_f / 4.5
  sleep 0.25
end
```

### 5. The Spread Function (Euclidean Rhythms)

```ruby
(spread 3, 8)  # => (ring true, false, false, true, false, false, true, false)
```

Creates boolean rings with Euclidean distribution - same algorithm Phonon uses with `bd(3,8)` syntax.

### 6. Synchronization Between Loops

```ruby
live_loop :foo do
  play :e4, release: 0.5
  sleep 0.5
end

live_loop :bar do
  sync :foo          # Wait for :foo to cue
  sample :bd_haus
  sleep 1
end
```

**Key functions:**
- `sync :loop_name` - Wait for loop's cue
- `cue :name` - Broadcast cue to waiting threads

### 7. Sample Manipulation

```ruby
sample :loop_amen, beat_stretch: 2        # Stretch to 2 beats
sample :loop_amen, rate: 2                # Play at double speed
sample :loop_amen, onset: 3               # Play 3rd onset slice
sample :loop_breakbeat, start: 0.5        # Start from middle
sample :bd_haus, amp: 0.8, pan: -0.5      # Volume and pan
```

### 8. Effects (FX)

```ruby
with_fx :slicer, phase: 0.25, mix: 1 do
  sample :loop_amen, beat_stretch: 2
end
```

## The Drum Machine Pattern

A complete step sequencer implementation:

```ruby
use_bpm 95

drum_kits = {
  acoustic: {hat: :drum_cymbal_closed, kick: :drum_bass_hard, snare: :drum_snare_hard}
}

current_drum_kit = drum_kits[:acoustic]

live_loop :pulse do
  sleep 4
end

define :run_pattern do |name, pattern|
  live_loop name do
    sync :pulse
    pattern.each do |p|
      sample current_drum_kit[name], amp: p/9.0
      sleep 0.25
    end
  end
end

# User programs these arrays (0-9 velocity, 16 steps)
hat   [5, 0, 5, 0, 5, 0, 5, 0, 5, 0, 5, 0, 5, 0, 5, 0]
kick  [9, 0, 9, 0, 0, 0, 0, 0, 9, 0, 0, 3, 0, 0, 0, 0]
snare [0, 0, 0, 0, 9, 0, 0, 2, 0, 1, 0, 0, 9, 0, 0, 1]
```

## Comparison: Sonic Pi vs Phonon

| Aspect | Sonic Pi | Phonon |
|--------|----------|--------|
| Language | Ruby | Custom DSL |
| Timing Model | Sequential/imperative | Cycle-based (Tidal) |
| Pattern Syntax | No mini-notation | Rich mini-notation |
| Euclidean | `spread(3,8)` function | `bd(3,8)` inline |
| Live Update | `live_loop` redefinition | Graph hot-swap |
| Effects | `with_fx` blocks | `#` chain operator |
| Synthesis | Built-in SuperCollider synths | fundsp + VST |

## Key Insights for Phonon

### What Sonic Pi Does Well

1. **Readability**: Ruby syntax is approachable for beginners
2. **Visual patterns**: String-based `"x--x--x-"` is intuitive
3. **Explicit timing**: `sleep` makes timing visible
4. **Named loops**: Easy to identify and modify specific parts
5. **Sync mechanism**: Clear coordination between parts

### What Phonon Does Better

1. **Mini-notation density**: `bd(3,8) [hh hh hh hh] <sn cp>` is more compact
2. **Pattern as signal**: Patterns modulate synthesis at sample rate
3. **Functional transformations**: `fast 2`, `rev`, `every 4 rev` chain naturally
4. **True polyrhythms**: Cycle-based model handles complex ratios elegantly

### Potential Inspirations

1. **Visual step patterns**: Consider supporting `"x--x--x-"` style notation
2. **Named sections**: Could add named pattern groups for easy modification
3. **Velocity arrays**: Numeric arrays for dynamics `[9,0,5,0,9,0,5,0]`
4. **Spread function**: Already have Euclidean, but `spread` name is more intuitive

## References

- [Sonic Pi Tutorial](https://sonic-pi.net/tutorial.html)
- [Coded Beats Tutorial](https://github.com/sonic-pi-net/sonic-pi/blob/dev/etc/doc/tutorial/A.03-coded-beats.md)
- [Sonic Pi Drum Machine Gist](https://gist.github.com/darinwilson/a3e5909db339838a67fe)
- [Mehackit Drum Beat Tutorial](https://sonic-pi.mehackit.org/exercises/en/02-make-a-song/02-drum-beat.html)
- [Rings Tutorial](https://sunderb.me/sonic-pi-docs-test/en/tutorial/08.4-Rings.html)
- [Ticking Tutorial](https://github.com/sonic-pi-net/sonic-pi/blob/dev/etc/doc/tutorial/09.4-Ticking.md)
- [Live Coding Comparison](https://www.soniare.net/blog/live-coding-systems-comparison)
