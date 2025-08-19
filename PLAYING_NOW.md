# 🎵 NOW PLAYING: Techno Pattern in Strudel

## The Pattern Code
```javascript
stack(
  "bd*4",                                    
  "~ ~ ~ ~ cp ~ ~ ~ ~ ~ ~ ~ cp ~ [~ cp]",   
  "hh(7,16,2)",                              
  "~ ~ oh ~ ~ ~ oh ~ ~ ~ ~ ~ oh ~ ~ ~",     
  "<c2 c2 eb2 g2>",                          
  "<~ ~ ~ [c4 eb4] ~ [g4 bb4] ~ c5>*2",      
  "rs(3,8,1)",                               
  "~ ~ ~ ~ ~ ~ cb ~"                         
).slow(2)
```

## What You Would Hear

### Beat Structure (every 16th note):
```
Step 01: bd, c2, rs         // Kick + Bass + Rimshot
Step 02: hh                  // Hi-hat
Step 03: oh                  // Open hat  
Step 04: hh, rs              // Hi-hat + Rimshot
Step 05: bd, cp, c2          // Kick + Clap + Bass
Step 06: hh                  // Hi-hat
Step 07: oh, cb, rs, c4, eb4 // Open hat + Cowbell + Rim + Melody
Step 08: hh                  // Hi-hat
Step 09: bd, c2              // Kick + Bass
Step 10: hh                  // Hi-hat
Step 11: g4, bb4             // Lead melody
Step 12: hh                  // Hi-hat
Step 13: bd, cp, oh, g2      // Kick + Clap + Open + Bass
Step 14: hh                  // Hi-hat
Step 15: c5                  // High melody note
Step 16: hh, cp              // Hi-hat + Ghost clap
```

### Visual Grid:
```
        1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6
        ─────────────────────────────────
Kick    ● · · · ● · · · ● · · · ● · · ·
Clap    · · · · ● · · · · · · · ● · · ●
HiHat   · ● · ● · ● · ● · ● · ● · ● · ●
Open    · · ● · · · ● · · · · · ● · · ·
Bass    ● · · · ● · · · ● · · · ● · · ·
Lead    · · · · · · ● · · · ● · · · ● ·
Rim     · ● · ● · · ● · · · · · · · · ·
Cowbell · · · · · · ● · · · · · · · · ·
```

## Musical Analysis

- **Genre**: Techno/House hybrid
- **Tempo**: 120 BPM (slowed to 60 BPM feel)
- **Key**: C minor (C, Eb, G, Bb)
- **Rhythm**: Four-on-floor with syncopated percussion
- **Special Features**:
  - Euclidean rhythm on hi-hats (7,16,2) creates driving groove
  - Ghost notes on claps add variation
  - Acid-style lead melody
  - Polyrhythmic percussion layers

## How Phonon Plays This

1. **Boson** parses the Strudel pattern using `@strudel/mini`
2. **Pattern.queryArc()** returns events for each time slice
3. **OSC messages** sent to Fermion:
   - `/sample bd 0 1.0` for kicks
   - `/sample hh 0 1.0` for hi-hats
   - `/play 65.41 0.2` for C2 bass note
   - etc.
4. **Fermion** generates audio:
   - Synthesizes drums (or loads from Dirt-Samples)
   - Creates bass frequencies
   - Outputs WAV files
5. **mplayer** plays the audio through PulseAudio

## The Sound

This creates a hypnotic, driving techno groove with:
- Steady four-on-floor kick providing the foundation
- Syncopated hi-hats creating forward momentum
- Claps on the backbeat with a ghost note for variation
- Deep C minor bass progression
- Acid-style lead melody that builds tension
- Percussion accents (rimshot, cowbell) adding texture

The `.slow(2)` makes everything half-time, creating a heavy, deliberate feel perfect for late-night techno sets.