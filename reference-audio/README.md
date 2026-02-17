# Phonon Golden Reference Audio

This directory contains golden reference audio files for validating Phonon's pattern output. Each genre has multiple representative patterns with canonical characteristics.

## Purpose

These reference files serve as:
1. **Validation targets** - Ensure rendered patterns match expected audio characteristics
2. **Regression testing** - Detect if pattern engine changes break output
3. **Genre documentation** - Demonstrate canonical rhythm patterns for each genre
4. **Audio comparison baselines** - For A/B testing and similarity scoring

## Directory Structure

```
reference-audio/
├── house-techno/           # 125-128 BPM, four-on-the-floor
│   ├── four-on-floor-basic.phonon
│   ├── four-on-floor-16th-hats.phonon
│   └── tech-house-groove.phonon
├── dnb/                    # 170-175 BPM, two-step patterns
│   ├── two-step-basic.phonon
│   ├── amen-style.phonon
│   └── half-time-liquid.phonon
├── breakbeat/              # 130-160 BPM, syncopated
│   ├── classic-break.phonon
│   ├── nu-skool.phonon
│   └── jungle-influenced.phonon
├── hip-hop-trap/           # 85-140 BPM, boom bap to trap
│   ├── boom-bap.phonon
│   ├── trap-808.phonon
│   └── modern-hip-hop.phonon
├── uk-garage/              # 130-138 BPM, 2-step shuffle
│   ├── 2-step-classic.phonon
│   ├── speed-garage.phonon
│   └── bassline-ukg.phonon
├── dub-reggae/             # 68-75 BPM, one drop / steppers
│   ├── one-drop.phonon
│   ├── steppers.phonon
│   └── roots-dub.phonon
├── ambient-idm/            # 80-110 BPM, irregular textures
│   ├── sparse-texture.phonon
│   ├── glitch-rhythm.phonon
│   └── polyrhythmic.phonon
└── minimal-techno/         # 120-125 BPM, stripped down
    ├── basic-minimal.phonon
    ├── microhouse.phonon
    └── dub-techno.phonon
```

## Genre Characteristics

### House/Techno (125-128 BPM)
- Four-on-the-floor kick drum
- Offbeat hi-hats
- Clap/snare on 2 and 4
- 16th note hi-hat variations

### Drum & Bass (170-175 BPM)
- Two-step pattern (kick-snare-kick-snare)
- Syncopated kick variations
- Fast hi-hats
- Amen break influences

### Breakbeat (130-160 BPM)
- Syncopated kick patterns
- Funk-derived rhythms
- Ghost notes
- Jungle influences at higher tempos

### Hip-hop/Trap (85-140 BPM)
- Boom bap: slower, soulful samples
- Trap: hi-hat rolls, 808 kicks
- Snare on 2 and 4

### UK Garage (130-138 BPM)
- 2-step shuffle (skipping downbeat)
- Shuffled hi-hats
- Speed garage with four-on-floor

### Dub/Reggae (68-75 BPM)
- One drop (drop the 1, accent on 3)
- Offbeat hi-hats (ska influence)
- Steppers (four-on-floor reggae)
- Space for delay/echo

### Ambient/IDM (80-110 BPM)
- Irregular timing
- Euclidean rhythms
- Sparse textures
- Glitch/polyrhythmic elements

### Minimal Techno (120-125 BPM)
- Stripped down patterns
- Microhouse clicks
- Dub techno space
- Hypnotic repetition

## Rendering Reference Audio

Use the included script to render all patterns:

```bash
./render-all.sh
```

Or render individual patterns:

```bash
phonon render house-techno/four-on-floor-basic.phonon house-techno/four-on-floor-basic.wav -d 8.0
```

## Audio Characteristics

Each rendered WAV file includes:
- 8 cycles of the pattern (varies by tempo)
- 44100 Hz sample rate
- Mono output
- Normalized levels (-20dB target RMS)

## Validation Tests

Reference audio can be used with the pattern verification utils:

```rust
use pattern_verification_utils::{detect_audio_events, compare_events};

// Load reference audio
let reference = load_wav("house-techno/four-on-floor-basic.wav");
let reference_events = detect_audio_events(&reference, 44100.0, 0.01);

// Render pattern under test
let test = render_pattern(code);
let test_events = detect_audio_events(&test, 44100.0, 0.01);

// Compare
let comparison = compare_events(&reference_events, &test_events, 0.05);
assert!(comparison.match_rate > 0.95, "Events should match reference");
```

## Adding New Reference Patterns

1. Create `.phonon` file in appropriate genre directory
2. Include comment header with:
   - Genre and style name
   - BPM/tempo (CPS)
   - Key characteristics
3. Run `./render-all.sh` to generate WAV
4. Verify audio manually before committing

## Tempo to CPS Conversion

```
CPS = BPM / 60
```

| BPM | CPS |
|-----|-----|
| 70  | 1.17 |
| 90  | 1.5  |
| 120 | 2.0  |
| 128 | 2.13 |
| 140 | 2.33 |
| 170 | 2.83 |
