# Four-on-the-Floor House/Techno Pattern Research

## Summary

Four-on-the-floor is the foundational rhythm for house and techno music, characterized by a bass drum hitting on every beat of a 4/4 bar. This research documents the patterns, variations, and Phonon implementations.

## Core Definition

**Four-on-the-floor**: A steady, uniformly accented beat where the bass drum hits on every quarter note (beats 1, 2, 3, 4). Originated in disco (1970s), became foundational for house (Chicago, 1980s) and techno (Detroit, 1980s).

## Typical BPM Ranges

| Genre | BPM Range | Phonon CPS |
|-------|-----------|------------|
| Deep House | 118-125 | 1.97-2.08 |
| House | 120-130 | 2.0-2.17 |
| Tech House | 124-130 | 2.07-2.17 |
| Techno | 130-145 | 2.17-2.42 |
| Industrial Techno | 140-150 | 2.33-2.5 |

## Standard 16-Step Notation

For a typical house pattern (16 steps = 1 bar):

```
Step:    1  2  3  4  5  6  7  8  9 10 11 12 13 14 15 16
Beat:    1        2        3        4
------------------------------------------------------
Kick:    X  -  -  -  X  -  -  -  X  -  -  -  X  -  -  -
Clap:    -  -  -  -  X  -  -  -  -  -  -  -  X  -  -  -
CH:      -  X  -  X  -  X  -  X  -  X  -  X  -  X  -  X
OH:      -  -  X  -  -  -  X  -  -  -  X  -  -  -  X  -
```

## Phonon Pattern Translations

### Basic Four-on-the-Floor
```phonon
-- Kick on every beat
s "bd*4"

-- Or explicitly
s "bd ~ ~ ~ bd ~ ~ ~ bd ~ ~ ~ bd ~ ~ ~"
```

### Classic House Beat
```phonon
cps: 2.0

~kick $ s "bd*4"                    -- Four-on-the-floor
~clap $ s "~ cp ~ cp"               -- Claps on 2 and 4
~hats $ s "hh*8"                    -- Eighth note hi-hats
~oh   $ s "~ oh ~ oh ~ oh ~ oh"     -- Off-beat open hats

out $ ~kick + ~clap + ~hats + ~oh
```

### Minimal Techno
```phonon
cps: 2.17

~kick $ s "bd*4"
~clap $ s "~ ~ cp ~"                -- Sparse clap (beat 3 only)
~hats $ s "hh(7,16)"                -- Euclidean distribution
~rim  $ s "[~ rim]*2"               -- Off-beat rimshots

out $ ~kick + ~clap + ~hats * 0.5 + ~rim * 0.5
```

### Deep House
```phonon
cps: 1.97

~kick $ s "bd*4"
~clap $ s "~ cp ~ ~" # reverb 0.5 0.85
~hats $ s "hh*8" # gain 0.4
~rim  $ s "~ ~ ~ rim" # gain 0.3

out $ ~kick + ~clap + ~hats + ~rim
```

## Essential Variations

### 1. Double Kick (End-of-Phrase)
Common in tech house to mark phrase endings:
```phonon
-- Last bar of 8-bar phrase
s "bd*4"           -- Normal bar
s "bd bd bd [bd bd]" -- Bar with double kick before phrase
```

### 2. Offbeat Bass Pattern
House often has syncopated basslines that complement the four-on-the-floor:
```phonon
~kick $ s "bd*4"
~bass $ saw "55 ~ 55 82.5 ~ 55 ~ 110"  -- Syncopated
```

### 3. Swing/Shuffle
Adds human feel to hi-hats:
```phonon
~hats $ s "hh*16" $ swing 0.08
```

### 4. Ghost Notes
Subtle velocity variations on hi-hats:
```phonon
~hats $ s "hh*16" # gain "0.3 0.5 0.4 0.6 0.3 0.5 0.4 0.7"
```

### 5. Euclidean Rhythms for Techno
More interesting than straight patterns:
```phonon
~hats $ s "hh(5,8)"     -- 5 hits in 8 steps
~hats $ s "hh(7,16)"    -- 7 hits in 16 steps
~hats $ s "hh(9,16)"    -- 9 hits in 16 steps (busier)
```

### 6. Rolling Hi-Hats (Velocity Cycling)
3 or 5 step loops create rolling feel:
```phonon
-- 3-step pattern repeating over 16 steps creates polyrhythm
~hats $ s "hh*16" # gain "0.7 0.4 0.5"
```

## Drum Machine Sounds

