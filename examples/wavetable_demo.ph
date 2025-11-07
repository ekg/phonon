-- Wavetable Demo: Sine Wave Oscillator
-- Wavetable provides a sine wave oscillator with pattern-controllable frequency
-- Syntax: wavetable freq
--   freq: Frequency in Hz (pattern-controllable)

tempo: 1.5

-- Example 1: Bass tone (55 Hz)
~bass: wavetable 55

-- Example 2: Mid-range tone (220 Hz)
~mid: wavetable 220

-- Example 3: Lead tone (440 Hz)
~lead: wavetable 440

-- Example 4: Pattern-controlled melody
~melody: wavetable "110 165 220 330"

-- Example 5: Filtered pad
~pad: wavetable 110 # lpf 800 0.5

-- Mix all examples
out: (~bass + ~mid * 0.5 + ~lead * 0.3 + ~melody * 0.7 + ~pad) * 0.15
