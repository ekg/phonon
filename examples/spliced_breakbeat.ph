-- Spliced breakbeat (feat-splice-stitch)
--
-- `splice n indices` chops a sample into `n` equal slices and plays the slices
-- named by `indices` -- but, unlike `slice`, it time-stretches each slice (by
-- adjusting playback speed) so the slice exactly FILLS its event's slot. This
-- "beat-locks" a break to the grid: rearrange the slices however you like and
-- every hit still lands, and stretches, on time.
--
--   splice 8 "0 1 2 3"   ->  4 events, each a 1/8 slice stretched to a 1/4 slot
--                            (speed = (1/8) / (1/4) = 0.5, so it plays 2x slower)
--
-- Compare with plain `slice`, where each 1/8 slice plays at natural speed and
-- leaves a gap before the next hit. `splice` fills those gaps.

tempo: 0.5

-- The classic move: reorder the slices of a breakbeat, beat-locked to the grid.
~break $ s "breaks125" $ splice 8 "0 1 2 3 4 5 6 7"

-- A rearranged, stuttering variant -- same slices, new order, still on-grid.
~chopped $ s "breaks125" $ splice 8 "0 0 3 2 4 5 7 6"

-- `stitch` interleaves two patterns using a boolean pattern for STRUCTURE
-- (the complement of `sew`): take from the first pattern on `t`, second on `f`.
~hats $ stitch "t f t f t f t f" "hh*2" "oh"

-- Mix: the straight break, a touch of the chopped variant, and hats on top.
out $ ~break * 0.6 + ~chopped * 0.25 + ~hats * 0.3
