# Phonon Genre Tutorials

Learn to create music in specific genres using Phonon. Each tutorial provides step-by-step instructions, complete code examples, and tips for authentic sound design.

## Available Tutorials

### Electronic Dance Music

| Genre | Tempo | Tutorial | Description |
|-------|-------|----------|-------------|
| **House** | 120-130 BPM | [HOUSE_MUSIC.md](./HOUSE_MUSIC.md) | Four-on-the-floor, funky basslines, chord stabs |
| **Techno** | 130-150 BPM | [TECHNO.md](./TECHNO.md) | Driving beats, acid lines, dark atmospheres |
| **Drum & Bass** | 160-180 BPM | [DRUM_AND_BASS.md](./DRUM_AND_BASS.md) | Two-step rhythms, heavy sub-bass, breakbeats |

### Hip-Hop & Urban

| Genre | Tempo | Tutorial | Description |
|-------|-------|----------|-------------|
| **Hip-Hop** | 80-160 BPM | [HIP_HOP.md](./HIP_HOP.md) | Boom bap, trap, lo-fi production techniques |

### Atmospheric

| Genre | Tempo | Tutorial | Description |
|-------|-------|----------|-------------|
| **Ambient** | Variable | [AMBIENT.md](./AMBIENT.md) | Drones, pads, downtempo, soundscapes |

## Quick Start

New to Phonon? Start here:

1. Read the [Quick Start Guide](../QUICK_START.md) first
2. Try the [House Music](./HOUSE_MUSIC.md) tutorial - it covers fundamentals
3. Explore other genres based on your interests

## Common Elements Across Genres

### Tempo (cps)
All tutorials use `cps` (cycles per second) for tempo:
```phonon
cps: 2.0    -- 120 BPM
cps: 2.17   -- 130 BPM
cps: 2.5    -- 150 BPM
cps: 2.83   -- 170 BPM
```

**Formula**: `cps = BPM / 60`

### Bus Syntax
```phonon
~drums $ s "bd sn"     -- Audio source bus
~lfo # sine 2          -- Modifier/control bus
out $ ~drums           -- Output
```

### Pattern Chaining
```phonon
~pattern $ s "bd sn" $ fast 2 $ every 4 rev
~pattern_fx $ ~pattern # reverb 0.5 0.8
```

### Mixing
```phonon
out $ ~drums * 0.8 + ~bass * 0.5 + ~synth * 0.3
```

## What Each Tutorial Covers

Every tutorial includes:

1. **Genre characteristics** - tempo, time signature, key elements
2. **Step-by-step beat building** - from basic to complex
3. **Sound design** - bass, synths, pads, effects
4. **Complete track example** - putting it all together
5. **Subgenre variations** - exploring related styles
6. **Production tips** - authentic techniques

## Genre Cross-Reference

Looking for specific techniques? Here's where to find them:

| Technique | Found In |
|-----------|----------|
| Four-on-the-floor | [House](./HOUSE_MUSIC.md), [Techno](./TECHNO.md) |
| Two-step rhythms | [Drum & Bass](./DRUM_AND_BASS.md) |
| Boom bap patterns | [Hip-Hop](./HIP_HOP.md) |
| Rolling hi-hats | [Hip-Hop](./HIP_HOP.md) (Trap section) |
| Acid basslines | [Techno](./TECHNO.md), [House](./HOUSE_MUSIC.md) |
| Reese bass | [Drum & Bass](./DRUM_AND_BASS.md) |
| Sub bass | [Drum & Bass](./DRUM_AND_BASS.md), [Hip-Hop](./HIP_HOP.md) |
| Drones & pads | [Ambient](./AMBIENT.md) |
| LFO modulation | All tutorials |
| Swing/shuffle | [House](./HOUSE_MUSIC.md), [Hip-Hop](./HIP_HOP.md) |
| Euclidean rhythms | [Techno](./TECHNO.md), [Drum & Bass](./DRUM_AND_BASS.md) |

## Additional Resources

- [Pattern Guide](../../PATTERN_GUIDE.md) - Mini-notation and pattern reference
- [Language Reference](../PHONON_LANGUAGE_REFERENCE.md) - Complete syntax documentation
- [Mini Notation Guide](../MINI_NOTATION_GUIDE.md) - Pattern syntax details
- [Pattern Transformations](../PATTERN_TRANSFORMATIONS.md) - All available transforms

## Contributing

Want to add a tutorial for another genre? Consider:
- Dub/Reggae
- Jazz/Neo-Soul
- IDM/Experimental
- Breakbeat
- Trance
- Dubstep

Each tutorial should follow the established format with step-by-step examples, variations, and production tips.

## Feedback

Found an error or have suggestions? Please open an issue on the Phonon repository.
