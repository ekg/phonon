-- Filter Sweep with Delay Feedback Example
-- LFO-modulated filter combined with delay feedback
-- Creates evolving, rhythmic textures

tempo: 2.0

-- Source signal
~source: saw 110 * 0.5

-- LFO sweeps filter cutoff (2000Hz range, centered at 1000Hz)
~lfo: sine 0.25 * 2000 + 1000

-- Filtered signal with delay feedback
~filtered: ~source # lpf ~lfo 0.8 # delay 0.25 0.6

-- Output
out: ~filtered * 0.7

-- Try adjusting:
-- - LFO rate (0.25 Hz) for faster/slower sweeps
-- - LFO depth (2000) for wider/narrower sweeps
-- - delay feedback (0.6) for longer/shorter echoes
-- - Q factor (0.8) for more/less resonance
