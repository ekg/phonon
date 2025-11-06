tempo: 4.0
-- Test voice pool shrinking with short samples:
-- First 2 cycles: trigger 64 voices each (will grow pool)
-- Next 8 cycles: complete silence (allow voices to finish and pool to shrink)
-- Last 2 cycles: trigger only 4 voices (should use shrunk pool)
out: s "[hh*64 hh*64] ~ ~ ~ ~ ~ ~ ~ ~ ~ [hh*4 hh*4]"
