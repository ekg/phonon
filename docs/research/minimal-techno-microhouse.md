# Minimal Techno & Microhouse Research

*Research for Phonon live-coding language - January 2026*

## Overview

Minimal techno and microhouse are closely related electronic music genres characterized by stripped-down aesthetics, repetition, and subtle variation. These genres are highly relevant to Phonon because they emphasize:

1. **Pattern-based composition** - Perfect for Phonon's pattern-first approach
2. **Subtle parameter modulation** - Aligns with Phonon's unique pattern-as-control-signal capability
3. **Real-time manipulation** - Ideal for live coding

---

## Minimal Techno

### History & Origins

- Developed in the early 1990s by Detroit-based producers **Robert Hood** and **Daniel Bell**
- Robert Hood defined it as: *"a basic stripped down, raw sound. Just drums, basslines and funky grooves and only what's essential"*
- Evolved through the mid-1990s with pioneers like Richie Hawtin (Plastikman)
- By the 2000s, merged significantly with microhouse

### Key Characteristics

| Attribute | Details |
|-----------|---------|
| **Tempo** | 125-137 BPM (typically ~130 BPM) |
| **Time Signature** | 4/4 |
| **Instrumentation** | Minimal - drum machine, looping analog synth, sparse hats |
| **Track Length** | 5-8+ minutes (average ~6 min) |
| **Structure** | Long-form tension/release, subtle evolution |

### Musical Elements

- **Kick**: 4-on-the-floor, often with "techno rumble" sub bass
- **Hi-hats**: Sparse, often with swing/shuffle (35%+ shuffle common)
- **Snare/Clap**: Backbeat on 2 and 4
- **Bassline**: Repetitive, hypnotic, often filtered
- **Percussion**: Minimal, carefully placed micro-edits

### Production Techniques

1. **Microsampling**: Tiny fragments looped and layered for complexity
2. **Side-chaining/Compression**: Pulsing, breathing dynamics
3. **Surgical EQ**: Precise frequency sculpting
4. **Dynamic envelope play**: Subtle automation over time
5. **Razor edits**: Micro-fragments instead of traditional drum kits

### Notable Artists

- **Robert Hood** - Pioneer, defined the stripped-down aesthetic
- **Richie Hawtin** (Plastikman, Minus label) - Technological innovator
- **Ricardo Villalobos** - Deep, hypnotic, extended tracks
- **Daniel Bell** - Detroit second wave
- **Thomas Brinkmann** - Rhythmic/textural minimalism

### Key Labels

- Minus (Richie Hawtin)
- Perlon (Ricardo Villalobos)
- Tresor
- Ostgut Ton

---

## Microhouse

### History & Origins

- Term coined by Philip Sherburne in 2001 (The Wire magazine)
- Roots in: minimal techno (1990s), bitpop (1990s), house (1980s)
- German scene crystallized around labels like Kompakt, Perlon, Playhouse
- "Clicks & Cuts" compilations (Mille Plateaux, 1999-2001) codified the aesthetic

### Key Characteristics

| Attribute | Details |
|-----------|---------|
| **Tempo** | 120-128 BPM (typically ~125 BPM) |
| **Time Signature** | 4/4 |
| **Sound Design** | Clicks, pops, glitchy textures, micro-samples |
| **Vocals** | Simplistic, monotone, often nonsensical fragments |
| **Emotional Quality** | Deeper, more melodic than minimal techno |

### The "Micro" in Microhouse

- Extremely short samples (<50ms, often 10-30ms "grains")
- Sources: human voice, instruments, everyday noises, computer-generated waves
- Equipment malfunctions (scratching, hissing, clicking, distortion) used musically
- Creates complex melodies from tiny fragments

### Production Techniques

1. **Granular Synthesis**: Slicing samples into 1-100ms grains
2. **Deliberate Glitches**: Damaged CDs, mismatched bitrates, overdriven inputs
3. **Intentional Silences**: Negative space creates tension
4. **Click/Pop Libraries**: Building personal collections of micro-transients
5. **Hanning Windows**: Envelope grains to avoid unwanted clicks

### Notable Artists

- **Akufen** - Live clicking/cutting pioneer
- **Luomo** - Sensual microhouse
- **Isolée** - Playhouse sound
- **Ricardo Villalobos** - Crossover minimal/microhouse
- **Jan Jelinek** - Minimal/glitch fusion
- **Oval** (Markus Popp) - Deliberately damaged CD pioneer
- **Pole** - Minimal dub techno

### Key Labels

- Kompakt (Cologne)
- Perlon
- Playhouse
- Klang Elektronik
- Force Tracks
- Trapez

---

## Track Structure

### Section Lengths (All Divisible by 8)

| Section | Typical Length |
|---------|----------------|
| Intro | 16-32 bars |
| Build | 16-32 bars |
| Break/Breakdown | 8-32 bars |
| Drop/Main | 32-64 bars |
| Outro | 16-32 bars |

### Arrangement Approach: Subtraction Method

1. Create main loops covering 4-6 minutes
2. Run all elements simultaneously
3. Gradually mute/remove elements
4. Each section should feel purposeful
5. Use contrast: stripped-back vs full sections

### Breakdowns

- Drums (and usually bass) drop out completely
- Atmospheric textures, pads, effects remain
- Creates tension and resets the listener's ear
- Leads to rebuilding intensity

---

## Swing, Shuffle & Groove

### What is Swing?

- Slight delay applied to every other subdivision
- Displaces off-beats by a fraction
- Creates organic, non-robotic feel

### Swing Percentages

