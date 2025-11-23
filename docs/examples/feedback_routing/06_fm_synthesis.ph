-- FM Synthesis Example
-- Frequency modulation creates complex harmonic spectra
-- Multiple modulators create rich, evolving timbres

tempo: 2.0

-- Slow LFO modulator (vibrato-like)
~lfo1: sine 3.0 * 50

-- Faster LFO modulator (adds shimmer)
~lfo2: sine 7.0 * 30

-- Carrier frequency modulated by both LFOs
~carrier: sine (~lfo1 + ~lfo2 + 440)

-- Output
out: ~carrier * 0.5

-- Try adjusting:
-- - modulator frequencies (3.0, 7.0 Hz) for different rates
-- - modulation depths (50, 30) for more/less harmonics
-- - carrier frequency (440) for different pitches
-- - use audio-rate modulators (e.g., sine 220) for classic FM

-- Advanced: Try cascading FM operators
-- ~mod1: sine 220
-- ~mod2: sine (~mod1 * 100 + 440)
-- ~carrier: sine (~mod2 * 50 + 880)
