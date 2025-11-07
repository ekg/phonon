-- SuperSaw Demo: Thick Detuned Saw Stack
-- SuperSaw combines 7 sawtooth oscillators with progressive detuning
-- Syntax: supersaw freq detune
--   freq: Base frequency in Hz (pattern-controllable)
--   detune: Detune amount 0.0 to 1.0 (0.0 = unison, 1.0 = wide)

tempo: 1.5

-- Example 1: Classic trance supersaw (moderate detune)
~trance: supersaw 110 0.5

-- Example 2: Tight supersaw bass (low detune)
~bass: supersaw 55 0.2

-- Example 3: Wide supersaw lead (high detune)
~lead: supersaw 440 0.7

-- Example 4: Filtered supersaw pad
~pad: supersaw 165 0.6 # lpf 1200 0.5

-- Example 5: Pattern-controlled pitch
~melody: supersaw "110 165 220" 0.5

-- Mix all examples
out: (~trance + ~bass * 1.5 + ~lead * 0.5 + ~pad + ~melody) * 0.12
