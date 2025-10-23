-- SuperSaw Bass Demo
-- Demonstrates thick, rich bass using detuned saw waves

tempo: 2.0

-- Deep bass with maximum detuning and 7 voices
~bass: supersaw 55 0.8 7

-- Apply low-pass filter for warmth
~filtered: ~bass # lpf 800 1.2

-- Add subtle distortion for character
~distorted: ~filtered # distortion 1.5 0.2

-- Output with moderate gain
out: ~distorted * 0.4
