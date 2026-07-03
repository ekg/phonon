-- Chord progression (feat-chord-support)
--
-- Chords are written `root'quality` in mini-notation. A chord token expands
-- into a STACK of simultaneous notes that trigger polyphonic voices:
--
--   c'maj   -> C  E  G        (major triad,   intervals 0 4 7)
--   e'min7  -> E  G  B  D     (minor 7th,     intervals 0 3 7 10)
--   g'dom7  -> G  B  D  F     (dominant 7th,  intervals 0 4 7 10)
--
-- Qualities include: maj min dom7 maj7 min7 dim aug sus2 sus4 (and more).
--
-- Two surfaces:
--   * root'quality tokens:  note "c4'maj f4'maj g4'dom7 c4'maj"
--   * the `chord` modifier:  n "c e g c" # chord "maj"   (relative semitones)

tempo: 0.5

-- A ii-V-I-ish progression voiced through a sine "synth" whose base pitch is
-- C4 (261.63 Hz), so the voice manager repitches it to each chord tone.
~synth $ sine 261.63
~pads  $ s "~synth*4" # note "c4'maj a3'min7 d4'min7 g3'dom7"

-- A sparse arpeggio-free stab an octave up using richer maj7 / min7 colours.
~stab  $ s "~synth*2" # note "c5'maj7 ~ e4'min7 ~"

-- A relative-semitone harmony line built with the `chord` modifier: a major
-- triad stacked on each degree, transposed up to a root and sounded via mtof.
~triad $ n "0 5 7" # chord "maj"
~voice $ sine (mtof (~triad + 48)) * 0.10

out $ ~pads * 0.28 + ~stab * 0.18 + ~voice
