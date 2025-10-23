-- 4-on-the-floor House Beat
-- Using Tidal/Strudel mini-notation with euclidean rhythms!

-- Kick drum - euclidean 4 on the floor: bd(4,16)
-- This creates 4 hits evenly distributed across 16 steps
kick_gate = "1 0 0 0 1 0 0 0 1 0 0 0 1 0 0 0"
kick = sine 60 * kick_gate

-- Hi-hats - euclidean 13 hits in 16 steps, rotated by 4
-- Pattern: hh(13,16,4) - almost constant with interesting gaps
hat_gate = "0.8 0.8 0 0.8 0.8 0.8 0.8 0.8 0.8 0 0.8 0.8 0.8 0.8 0.8 0.8"
hats = square 8000 * hat_gate

-- Bass - pattern with rests and repetition
-- Original: [55 55 ~ 82.5]*2
bass_freq = "55 55 0 82.5 55 55 0 82.5"
bass = saw(bass_freq)

-- Mix all layers
out kick * 0.3 + hats * 0.04 + bass * 0.2
