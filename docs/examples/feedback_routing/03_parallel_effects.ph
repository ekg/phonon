-- Parallel Effects Routing Example
-- Split signal into parallel paths with different processing
-- Common in mixing: wet/dry parallel compression, parallel reverb, etc.

tempo: 0.5

-- Source signal
~source: saw 110 * 0.5

-- Path A: Low-pass filter + delay (warm, washy)
~path_a: ~source # lpf 1500 0.8 # delay 0.25 0.6

-- Path B: High-pass filter + delay (bright, crisp)
~path_b: ~source # hpf 500 0.8 # delay 0.33 0.5

-- Mix both paths together (balanced stereo-like effect)
out: ~path_a * 0.5 + ~path_b * 0.5

-- Try adjusting:
-- - filter cutoffs for different tonal balance
-- - delay times for rhythmic variations
-- - mix ratios for emphasis on one path
