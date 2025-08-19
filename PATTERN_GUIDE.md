# 🎵 Phonon Pattern Guide

## Basic Syntax

### Samples
```javascript
"bd"        // Play kick drum sample
"bd:1"      // Play second kick drum sample  
"bd*4"      // Play kick 4 times in the cycle
"bd/2"      // Play kick over 2 cycles (half speed)
```

### Notes and Chords
```javascript
"c4 e4 g4"              // Play notes in sequence
"[c4,e4,g4]"            // Play chord (simultaneous notes)
"<c4 e4 g4>"            // Alternate between notes each cycle
"c4:0.5"                // Note with duration
```

### Rests and Patterns
```javascript
"~"                     // Rest (silence)
"bd ~ sd ~"             // Pattern with rests
"[bd cp] sd"            // Group patterns
"bd . . sd"             // Dots also work as rests
```

## Euclidean Rhythms

Euclidean rhythms distribute beats evenly across a pattern using the notation `(pulses,steps,rotation)`:

```javascript
"bd(3,8)"       // 3 kicks in 8 steps: x..x..x.
"hh(5,8)"       // 5 hats in 8 steps: x.xx.xx.
"sn(3,8,1)"     // 3 snares, rotated by 1: .x..x..x
```

### Common Euclidean Patterns:
- `(2,5)` = x..x. (Kpanlogo - Ghana)
- `(3,8)` = x..x..x. (Tresillo - Cuba)
- `(5,8)` = x.xx.xx. (Cinquillo - Cuba)
- `(5,12)` = x..x.x..x.x. (Venda - South Africa)
- `(7,16)` = x.x.x.x..x.x.x.. (Samba)

## Musical Scales & Chords

### Note Names
```javascript
// Notes: c, d, e, f, g, a, b
// Sharps: c#, d#, f#, g#, a#
// Flats: db, eb, gb, ab, bb
// Octaves: c2, c3, c4, c5

"c4 d4 e4 f4 g4 a4 b4 c5"  // C major scale
```

### Common Chord Progressions
```javascript
// C major - A minor - F major - G major (I-vi-IV-V)
"[c4,e4,g4] [a3,c4,e4] [f3,a3,c4] [g3,b3,d4]"

// Jazz ii-V-I in C
"[d4,f4,a4,c5] [g3,b3,d4,f4] [c4,e4,g4,b4]"
```

## Genre-Specific Patterns

### 🏠 House (120-130 BPM)
```javascript
// Classic four-on-floor
stack(
  "bd*4",                     // Kick every beat
  "~ cp ~ cp",                // Clap on 2 and 4
  "hh*8",                     // Steady hi-hats
  "~ oh ~ oh ~ oh ~ oh"       // Off-beat open hats
)

// Minimal house
"bd ~ ~ ~ bd ~ ~ ~ bd ~ ~ ~ bd ~ ~ ~, hh*16, <c2 ~ ~ g2>"
```

### 🎛️ Techno (130-140 BPM)
```javascript
// Driving techno
stack(
  "bd*4",                     // Four-on-floor
  "~ ~ cp ~",                 // Minimal clap
  "hh(7,16)",                 // Euclidean hi-hats
  "[~ rs]*2",                 // Syncopated rimshot
  "<c2 c2 eb2 g2>/2"         // Dark bassline
)

// Berlin techno
"bd bd bd bd, ~ ~ ~ cp, hh*16, <c2 ~ ~ ~>"
```

### 🎺 Jazz (100-180 BPM)
```javascript
// Jazz swing (with triplet feel)
stack(
  "bd ~ ~ bd ~ ~",            // Syncopated kick
  "~ ~ sn ~ ~ ~",             // Light snare
  "ride*6",                   // Ride cymbal swing
  "[c3,e3,g3,bb3] ~ [d3,f3,a3,c4] ~"  // Jazz chords
)

// Bebop pattern
"bd ~ [bd ~] bd, ~ sn ~ [~ sn], ride*8"
```

### 🎤 Hip Hop (80-100 BPM)
```javascript
// Boom bap
stack(
  "bd ~ ~ bd ~ ~ bd ~",       // Boom
  "~ ~ sn ~ ~ ~ ~ sn",        // Bap
  "hh*8",                     // Steady hats
  "~ ~ ~ oh"                  // Occasional open hat
)

// Trap (130-150 BPM, half-time feel)
"bd ~ ~ bd ~ bd bd ~, ~ sn ~ ~ ~ sn ~ ~, hh*16, 808:2"
```

