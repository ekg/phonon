-- Soft Sawtooth Oscillator Demo
-- Demonstrates softer saw wave with fewer harmonics than regular saw

tempo: 0.5

-- Basic soft saw bass
~bass: soft_saw_hz 55 * 0.3

-- Pattern-modulated soft saw melody
~melody: soft_saw_hz "220 330 440 330" * 0.2

-- LFO-modulated soft saw pad
~lfo: sine 0.25
~pad: soft_saw_hz (~lfo * 100 + 220) * 0.15

-- Compare soft_saw vs regular saw
~soft: soft_saw_hz 110 * 0.15
~regular: saw_hz 220 * 0.15
~comparison: ~soft + ~regular

-- Filtered soft saw
~filtered: soft_saw_hz 110 * 0.2 # lpf 800 0.7

-- Detuned soft saws (chorus-like effect)
~detune1: soft_saw_hz 165 * 0.1
~detune2: soft_saw_hz 167 * 0.1
~detune3: soft_saw_hz 163 * 0.1
~detuned: ~detune1 + ~detune2 + ~detune3

-- Output: Choose one by uncommenting
out: ~bass
-- out: ~melody
-- out: ~pad
-- out: ~comparison
-- out: ~filtered
-- out: ~detuned
