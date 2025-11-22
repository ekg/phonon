-- Simple Select/Multiplex Example
-- Demonstrates the 'select' function for choosing between multiple signals

tempo: 2.0

-- Create four different frequencies
~freq1: sine 220  -- A3
~freq2: sine 330  -- E4
~freq3: sine 440  -- A4
~freq4: sine 550  -- C#5

-- Pattern that cycles through indices 0, 1, 2, 3
-- Each number selects a different signal
~selector: "0 1 2 3"

-- Select outputs the signal corresponding to the index
-- 0 → freq1, 1 → freq2, 2 → freq3, 3 → freq4
~melody: select ~selector ~freq1 ~freq2 ~freq3 ~freq4

-- Output with volume control
out: ~melody * 0.3
