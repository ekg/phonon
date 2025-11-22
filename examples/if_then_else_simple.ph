-- Simple Conditional Routing Example
-- Demonstrates the 'if' function for dynamic signal routing

tempo: 1.0

-- Create a control signal (LFO that oscillates between -1 and 1)
~condition: sine 0.5

-- Create two different signals
~high_note: sine 880  -- A5
~low_note: sine 220   -- A3

-- Use conditional routing:
-- When condition > 0.5, play high_note
-- When condition <= 0.5, play low_note
~output: if ~condition ~high_note ~low_note

-- Output with volume control
out: ~output * 0.3
