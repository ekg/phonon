-- Fixed Demo - Should work with current parser
tempo 2.0

-- Bass with pattern filter
bass = saw "55 41 44 55" # lpf("200 500 1000 2000", 3)

-- Simple lead
lead = square 220 # lpf("500 1000 2000 4000", 5)

-- Drums
drums = noise # lpf("100 100 100 5000", 10)

-- Mix - multiplication must be last
out bass * 0.2

-- To hear other parts, replace the out line with:
-- out lead * 0.2
-- out drums * 0.3