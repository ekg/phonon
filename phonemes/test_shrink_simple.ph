tempo: 0.5
-- Ultra-simple shrink test:
-- Cycle 0-1: Trigger 50 voices (will grow from 16)
-- Cycle 2-9: Complete silence for 4 seconds (plenty of time for cleanup)
-- Cycle 10-11: Trigger 4 voices (should see shrunk pool)
out: s "hh*50 ~ ~ ~ ~ ~ ~ ~ ~ ~ hh*4 hh*4"
