tempo: 4.0
-- Benchmark: Heavy chord load
-- 25 events × 4 notes per chord = 100 simultaneous voices per cycle
-- Tests parallel processing with realistic musical content
out: s "bd*25" # note "c4'dom7" # gain 0.1
