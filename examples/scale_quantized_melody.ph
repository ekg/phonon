-- Scale Quantization + Note Names (feat-scale-quantization)
--
-- `n "..."` gives raw scale-degree indices; `# scale "<name>"` remaps those
-- indices to semitone offsets for the named scale. The result is *relative*
-- semitones (degree 0 -> 0), so add a root (MIDI number) and run it through
-- `mtof` to sound the melody with a `sine` oscillator.
--
-- Note names work too: `note "c e g"` and `n "c4 e4 g4"` (see ~chime below).
--
-- Scales available include: major, minor, dorian, phrygian, lydian,
-- mixolydian, locrian, pentatonic, blues, harmonic, melodic, chromatic, ...

tempo: 0.5

-- A minor-scale melodic line.
--   degrees 0 2 4 7 5 4 2 0  ->  minor semitones  0 3 7 12 8 7 3 0
--   + 57 (A3)               ->  MIDI               57 60 64 69 65 64 60 57
~degrees $ n "0 2 4 7 5 4 2 0" # scale "minor"
~pitch   $ ~degrees + 57
~lead    $ sine (mtof ~pitch) * 0.25

-- A pentatonic counter-line an octave up, softer.
~penta   $ n "0 2 4 2" # scale "pentatonic"
~pitch2  $ ~penta + 69
~bells   $ sine (mtof ~pitch2) * 0.12

-- A little note-name chime using absolute pitches (c4 e4 g4 = C major triad).
~chime   $ sine (n "c4 e4 g4") * 0.10

out $ ~lead + ~bells + ~chime
