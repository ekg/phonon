-- Synth Drums vs Sample Drums
-- Compare synthesized drums with sample-based drums

tempo: 0.5

-- ========== Synthesized Drums ==========

-- SuperKick - synthesized kick
~synth_kick: superkick 60 0.5 0.3 0.1

-- SuperSnare - synthesized snare
~synth_snare: supersnare 200 0.8 0.15

-- SuperHat - synthesized hi-hat
~synth_hat: superhat 0.7 0.05

-- ========== Sample-Based Drums ==========

-- Sample playback (requires dirt-samples)
-- ~sample_kick: s("bd")
-- ~sample_snare: s("sn")
-- ~sample_hat: s("hh")

-- ========== Mix ==========

-- Using synthesized drums
out: ~synth_kick * 0.8 + ~synth_snare * 0.6 + ~synth_hat * 0.4

-- Advantages of synth drums:
-- - Fully parametric (adjust pitch, decay, noise, etc.)
-- - No sample dependencies
-- - Deterministic output
-- - Pattern-controlled parameters
--
-- Advantages of sample drums:
-- - Realistic sound
-- - Complex timbres
-- - Authentic character
-- - Minimal CPU
