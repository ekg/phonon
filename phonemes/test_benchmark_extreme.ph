tempo: 8.0
-- Extreme benchmark: ~200 simultaneous voices
-- 50 events × 4 notes per chord = 200 voices
-- Tests system limits with parallel processing
out: s "bd*50" # note "c4'dom7" # gain 0.08
