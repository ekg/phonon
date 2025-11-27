-- White Noise Generator Demo
-- Generates random samples with equal energy across all frequencies
--
-- NOTE: This example documents the NoiseNode API for the new
-- block-based architecture (src/nodes/noise.rs). Full integration
-- into the Phonon DSL is pending.
--
-- The NoiseNode generates white noise scaled by an amplitude control signal.
-- Unlike oscillators which generate periodic waveforms, noise produces
-- random values useful for percussion, effects, and synthesis.

tempo: 0.5

-- Example 1: Basic white noise at constant amplitude
-- ~noise: noise 0.3
-- out1: ~noise

-- Example 2: Amplitude-modulated noise (tremolo effect)
-- ~lfo: sine 4  -- 4 Hz modulation
-- ~amplitude: ~lfo * 0.2 + 0.3  -- Varies between 0.1 and 0.5
-- ~tremolo_noise: noise ~amplitude
-- out2: ~tremolo_noise

-- Example 3: Filtered noise for hi-hat sounds
-- ~raw_noise: noise 0.8
-- ~hihat: ~raw_noise # hpf 8000 0.3 # lpf 12000 0.5
-- out3: ~hihat * 0.5

-- Example 4: Noise burst for snare drum body
-- ~trigger: s "~ x ~ x"  -- On beats 2 and 4
-- ~envelope: ~trigger # adsr 0.001 0.05 0.0 0.01  -- Short decay
-- ~snare_body: noise 1.0 * ~envelope
-- ~snare: ~snare_body # hpf 200 0.5
-- out4: ~snare * 0.6

-- Example 5: Pattern-controlled noise density
-- ~density: "0.1 0.3 0.5 0.8"  -- Amplitude pattern
-- ~patterned_noise: noise ~density
-- out5: ~patterned_noise

-- Technical Details:
--
-- The NoiseNode generates white noise using a deterministic random
-- number generator (StdRng). Key characteristics:
--
-- 1. Equal spectral energy across all frequencies
-- 2. Zero mean (averages to 0.0 over time)
-- 3. Gaussian-like amplitude distribution
-- 4. Deterministic with seed (reproducible for testing)
--
-- Parameters:
-- - amplitude: Control signal (0.0-1.0 typical, can exceed)
--   Output range: [-amplitude, +amplitude]
--
-- Common Uses:
-- - Percussion synthesis (hi-hats, snares, cymbals)
-- - Texture and atmosphere
-- - Sound effects (wind, rain, static)
-- - Modulation source (sample & hold)
-- - Dithering in audio processing

-- Future Extensions:
-- - Pink noise (1/f spectrum, more natural)
-- - Brown noise (1/f² spectrum, deeper)
-- - Colored noise (arbitrary spectral shaping)
-- - Burst mode (triggered noise envelopes)