### 🌍 Afrobeat (100-120 BPM)
```javascript
// Fela Kuti style
stack(
  "bd(5,8)",                  // Polyrhythmic kick
  "sn ~ sn sn ~ sn ~ sn",     // Syncopated snare
  "hh*16",                    // Constant hi-hats
  "conga(7,16)",              // Conga pattern
  "shaker*8"                  // Shaker layer
)
```

### 🇧🇷 Bossa Nova (120-130 BPM)
```javascript
// Classic bossa
stack(
  "bd ~ bd ~ ~ bd ~ bd",      // Bossa clave
  "~ ~ rs ~ rs ~ ~ rs",       // Rim clicks
  "hh*8",                     // Soft hi-hats
  "<c3 ~ g3 ~> <e3 ~ bb3 ~>"  // Smooth bass
)
```

### 🇯🇲 Reggae (60-90 BPM)
```javascript
// One drop
stack(
  "~ ~ ~ bd",                 // One drop on beat 3
  "~ ~ ~ sn",                 // Snare with kick
  "hh*4",                     // Steady hats
  "~ ~ [c3,e3,g3] ~"         // Skank chord
)
```

### 🎸 Rock (110-140 BPM)
```javascript
// Basic rock beat
stack(
  "bd ~ bd ~",                // Kick on 1 and 3
  "~ sn ~ sn",                // Snare on 2 and 4
  "hh*8",                     // Eighth note hi-hats
  "crash ~ ~ ~ ~ ~ ~ ~"       // Crash on 1
)

// Punk rock
"bd bd ~ bd bd ~ bd ~, ~ sn ~ sn, hh*16"
```

### 🎹 Drum & Bass (160-180 BPM)
```javascript
// Classic D&B
stack(
  "bd ~ ~ ~ ~ ~ bd ~",        // Syncopated kick
  "~ ~ ~ sn ~ ~ ~ sn",        // Off-beat snare
  "hh*16",                    // Fast hi-hats
  "sub ~ ~ sub ~ ~ ~ ~"       // Sub bass
)

// Jungle
"bd ~ bd ~ ~ ~ bd ~, ~ sn ~ sn, amen*2"
```

### 🌊 Ambient (60-90 BPM)
```javascript
// Minimal ambient
stack(
  "bd/4",                     // Sparse kick
  "<c3 e3 g3 c4>/2",         // Slow arpeggios
  "~ ~ ~ hh/8"               // Occasional hat
).slow(4)
```

## Advanced Techniques

### Polyrhythms
```javascript
// 3 against 4
stack(
  "bd*3",                     // 3 beats
  "sn*4"                      // 4 beats
)
```

### Pattern Manipulation
```javascript
"bd sn".fast(2)              // Double speed
"bd sn".slow(2)              // Half speed
"bd sn".rev()                // Reverse pattern
"bd sn".palindrome()         // Forward then backward
```

### Probability
```javascript
"bd?0.5"                     // 50% chance of playing
"bd sn?0.8 hh"              // 80% chance for snare
```

### Speed Control
```javascript
"bd*2"                       // Play twice as fast
"bd/2"                       // Play half speed
"bd(3,8)*2"                 // Speed up Euclidean
```

## Tips for Live Coding

1. **Start simple**: Begin with a basic beat and add complexity
2. **Use comments**: Mark sections for easy navigation
3. **Layer gradually**: Add one element at a time
4. **Save variations**: Keep multiple patterns commented out
5. **Use Euclidean rhythms**: Quick way to get interesting patterns
6. **Combine genres**: Mix elements from different styles

## Example: Building a Track

```javascript
// Start with kick
"bd*4"

// Add clap
"bd*4, ~ cp ~ cp"

// Add hi-hats with variation
"bd*4, ~ cp ~ cp, hh(7,16)"

// Add bassline
"bd*4, ~ cp ~ cp, hh(7,16), <c2 ~ eb2 g2>"

// Add melody
"bd*4, ~ cp ~ cp, hh(7,16), <c2 ~ eb2 g2>, ~ ~ <c4 eb4> ~"

// Full arrangement with breaks
stack(
  "bd*4",
  "~ cp ~ cp",
  "hh(7,16)",
  "<c2 ~ eb2 g2>",
  "~ ~ <[c4,eb4,g4] [d4,f4,ab4]> ~"
).slow(2)
```