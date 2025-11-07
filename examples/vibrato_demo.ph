-- Vibrato Demo: Classic Pitch Modulation Effect
-- Vibrato modulates pitch using an LFO-driven delay line
-- Syntax: signal # vibrato rate depth
--   rate: LFO frequency in Hz (0.1 to 20.0)
--   depth: Modulation depth in semitones (0.0 to 2.0)

tempo: 1.5

-- Example 1: Classic vocal vibrato (5.5 Hz, subtle)
~vocal: sine 330 # vibrato 5.5 0.4

-- Example 2: Wide vibrato for expression
~expressive: sine 220 # vibrato 4.0 0.8

-- Example 3: Slow vibrato swell (pad/string effect)
~pad: saw 110 # lpf 800 0.4 # vibrato 2.0 0.6

-- Example 4: Fast vibrato (tremolo-like pitch warble)
~warble: square 440 # vibrato 12.0 0.5

-- Example 5: Pattern-modulated vibrato rate
~vib_rate: sine 0.3 * 2.0 + 5.0
~modulated: sine 440 # vibrato ~vib_rate 0.5

-- Mix all examples
out: (~vocal + ~expressive + ~pad + ~warble + ~modulated) * 0.2