| Setting | Result |
|---------|--------|
| 50% | Straight timing (no swing) |
| 57% | Septuplet swing |
| 60% | Quintuplet swing |
| 66.7% | Triplet shuffle |

### Minimal Techno Swing

- Minimal techno *started* the trend of using swing in techno
- 909 shuffle and 808 16 triplets were common early on
- Microhouse patterns typically use 35%+ shuffle
- Even subtle humanization (<25ms deviation) adds life

### Humanization Techniques

1. **Random timing deviations**: ±5-25ms per note
2. **Velocity variation**: ±10-30% on hi-hats/percussion
3. **Groove templates**: Apply swing + velocity curves
4. **Micro-timing**: Shift hats slightly behind/ahead

---

## Modulation & Sound Design

### LFO Applications

| LFO Shape | Effect on Filter |
|-----------|------------------|
| Sine | Smooth, gradual shifts |
| Triangle | Balanced, evolving textures |
| Square | Rhythmic dropouts, gating |
| Sample & Hold | Random, stepped modulation |

### Key Modulation Targets

1. **Filter Cutoff** - Classic wobble/sweep
2. **Filter Resonance** - Emphasis variation
3. **Amplitude** - Tremolo, gating
4. **Pan** - Stereo movement
5. **LFO Rate** - LFO-to-LFO modulation

### Tempo-Synced Modulation

- Lock LFO to musical divisions (1 bar, 1/2 bar, etc.)
- Phase offset for precise beat placement
- Creates rhythmic effects tied to groove

---

## Relevance to Phonon

### Why This Matters for Phonon

Phonon's unique capability is that **patterns ARE control signals** - evaluated at sample rate. This makes it uniquely suited for minimal techno and microhouse because:

1. **Real-time filter modulation**:
```phonon
~lfo $ sine 0.25
out $ saw 55 # lpf (~lfo * 2000 + 500) 0.8
```

2. **Swing via pattern functions**:
```phonon
-- Phonon has swing built-in
~drums $ s "bd sn hh cp" $ swing 0.1
```

3. **Micro-timing with pattern transforms**:
```phonon
-- Ghost notes for groove
~hits $ s "bd sn" $ ghost
```

4. **Stepped parameter patterns** (microhouse clicks):
```phonon
~clicks # "0 1 0 0 1 0 1 1"  -- trigger pattern
~cutoff # "500 800 1200 600"  -- stepped filter
```

### Features Phonon Already Has

| Genre Need | Phonon Feature |
|------------|----------------|
| 4/4 kick patterns | `s "bd*4"` |
| Swing/shuffle | `swing` function |
| Pattern transforms | `fast`, `slow`, `every`, `rotL/R` |
| LFO modulation | `sine`, `saw` as patterns |
| Filter control | `lpf`, `hpf` with pattern params |
| Sample triggering | Voice-based playback |
| Ghost notes | `ghost` / `ghostWith` |

### Features That Would Enhance Minimal Techno

1. **Granular synthesis** - For microhouse textures
2. **White noise** - For hats, risers
3. **Probability patterns** - For humanization
4. **Delay/reverb** - For space and depth
5. **Sidechain compression** - For pumping dynamics

---

## Example Phonon Patterns for Minimal Techno

### Basic 4/4 Pattern
```phonon
cps: 2.166  -- ~130 BPM

~kick $ s "bd*4"
~hat $ s "~ hh ~ hh"
~snare $ s "~ sn"

out $ ~kick * 0.8 + ~hat * 0.3 + ~snare * 0.5
```

### With Swing
```phonon
cps: 2.166

~kick $ s "bd*4"
~hat $ s "hh*8" $ swing 0.08
~perc $ s "cp ~ ~ rim" $ swing 0.1

out $ ~kick * 0.8 + ~hat * 0.2 + ~perc * 0.4
```

### Filter Sweep (Hypnotic)
```phonon
cps: 2.166

~bass $ saw 55
~lfo $ sine 0.0625  -- 16-bar cycle
~filtered $ ~bass # lpf (~lfo * 3000 + 200) 0.7

~kick $ s "bd*4"

out $ ~kick * 0.6 + ~filtered * 0.4
```

### Evolving Pattern
```phonon
cps: 2.166

~drums $ s "bd ~ sn bd ~ sn bd ~"
~evolve $ ~drums $ every 4 (fast 2)

out $ ~evolve * 0.7
```

---

## Sources

- [Minimal Techno - Wikipedia](https://en.wikipedia.org/wiki/Minimal_techno)
- [Microhouse - Wikipedia](https://en.wikipedia.org/wiki/Microhouse)
- [The History of Minimal Tech - Samplesound](https://www.samplesoundmusic.com/blogs/news/the-history-of-minimal-tech-from-microhouse-to-now)
- [Glitch - Melodigging](https://www.melodigging.com/genre/glitch)
- [Granular Synthesis - Sound on Sound](https://www.soundonsound.com/techniques/granular-synthesis)
- [Native Instruments - What is an LFO?](https://blog.native-instruments.com/what-is-an-lfo/)
- [Native Instruments - Swing in Music](https://blog.native-instruments.com/swing-in-music/)
- [Techno Song Structure - Mastrng](https://www.mastrng.com/song-structure-arrangement/)
- [Studio Brootle - Techno Drum Patterns](https://www.studiobrootle.com/techno-drum-patterns-and-drum-programming-tips/)
- [Swing, Shuffle, and Humanization - Sample Focus](https://blog.samplefocus.com/blog/swing-shuffle-and-humanization-how-to-program-grooves/)
