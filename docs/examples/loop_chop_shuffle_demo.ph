-- Loop Chopping and Shuffling Demo
-- Demonstrates how to take audio loops and creatively re-arrange them

tempo: 0.5

-- BASIC WORKFLOW: Loop → Chop → Shuffle/Scramble
-- ================================================

-- Example 1: Chop a drum pattern into 8 pieces and scramble them
-- This creates a randomized version of the original pattern
~chopped: s "bd sn hh cp" $ chop 8 $ scramble 8
out1: ~chopped # gain 0.7

-- Example 2: Chop into 16 pieces for more granular chopping
-- ~finely_chopped: s "bd*4" $ chop 16 $ scramble 16
-- out2: ~finely_chopped # gain 0.6

-- Example 3: Use shuffle instead of scramble
-- shuffle shifts events in time (creates timing variations)
-- scramble reorders events (creates sequence variations)
-- ~time_shuffled: s "bd sn hh*2 cp" $ chop 8 $ shuffle 0.2
-- out3: ~time_shuffled

-- COMBINING WITH EFFECTS
-- ================================================

-- Example 4: Chop, scramble, then add filter
-- ~chopped_filtered: s "bd sn hh cp" $ chop 16 $ scramble 16 # lpf 2000 0.8
-- out4: ~chopped_filtered

-- Example 5: Chop with reverb for atmospheric glitchy sound
-- ~chopped_reverb: s "bd sn" $ chop 32 $ scramble 32 # reverb 0.8 0.5 0.3
-- out5: ~chopped_reverb

-- ADVANCED PATTERNS
-- ================================================

-- Example 6: Chop a euclidean pattern
-- ~euclidean_chop: s "bd(5,8)" $ chop 16 $ scramble 16
-- out6: ~euclidean_chop

-- Example 7: Combine different transforms
-- Fast the pattern, then chop and scramble
-- ~fast_chopped: s "bd sn" $ fast 2 $ chop 16 $ scramble 16
-- out7: ~fast_chopped

-- Example 8: Layer multiple chopped versions
-- ~layer1: s "bd sn hh cp" $ chop 8 $ scramble 8 # gain 0.5
-- ~layer2: s "bd sn hh cp" $ chop 16 $ scramble 16 # gain 0.3 # lpf 1000 0.9
-- out8: ~layer1 + ~layer2

-- CREATIVE TECHNIQUES
-- ================================================

-- Example 9: Granular-style chopping (many tiny pieces)
-- ~granular: s "bd" $ chop 64 $ scramble 64
-- out9: ~granular # gain 0.4

-- Example 10: Very fine chopping for glitchy textures
-- ~ultra_fine: s "bd*4" $ chop 64 $ scramble 64 # gain 0.3
-- out10: ~ultra_fine

-- HOW IT WORKS:
-- ================================================
-- chop n    - Slices pattern into n equal parts and stacks them
-- scramble n - Randomly reorders events using Fisher-Yates shuffle
-- shuffle x  - Randomly shifts events in time by ±x amount
--
-- The workflow is: sample → chop (slice) → scramble/shuffle (reorder)
--
-- This is commonly used for:
-- - Glitch effects
-- - Breakbeat mangling
-- - Creating variation from loops
-- - Granular-style sound design
