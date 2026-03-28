-- Demo: Pattern structure combination (Tidal # operator semantics)
-- The # operator takes structure from the RIGHT side

bpm: 120

-- Test 1: s "bd" # note "c4 e4 g4"
-- Should trigger 3 times (structure from note pattern)
out $ s "bd" # note "c4 e4 g4"

-- Test 2: s "bd sn" # note "c4 e4 g4 d4"
-- Should trigger 4 times (structure from note, not from sample)
-- out $ s "bd sn" # note "c4 e4 g4 d4"

-- Test 3: Multiple modifiers
-- out $ s "bd" # note "c4 e4" # gain "0.5 1.0 0.8"
-- Should have 3 triggers (last modifier wins)