### TR-909 (House/Techno Standard)
The iconic sound of house and techno. In Phonon:
- `bd` - Kick drums
- `sn` - Snares
- `cp` - Claps
- `hh` - Closed hi-hats
- `oh` or `808oh` - Open hi-hats

### TR-808 (Deeper/Harder Sounds)
- `808bd` - Heavy kick
- `808sd` - Snare
- `808hc` - Hi-hat closed
- `808oh` - Hi-hat open
- `808lt/808mt/808ht` - Toms

### Sample Bank Selection
Use `:N` syntax to select specific samples:
```phonon
s "bd:0 bd:1 bd:2 bd:3"  -- Cycle through kick variations
s "hh:0 hh:2 hh:4"       -- Different hi-hat characters
```

## Genre-Specific Patterns

### Chicago House
Funky, swinging, sample-heavy:
```phonon
cps: 2.0

~kick $ s "bd*4"
~clap $ s "~ cp ~ cp"
~hats $ s "hh*16" $ swing 0.12 # gain "0.4 0.6 0.5 0.7"
~oh   $ s "~ oh ~ oh ~ oh ~ oh" # gain 0.5

out $ ~kick + ~clap + ~hats + ~oh
```

### Detroit Techno
More melodic, futuristic:
```phonon
cps: 2.1

~kick $ s "bd*4"
~clap $ s "~ cp ~ cp" # reverb 0.4 0.7
~hats $ s "oh(3,8)" # gain 0.4
~rim  $ s "rim(5,16)" # gain 0.4

out $ ~kick + ~clap + ~hats + ~rim
```

### Berlin Techno
Raw, industrial, heavy:
```phonon
cps: 2.25

~kick $ s "bd:3*4" # distortion 0.5
~clap $ s "~ ~ cp ~" # reverb 0.15 0.3
~hats $ s "hh(3,8)" # gain 0.5

out $ ~kick + ~clap + ~hats
```

### Dub Techno
Spacious, delay-heavy:
```phonon
cps: 2.0

~kick $ s "bd*4" # reverb 0.1 0.3
~rim  $ s "~ rim ~ ~" # delay 0.375 0.6 0.4 # reverb 0.5 0.8
~hats $ s "hh(5,16)" # gain 0.35 # delay 0.25 0.4 0.3

out $ ~kick + ~rim + ~hats
```

### Acid House
303-style squelchy bass:
```phonon
cps: 2.0

~kick $ s "bd*4"
~hats $ s "hh*16" # gain 0.5

~bass   $ saw "55 55 110 55 82.5 55 110 55"
~accent # "1 0.5 1 0.7 1 0.5 0.8 1"
~acid   $ ~bass # lpf (~accent * 2000 + 200) 3.5 # distortion 1.5

out $ ~kick + ~hats + ~acid * 0.25
```

## Existing Phonon Resources

The following documentation already covers four-on-the-floor extensively:

1. **docs/tutorials/HOUSE_MUSIC.md** - Complete 10-step house production tutorial
2. **docs/tutorials/TECHNO.md** - Complete techno production tutorial with variations
3. **PATTERN_GUIDE.md** - Genre-specific patterns including house/techno sections

## Features Phonon Already Supports

- `bd*4` - Four-on-the-floor notation
- `swing` - Timing humanization
- `fast`, `slow` - Time scaling
- `every N (transform)` - Periodic variation
- Euclidean rhythms `(k,n)` - Polyrhythmic patterns
- LFO modulation for filter sweeps
- Sample bank selection with `:N`
- Effect chaining: `# reverb`, `# delay`, `# lpf`
- Bus mixing with `~name $ pattern`

## Potential Enhancements (Future Work)

1. **`stutter` / `chop`** - For build-ups and fills
2. **`loopAt`** - Sync sample length to cycle
3. **Velocity patterns as first-class** - `# velocity "1 0.5 0.8 0.6"`
4. **Probability/chance** - `degradeBy 0.2` for random dropout

## References

- [MusicRadar: 6 Four-to-the-Floor Grooves](https://www.musicradar.com/how-to/how-to-program-6-different-four-to-the-floor-grooves)
- [LANDR: 17 Essential Electronic Drum Patterns](https://blog.landr.com/drum-programming/)
- [Studio Brootle: Techno Drum Patterns](https://www.studiobrootle.com/techno-drum-patterns-and-drum-programming-tips/)
- [MasterClass: Four-on-the-Floor Explained](https://www.masterclass.com/articles/four-on-the-floor-rhythm-explained)
- [Tidal Cycles Mini Notation](https://tidalcycles.org/docs/reference/mini_notation/)
- [Wikipedia: Four on the Floor](https://en.wikipedia.org/wiki/Four_on_the_floor_(music))
