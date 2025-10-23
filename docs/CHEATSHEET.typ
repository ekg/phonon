#set page(
  paper: "a4",
  margin: (x: 1.5cm, y: 1.5cm),
  columns: 2,
)
#set text(size: 9pt, font: "IBM Plex Sans")
#set par(justify: true, leading: 0.52em)
#show heading.where(level: 1): set text(size: 16pt, weight: "bold")
#show heading.where(level: 2): set text(size: 11pt, weight: "bold")
#show heading.where(level: 3): set text(size: 9pt, weight: "bold")
#show raw: set text(font: "IBM Plex Mono", size: 8.5pt)
#set list(marker: ([•], [‣]))

#align(center)[
  #text(size: 20pt, weight: "black")[PHONON]
  #v(-0.3em)
  #text(size: 11pt)[Live Coding Audio Language – Quick Reference]
  #v(0.2em)
  #text(size: 8pt, fill: gray)[Pattern-rate DSP • Tidal Cycles mini-notation • SuperDirt synths]
]

#v(0.5em)

= Basic Syntax

```phonon
-- Comment (Haskell-style)
tempo: 2.0          -- Cycles per second (120 BPM)
~bus: expression    -- Bus assignment
out: ~bus * 0.5     -- Output (required!)
```

*Key operators:* `#` chain (pipe), `$` transform, `+` add, `*` multiply

= Oscillators & Synthesis

```phonon
sine 440            -- Sine wave at 440 Hz
saw "55 82.5 110"   -- Pattern-controlled sawtooth
square 220          -- Square wave
tri 330             -- Triangle wave
noise 0             -- White noise
```

= SuperDirt Synths

```phonon
superkick 60 0.5 0.1 0.2      -- Analog kick
supersaw "55 82.5" 0.4 7      -- Detuned saw bass
superpwm 220 0.3 2.0          -- Pulse width mod
superchip 440 2.0 0.1         -- Chiptune square
superfm 220 1.5 0.8           -- FM synthesis
supersnare 180 0.6 0.15       -- Snare drum
superhat 0.5 0.05             -- Hi-hat
```

= Sample Playback

```phonon
s "bd sn hh cp"               -- Basic sequence
s "bd*4"                      -- Subdivision (repeat)
s "bd ~ sn ~"                 -- Rests (silence)
s "bd(3,8)"                   -- Euclidean rhythm
s "<bd sn hh>"                -- Alternation
s "[bd, hh*8]"                -- Layering
s "bd:0 bd:1"                 -- Sample selection
s "bd sn" 0.8 0.0 1.0         -- gain, pan, speed
```

= Effects

```phonon
~dry # lpf 2000 0.8           -- Low-pass filter
~dry # hpf 500 0.6            -- High-pass filter
~dry # reverb 0.7 0.5 0.3     -- room, damp, wet
~dry # delay 0.25 0.6 0.3     -- time, fb, wet
~dry # chorus 1.0 0.5 0.4     -- rate, depth, mix
~dry # distort 3.0 0.5        -- drive, mix
~dry # bitcrush 8.0 4.0       -- bits, rate
~dry # compress 4.0 0.1 0.05 0.3  -- ratio, thresh, attack, release
```

= Pattern Transforms

```phonon
~p $ fast 2         -- Double speed
~p $ slow 2         -- Half speed
~p $ rev            -- Reverse
~p $ every 4 rev    -- Apply every N cycles
~p $ degrade        -- Random dropout
```

= Signal Flow

```phonon
-- Buses (named signals)
~kick: s "bd*4"
~bass: saw 55
~lfo: sine 0.25

-- Chaining (#)
~chain: ~bass # lpf 1000 0.8 # distort 2.0 0.5

-- Math ops
~sum: ~kick + ~bass
~scaled: ~lfo * 2000 + 500
~mix: (~a + ~b) * 0.5

-- Pattern modulation (Phonon's superpower!)
~mod_filter: ~bass # lpf (~lfo * 2000 + 500) 0.8
```

= Mini-Notation Reference

#table(
  columns: (1fr, 2fr),
  [*Pattern*], [*Meaning*],
  [`bd sn`], [Sequence],
  [`bd*4`], [Repeat 4 times],
  [`bd ~ sn`], [Rest (silence)],
  [`bd(3,8)`], [Euclidean: 3 in 8 steps],
  [`<bd sn>`], [Alternate per cycle],
  [`[bd, hh*8]`], [Layer (polyrhythm)],
  [`bd:0 bd:1`], [Sample selection],
  [`[bd bd] sn`], [Group (fast bd bd)],
)

= Classic Patterns

```phonon
-- House (120 BPM)
tempo: 2.0
~drums: s "[bd*4, hh*8, ~ sn ~ sn]"

-- Techno (140 BPM)
tempo: 2.33
~drums: s "[bd(5,16), hh*16, ~ ~ sn ~]"

-- DnB (180 BPM)
tempo: 3.0
~drums: s "[bd*2 bd ~ bd, hh*32, ~ sn ~ sn]"
```

= Common Idioms

```phonon
-- LFO modulation
~lfo: sine 0.25
~bass: saw 55 # lpf (~lfo * 2000 + 500) 0.8

-- Pattern-controlled filter
~sweep: s "bd sn" # lpf "500 2000" 0.8

-- Layered drums with effects
~drums: s "[bd*4, hh*8, ~ sn ~ sn]"
~wet: ~drums # reverb 0.5 0.5 0.25

-- Dynamic gain patterns
~kick: s "bd*8" "1.0 0.7 0.9 0.6 1.0 0.7 0.9 0.6"

-- Stereo pan patterns
~hats: s "hh*8" 0.6 "-1 1 -0.5 0.5"
```

= Live Coding Workflow

+ *Start:* `phonon live track.ph`
+ *Edit* file in your editor
+ *Save* → auto-reload!
+ *Silence:* `out: 0`
+ *Stop:* Ctrl+C

= Euclidean Rhythms

Classic patterns from world music:

- `bd(3,8)` – Tresillo (Cuban)
- `bd(5,8)` – Cinquillo
- `bd(5,12)` – York-Samai
- `bd(7,16)` – West African bell

= Tips

+ Use `~buses` to name parts
+ Build gradually (start with kick)
+ Comment out with `--` to mute
+ Parameter patterns add dynamics
+ Chain effects with `#`
+ Pattern everything (freq, cutoff, gain!)

= Complete Example

```phonon
-- House track with LFO-modulated bass
tempo: 2.0

-- Drums
~kick: s "bd*4"
~hats: s "hh*8" 0.6
~snare: s "~ sn ~ sn"
~drums: ~kick + ~hats + ~snare

-- Bass with filter sweep
~lfo: sine 0.5 * 0.5 + 0.5
~bass: supersaw "55 55 82.5 55" 0.4 5
~bass_filt: ~bass # lpf (~lfo * 1500 + 400) 0.85

-- Pad
~pad: superfm 220 1.5 0.8 * 0.08

-- Mix
out: (~drums # reverb 0.5 0.5 0.2) * 0.7
     + ~bass_filt * 0.3
     + ~pad * 0.15
```

#v(1em)
#align(center)[
  #text(size: 7pt, fill: gray)[
    github.com/yourusername/phonon • Patterns are control signals • Audio-rate everything
  ]
]
