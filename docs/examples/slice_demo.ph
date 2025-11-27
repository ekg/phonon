-- Slice: Deterministic Chunk Reordering
-- =======================================
-- slice n indices_pattern - divides pattern into n chunks, reorders by indices
-- Different from chop (stacks) and scramble (random) - slice gives you CONTROL

tempo: 0.5

-- BASIC USAGE
-- ============

-- Example 1: Identity - no reordering (0 1 2 3)
-- ~identity: s "bd sn hh cp" $ slice 4 "0 1 2 3"
-- out1: ~identity

-- Example 2: Reverse chunks - play in reverse order (3 2 1 0)
~reversed: s "bd sn hh cp" $ slice 4 "3 2 1 0"
out1: ~reversed # gain 0.7

-- Example 3: Custom reordering - first, third, second, fourth (0 2 1 3)
-- ~reordered: s "bd sn hh cp" $ slice 4 "0 2 1 3"
-- out2: ~reordered

-- CREATIVE TECHNIQUES
-- ====================

-- Example 4: Repeat first chunk 4 times
-- ~repeat: s "bd sn hh cp" $ slice 4 "0 0 0 0"
-- out3: ~repeat

-- Example 5: Skip chunks - only play chunks 0 and 2
-- ~skip: s "bd sn hh cp" $ slice 4 "0 2 0 2"
-- out4: ~skip

-- Example 6: Breakbeat-style reordering with 8 slices
-- ~breakbeat: s "bd*4" $ slice 8 "7 5 3 1 6 4 2 0"
-- out5: ~breakbeat

-- COMBINING WITH EFFECTS
-- =======================

-- Example 7: Slice then filter
-- ~sliced_filtered: s "bd sn hh cp" $ slice 4 "3 1 2 0" # lpf 2000 0.8
-- out6: ~sliced_filtered

-- Example 8: Slice with reverb for glitchy atmosphere
-- ~sliced_reverb: s "bd sn" $ slice 4 "3 1 2 0" # reverb 0.8 0.5 0.3
-- out7: ~sliced_reverb

-- PATTERN-CONTROLLED INDICES
-- ===========================

-- Example 9: Use shorter pattern for alternating chunks
-- This alternates between chunk 0 and chunk 2
-- ~alternating: s "bd sn hh cp" $ slice 4 "0 2"
-- out8: ~alternating

-- Example 10: Complex rhythmic pattern with slice
-- ~complex: s "bd(5,8)" $ slice 8 "7 3 5 1 6 2 4 0"
-- out9: ~complex

-- COMPARISON WITH OTHER TRANSFORMS
-- =================================
-- chop:     Slices and STACKS (plays simultaneously)
-- scramble: Random REORDERING (different each time)
-- shuffle:  Random TIME SHIFTS (timing variations)
-- slice:    DETERMINISTIC reordering (exact control)
--
-- Use slice when you want PRECISE CONTROL over chunk order
-- Use scramble when you want RANDOMNESS
-- Use chop when you want SIMULTANEOUS playback

-- MUSICAL APPLICATIONS
-- =====================
-- 1. Breakbeat rearrangement (reorder drum breaks)
-- 2. Melodic phrase reordering (create variations)
-- 3. Glitch effects (unexpected reordering)
-- 4. Call-and-response patterns (0 2 0 2 - skip chunks)
-- 5. Build-up/breakdown sections (progressive reordering)
